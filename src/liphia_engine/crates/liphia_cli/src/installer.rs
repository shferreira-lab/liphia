// liphia_cli/src/installer.rs
//
// Package manager: `liphia init | install | update | remove | list`.
//
// Files involved (see liphia_manifest for their formats):
//   liphia.toml    a project's or member's direct dependencies; a workspace
//                  root also lists members and [workspace.dependencies]
//   liphia.lock    exact versions installed, transitive ones included
//   index.toml     published versions of every package (from the registry)
//   package.toml   one package's files, dependencies and engine requirement
//
// Every command works on the workspace that contains the current folder
// (a single project is a workspace of one). liphia.lock and liphia_modules/
// live only at the workspace root, and all members are resolved together:
// one version of each package for the whole workspace, since natives are
// global inside the VM and two versions of one package could not coexist.
// `install <pkg>` and `remove <pkg>` edit the liphia.toml of the member the
// current folder belongs to.
//
// Where packages come from (the "registry"):
//   - default: the GitHub repo. Package files are read at the package's
//     release tag (<name>-v<version>), native libraries are the assets of
//     that release, and index.toml is read from the main branch.
//   - LIPHIA_REGISTRY=<dir>: a local folder laid out like src/packages
//     (index.toml + one folder per package, libs under <name>/lib/). Used
//     to test packages before publishing; versions are whatever the folder
//     holds.
//
// Downloads are cached per machine in <LIPHIA_HOME>/cache/<name>/<version>/
// (LIPHIA_HOME defaults to ~/.liphia) and copied into <root>/liphia_modules/.
// Conflicting requirements are an error that names who asked for what.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use liphia_manifest::{
    highest_matching, lib_asset_name, lib_file_name, parse_req, Dependency, Index, LockedPackage,
    Lockfile, PackageManifest, ProjectManifest, RunPlan, Version, Workspace, MODULES_DIR,
    PACKAGE_FILE, PROJECT_FILE,
};
use liphia_virtual_machine::vm::VM;

const REPO: &str = "shferreira-lab/liphia";
const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

// ── Registry ──────────────────────────────────────────────────────────────────

enum Registry {
    GitHub,
    Local(PathBuf),
}

impl Registry {
    fn from_env() -> Registry {
        match std::env::var("LIPHIA_REGISTRY") {
            Ok(dir) if !dir.trim().is_empty() => Registry::Local(PathBuf::from(dir)),
            _ => Registry::GitHub,
        }
    }

    fn describe(&self) -> String {
        match self {
            Registry::GitHub => format!("github.com/{}", REPO),
            Registry::Local(dir) => format!("local registry {}", dir.display()),
        }
    }

    fn index(&self) -> Result<Index, String> {
        let text = match self {
            Registry::GitHub => http_get_text(&format!(
                "https://raw.githubusercontent.com/{}/main/src/packages/index.toml",
                REPO
            ))?,
            Registry::Local(dir) => read_text(&dir.join("index.toml"))?,
        };
        Index::parse(&text)
    }

    // A file of `name` at `version`, as bytes. `rel` is relative to the
    // package folder (e.g. "num.lph", "composed/describe.lph").
    fn package_file(&self, name: &str, version: &Version, rel: &str) -> Result<Vec<u8>, String> {
        match self {
            Registry::GitHub => http_get_bytes(&format!(
                "https://raw.githubusercontent.com/{}/{}-v{}/src/packages/{}/{}",
                REPO, name, version, name, rel
            )),
            Registry::Local(dir) => read_bytes(&dir.join(name).join(rel)),
        }
    }

    // The native library of `name` at `version` for this platform.
    fn native_lib(&self, name: &str, version: &Version, lib: &str) -> Result<Vec<u8>, String> {
        match self {
            Registry::GitHub => http_get_bytes(&format!(
                "https://github.com/{}/releases/download/{}-v{}/{}",
                REPO,
                name,
                version,
                lib_asset_name(lib)
            )),
            Registry::Local(dir) => read_bytes(&dir.join(name).join("lib").join(lib_file_name(lib))),
        }
    }

    fn is_local(&self) -> bool {
        matches!(self, Registry::Local(_))
    }
}

// ── Commands ──────────────────────────────────────────────────────────────────

pub fn init_project(workspace: bool) {
    let path = Path::new(PROJECT_FILE);
    if path.exists() {
        println!("[liphia] {} already exists.", PROJECT_FILE);
        return;
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    let manifest = if workspace {
        ProjectManifest::new_workspace()
    } else {
        let name = cwd
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "my_project".to_string());
        ProjectManifest::new(&name)
    };
    if let Err(e) = manifest.save(path) {
        fail(&format!("failed to create {}: {}", PROJECT_FILE, e));
    }
    println!("[liphia] created {}", PROJECT_FILE);
    if workspace {
        println!("[liphia] list member folders in [workspace] members, e.g. members = [\"app\", \"libs/*\"].");
        return;
    }
    // A new project under a workspace root still has to be listed there.
    if let Some(root) = cwd.parent().and_then(enclosing_workspace_root) {
        println!(
            "[liphia] this folder is inside the workspace at {}; add it to [workspace] members there.",
            root.display()
        );
    }
    println!("[liphia] run 'liphia install <package>' to add dependencies.");
}

pub fn list_packages() {
    let registry = Registry::from_env();
    let index = registry.index().unwrap_or_else(|e| fail(&e));
    let lock_path = std::env::current_dir()
        .ok()
        .and_then(|cwd| Workspace::discover(&cwd).ok().flatten())
        .map(|ws| ws.lock_path())
        .unwrap_or_else(|| PathBuf::from(liphia_manifest::LOCK_FILE));
    let lock = Lockfile::load(&lock_path).ok().flatten();
    println!("[liphia] packages in {}:", registry.describe());
    for (name, entry) in &index.packages {
        let latest = index.latest(name).map(|v| v.to_string()).unwrap_or_default();
        let installed = lock
            .as_ref()
            .and_then(|l| l.get(name))
            .map(|p| format!("installed {}", p.version))
            .unwrap_or_default();
        println!("  {:<8} {:<8} {:<16} {}", name, latest, installed, entry.description);
    }
}

// `liphia members`: the workspace layout, for checking what discovery found.
pub fn list_members() {
    let ws = load_workspace();
    let kind = if ws.is_workspace { "workspace" } else { "project" };
    println!("[liphia] {} at {}", kind, ws.root.display());
    for m in &ws.members {
        let deps: Vec<String> = m
            .deps
            .iter()
            .map(|(name, dep)| match dep {
                liphia_manifest::MemberDep::Registry { req, .. } => format!("{} {}", name, req),
                liphia_manifest::MemberDep::Local(_) => format!("{} (member)", name),
            })
            .collect();
        println!(
            "  {:<12} {:<8} {:<20} {}",
            m.name,
            m.version,
            m.rel,
            if deps.is_empty() { "-".to_string() } else { deps.join(", ") }
        );
    }
}

// `liphia install` with no names installs the whole workspace as locked;
// `--frozen` additionally refuses to change the lock (for CI).
// With names, each spec is added to the current member's liphia.toml first:
//   num            latest version, saved as "^<latest>"
//   num@1.2.9      exactly 1.2.9 (npm semantics)
//   num@^1.2       any compatible version
//   num@latest     newest version, even across a major bump
//   db:sqlite      the db package plus its sqlite subpackage
pub fn install(specs: &[&str], frozen: bool) {
    let registry = Registry::from_env();
    let mut ws = load_workspace();
    let mut unlocked: HashSet<String> = HashSet::new();
    let mut edited: Option<usize> = None;

    if !specs.is_empty() {
        if frozen {
            fail("--frozen installs exactly liphia.lock; it cannot add packages");
        }
        let target = current_member(&ws, "add packages to");
        let mut manifest = ws.members[target].manifest.clone();
        let index = registry.index().unwrap_or_else(|e| fail(&e));
        for spec in specs {
            let (name, req, subpackages) = parse_spec(spec, &index);
            if ws.member_by_name(&name).is_some() {
                fail(&format!(
                    "'{}' is a workspace member; depend on it with {{ path = \"...\" }} in {}",
                    name,
                    ws.members[target].label()
                ));
            }
            let mut merged: Vec<String> = vec![];
            match manifest.dependencies.get(&name) {
                Some(dep) if !dep.is_registry() => fail(&format!(
                    "'{}' in {} is a path or workspace dependency; change it there by hand \
                     (shared requirements live in [workspace.dependencies] of the root {})",
                    name,
                    ws.members[target].label(),
                    PROJECT_FILE
                )),
                Some(dep) => merged.extend(dep.subpackages().iter().cloned()),
                None => {}
            }
            merged.extend(subpackages);
            manifest.dependencies.insert(name.clone(), Dependency::new(req, merged));
            // A package named on the command line is re-resolved, so
            // `install num@latest` or a changed requirement takes effect.
            unlocked.insert(name);
        }
        ws.set_member_manifest(target, manifest).unwrap_or_else(|e| fail(&e));
        edited = Some(target);
    }

    // The member's liphia.toml is only rewritten once the new set installed
    // cleanly, so a failed install leaves the project exactly as it was.
    run(&registry, &ws, Unlock::Some(unlocked), frozen).unwrap_or_else(|e| fail(&e));
    if let Some(i) = edited {
        save_member(&ws, i);
    }
}

// `liphia update` re-resolves every package within its requirement;
// `liphia update num` only num (and whatever num's new version needs).
pub fn update(names: &[&str]) {
    let registry = Registry::from_env();
    let ws = load_workspace();
    let unlock = if names.is_empty() {
        Unlock::All
    } else {
        Unlock::Some(names.iter().map(|s| s.to_string()).collect())
    };
    run(&registry, &ws, unlock, false).unwrap_or_else(|e| fail(&e));
}

pub fn remove(names: &[&str]) {
    if names.is_empty() {
        fail("usage: liphia remove <package> [package ...]");
    }
    let registry = Registry::from_env();
    let mut ws = load_workspace();
    let target = current_member(&ws, "remove packages from");
    let mut manifest = ws.members[target].manifest.clone();
    for name in names {
        if manifest.dependencies.remove(*name).is_none() {
            eprintln!(
                "[liphia] '{}' is not a dependency in {}",
                name,
                ws.members[target].label()
            );
        }
    }
    ws.set_member_manifest(target, manifest).unwrap_or_else(|e| fail(&e));
    run(&registry, &ws, Unlock::Some(HashSet::new()), false).unwrap_or_else(|e| fail(&e));
    save_member(&ws, target);
}

// ── Resolution ────────────────────────────────────────────────────────────────

enum Unlock {
    All,
    Some(HashSet<String>),
}

impl Unlock {
    fn contains(&self, name: &str) -> bool {
        match self {
            Unlock::All => true,
            Unlock::Some(set) => set.contains(name),
        }
    }
}

struct Resolved {
    version: Version,
    manifest: PackageManifest,
    subpackages: Vec<String>,
    required_by: Vec<String>,
}

struct Wanted {
    name: String,
    req: String,
    subpackages: Vec<String>,
    by: String,
}

// Resolves, installs and writes liphia.lock. Resolution happens before
// anything is touched, so an unsatisfiable request changes nothing.
fn run(registry: &Registry, ws: &Workspace, unlock: Unlock, frozen: bool) -> Result<(), String> {
    let lock_path = ws.lock_path();
    let modules_dir = ws.modules_dir();
    let lock = Lockfile::load(&lock_path)?;
    if frozen && lock.is_none() {
        return Err("--frozen requires liphia.lock".to_string());
    }
    let requirements = ws.registry_requirements();
    if requirements.is_empty() {
        println!("[liphia] no registry dependencies in {}.", describe_workspace(ws));
    }

    let resolved = resolve(registry, &requirements, lock.as_ref(), &unlock, frozen)?;

    let mut failed = 0;
    for (name, pkg) in &resolved {
        match install_one(registry, &modules_dir, name, pkg) {
            Ok(n) => println!("  {} {}  ({} file(s))", name, pkg.version, n),
            Err(e) => {
                eprintln!("  {} {}  FAILED\n    {}", name, pkg.version, e);
                failed += 1;
            }
        }
    }
    prune(&modules_dir, &resolved);

    if failed > 0 {
        return Err(format!("{} package(s) failed; liphia.lock was not updated", failed));
    }
    let new_lock = Lockfile {
        lock_version: 1,
        engine: ENGINE_VERSION.to_string(),
        packages: resolved
            .iter()
            .map(|(name, pkg)| LockedPackage {
                name: name.clone(),
                version: pkg.version.to_string(),
                native: pkg.manifest.is_native(),
                subpackages: pkg.subpackages.clone(),
            })
            .collect(),
    };
    if !frozen {
        new_lock.save(&lock_path)?;
    }
    println!(
        "[liphia] {} package(s) ready in {} ({}).",
        resolved.len(),
        describe_workspace(ws),
        registry.describe()
    );
    Ok(())
}

// Breadth-first over the dependency graph. The lock pins every package it
// knows unless that package is unlocked; everything else takes the highest
// indexed version its requirement allows.
fn resolve(
    registry: &Registry,
    requirements: &[liphia_manifest::Requirement],
    lock: Option<&Lockfile>,
    unlock: &Unlock,
    frozen: bool,
) -> Result<BTreeMap<String, Resolved>, String> {
    let index = registry.index()?;
    let engine = Version::parse(ENGINE_VERSION).expect("engine version is valid semver");
    let mut chosen: BTreeMap<String, Resolved> = BTreeMap::new();
    // Popped from the end, so reversed to handle members in listing order.
    let mut queue: Vec<Wanted> = requirements
        .iter()
        .rev()
        .map(|r| Wanted {
            name: r.name.clone(),
            req: r.req.clone(),
            subpackages: r.subpackages.clone(),
            by: r.by.clone(),
        })
        .collect();

    while let Some(want) = queue.pop() {
        let req = parse_req(&want.req).map_err(|e| format!("{} (required by {})", e, want.by))?;

        if let Some(existing) = chosen.get_mut(&want.name) {
            if !req.matches(&existing.version) {
                return Err(format!(
                    "version conflict for '{}': {} requires {}, but {} is already selected (required by {})",
                    want.name,
                    want.by,
                    want.req,
                    existing.version,
                    existing.required_by.join(", ")
                ));
            }
            existing.required_by.push(want.by);
            for s in want.subpackages {
                if !existing.subpackages.contains(&s) {
                    existing.subpackages.push(s);
                }
            }
            continue;
        }

        let locked = lock
            .and_then(|l| l.get(&want.name))
            .and_then(|p| Version::parse(&p.version).ok())
            .filter(|v| req.matches(v));
        let version = match locked {
            Some(v) if frozen || !unlock.contains(&want.name) => v,
            _ if frozen => {
                return Err(format!(
                    "--frozen: liphia.lock has no version of '{}' matching {}",
                    want.name, want.req
                ))
            }
            _ => {
                let entry = index.packages.get(&want.name).ok_or_else(|| {
                    format!("unknown package '{}' (required by {})", want.name, want.by)
                })?;
                highest_matching(&entry.versions, &req).ok_or_else(|| {
                    format!(
                        "no version of '{}' matches {} (required by {}); published: {}",
                        want.name,
                        want.req,
                        want.by,
                        entry.versions.join(", ")
                    )
                })?
            }
        };

        let text = registry.package_file(&want.name, &version, PACKAGE_FILE)?;
        let manifest = PackageManifest::parse(&String::from_utf8_lossy(&text))
            .map_err(|e| format!("{} {}: {}", want.name, version, e))?;
        let engine_req = manifest.engine_req()?;
        if !engine_req.matches(&engine) {
            return Err(format!(
                "{} {} requires engine {}, but this is engine {}; pick a version built for this engine",
                want.name, version, manifest.package.engine, engine
            ));
        }

        for (dep, dep_req) in &manifest.dependencies {
            queue.push(Wanted {
                name: dep.clone(),
                req: dep_req.clone(),
                subpackages: vec![],
                by: format!("{} {}", want.name, version),
            });
        }
        chosen.insert(
            want.name.clone(),
            Resolved {
                version,
                manifest,
                subpackages: want.subpackages,
                required_by: vec![want.by],
            },
        );
    }
    Ok(chosen)
}

// ── Installing ────────────────────────────────────────────────────────────────

// Fills the cache for this exact version (skipped for a local registry,
// whose content can change without a version bump), then replaces
// <root>/liphia_modules/<name> with a copy of it.
fn install_one(registry: &Registry, modules_dir: &Path, name: &str, pkg: &Resolved) -> Result<usize, String> {
    let mut files: Vec<String> = vec![PACKAGE_FILE.to_string()];
    files.extend(pkg.manifest.package.files.iter().cloned());
    for sub in &pkg.subpackages {
        let sub_files = pkg
            .manifest
            .subpackages
            .get(sub)
            .ok_or_else(|| format!("{} has no subpackage '{}'", name, sub))?;
        files.extend(sub_files.files.iter().cloned());
    }

    let source = if registry.is_local() {
        let dir = std::env::temp_dir().join(format!("liphia-install-{}", name));
        let _ = fs::remove_dir_all(&dir);
        fill(registry, name, pkg, &files, &dir)?;
        dir
    } else {
        let dir = cache_dir()
            .join(name)
            .join(pkg.version.to_string())
            .join(liphia_manifest::platform_key());
        let complete = dir.join(".complete");
        if !complete.exists() || !files.iter().all(|f| dir.join(f).exists()) {
            let _ = fs::remove_dir_all(&dir);
            fill(registry, name, pkg, &files, &dir)?;
            write(&complete, b"")?;
        }
        dir
    };

    let dest = modules_dir.join(name);
    let _ = fs::remove_dir_all(&dest);
    copy_dir(&source, &dest)?;
    let _ = fs::remove_file(dest.join(".complete"));
    Ok(files.len() + usize::from(pkg.manifest.is_native()))
}

fn fill(
    registry: &Registry,
    name: &str,
    pkg: &Resolved,
    files: &[String],
    dir: &Path,
) -> Result<(), String> {
    for rel in files {
        let bytes = registry.package_file(name, &pkg.version, rel)?;
        write(&dir.join(rel), &bytes)?;
    }
    if let Some(native) = &pkg.manifest.native {
        let bytes = registry.native_lib(name, &pkg.version, &native.lib)?;
        write(&dir.join("lib").join(lib_file_name(&native.lib)), &bytes)?;
    }
    Ok(())
}

// Removes installed packages the resolution no longer includes. Only
// folders holding a package.toml are touched.
fn prune(modules_dir: &Path, resolved: &BTreeMap<String, Resolved>) {
    let Ok(entries) = fs::read_dir(modules_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.join(PACKAGE_FILE).exists() && !resolved.contains_key(&name) {
            if fs::remove_dir_all(&path).is_ok() {
                println!("  {} removed", name);
            }
        }
    }
}

// ── Loading installed native packages ─────────────────────────────────────────
//
// Native packages ship prebuilt libraries (see liphia_virtual_machine::
// external). Inside a project only the packages in the running member's
// dependency closure are loaded (RunPlan::native_dirs); outside any project
// every package in ./liphia_modules/ is, as before workspaces. Failures are
// warnings, not fatal: a program that never calls a package's natives
// should not be blocked by it failing to load.
pub fn load_native_packages(vm: &mut VM, plan: Option<&RunPlan>) {
    match plan {
        Some(plan) => {
            for dir in &plan.native_dirs {
                if let Err(e) = vm.load_external_module(dir) {
                    let name = dir.file_name().unwrap_or_default().to_string_lossy();
                    eprintln!("[liphia] warning: failed to load package '{}': {}", name, e.message);
                }
            }
        }
        None => {
            for (name, err) in vm.load_installed_external_modules(MODULES_DIR) {
                eprintln!(
                    "[liphia] warning: failed to load package '{}': {}",
                    name, err.message
                );
            }
        }
    }
}

// Plan for the REPL: the member the current folder belongs to, or the whole
// workspace from its root; None outside any project.
pub fn plan_for_current_dir() -> Option<RunPlan> {
    let cwd = std::env::current_dir().ok()?;
    match Workspace::discover(&cwd) {
        Ok(Some(ws)) => {
            let member = ws.member_at(&cwd);
            Some(ws.run_plan(member))
        }
        Ok(None) => None,
        Err(e) => {
            eprintln!("[liphia] warning: {}", e);
            None
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

// "name[:subpackage][@requirement]" -> (name, requirement, subpackages)
fn parse_spec(spec: &str, index: &Index) -> (String, String, Vec<String>) {
    let (target, req) = match spec.split_once('@') {
        Some((t, r)) => (t, Some(r)),
        None => (spec, None),
    };
    let (name, sub) = match target.split_once(':') {
        Some((n, s)) => (n.to_string(), vec![s.to_string()]),
        None => (target.to_string(), vec![]),
    };
    let latest = || {
        index
            .latest(&name)
            .unwrap_or_else(|| fail(&format!("unknown package '{}'", name)))
    };
    match req {
        None | Some("latest") => (name.clone(), format!("^{}", latest()), sub),
        Some(r) => {
            if let Err(e) = parse_req(r) {
                fail(&e);
            }
            (name, r.to_string(), sub)
        }
    }
}

fn load_workspace() -> Workspace {
    let cwd = std::env::current_dir().unwrap_or_else(|e| fail(&e.to_string()));
    Workspace::discover(&cwd)
        .unwrap_or_else(|e| fail(&e))
        .unwrap_or_else(|| fail("no liphia.toml here or in any parent folder; run 'liphia init' first"))
}

// The member whose liphia.toml a command edits: the one the current folder
// belongs to. A virtual workspace root (no [package]) has none.
fn current_member(ws: &Workspace, action: &str) -> usize {
    let cwd = std::env::current_dir().unwrap_or_default();
    ws.member_at(&cwd).unwrap_or_else(|| {
        let names: Vec<String> = ws.members.iter().map(|m| m.rel.clone()).collect();
        fail(&format!(
            "the workspace root has no [package] to {}; cd into a member ({}) \
             or edit [workspace.dependencies] in the root {}",
            action,
            names.join(", "),
            PROJECT_FILE
        ))
    })
}

fn save_member(ws: &Workspace, index: usize) {
    let member = &ws.members[index];
    member
        .manifest
        .save(&member.manifest_path())
        .unwrap_or_else(|e| fail(&e));
}

fn describe_workspace(ws: &Workspace) -> String {
    if ws.is_workspace {
        format!("workspace {} ({} member(s))", ws.root.display(), ws.members.len())
    } else {
        ws.root.display().to_string()
    }
}

// Nearest ancestor of `dir` (inclusive) whose liphia.toml has [workspace].
fn enclosing_workspace_root(dir: &Path) -> Option<PathBuf> {
    dir.ancestors()
        .find(|d| {
            ProjectManifest::load(&d.join(PROJECT_FILE))
                .map(|m| m.workspace.is_some())
                .unwrap_or(false)
        })
        .map(|d| d.to_path_buf())
}

fn liphia_home() -> PathBuf {
    if let Ok(dir) = std::env::var("LIPHIA_HOME") {
        return PathBuf::from(dir);
    }
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".liphia")
}

fn cache_dir() -> PathBuf {
    liphia_home().join("cache")
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| format!("{}: {}", to.display(), e))?;
    let entries = fs::read_dir(from).map_err(|e| format!("{}: {}", from.display(), e))?;
    for entry in entries.flatten() {
        let src = entry.path();
        let dst = to.join(entry.file_name());
        if src.is_dir() {
            copy_dir(&src, &dst)?;
        } else {
            fs::copy(&src, &dst).map_err(|e| format!("{}: {}", dst.display(), e))?;
        }
    }
    Ok(())
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {}", parent.display(), e))?;
    }
    fs::write(path, bytes).map_err(|e| format!("{}: {}", path.display(), e))
}

fn read_text(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("{}: {}", path.display(), e))
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("{}: {}", path.display(), e))
}

fn fail(message: &str) -> ! {
    eprintln!("[liphia] error: {}", message);
    process::exit(1);
}

// ── HTTP (curl, wget fallback) ────────────────────────────────────────────────
//
// curl ships with Windows 10+, macOS and most Linux distributions. `-f`
// turns HTTP errors (404 for a missing tag or asset) into a failure.

fn http_get_text(url: &str) -> Result<String, String> {
    http_get_bytes(url).map(|b| String::from_utf8_lossy(&b).to_string())
}

fn http_get_bytes(url: &str) -> Result<Vec<u8>, String> {
    match process::Command::new("curl").args(["-fsSL", url]).output() {
        Ok(out) if out.status.success() => Ok(out.stdout),
        Ok(out) => Err(format!(
            "download failed: {} ({})",
            url,
            String::from_utf8_lossy(&out.stderr).trim()
        )),
        Err(_) => match process::Command::new("wget").args(["-qO-", url]).output() {
            Ok(out) if out.status.success() => Ok(out.stdout),
            Ok(_) => Err(format!("download failed: {}", url)),
            Err(_) => Err("neither curl nor wget is available".to_string()),
        },
    }
}
