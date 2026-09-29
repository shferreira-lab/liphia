// liphia_manifest/src/lib.rs
//
// Data model of the Liphia toolchain files:
//
//   liphia.toml     a project or workspace member: its name, entry and the
//                   packages it depends on; a workspace root also lists its
//                   members and shared dependency requirements
//   liphia.lock     the exact versions installed, including transitive ones
//   package.toml    a package: version, engine compatibility, files, native lib
//   index.toml      every published version of every official package
//
// Version requirements follow npm: "1.2.9" is exact, "^1.2.9" accepts
// compatible versions (same major), "~1.2.9" accepts patches (same minor),
// "*" or "latest" accepts anything. The semver crate treats a bare "1.2.9"
// as caret, so `parse_req` adds the "=" that the npm meaning requires.
//
// Workspaces follow Cargo: a root liphia.toml with [workspace] lists member
// folders; the root holds the only liphia.lock and liphia_modules/, so every
// member sees the same version of every package. Members depend on each
// other by path and import each other by name, like registry packages.
//
// This crate reads and writes these files and walks the folders a workspace
// names; networking and installation live in liphia_cli.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

pub use semver::{Version, VersionReq};

// ── Version requirements ──────────────────────────────────────────────────────

pub fn parse_req(text: &str) -> Result<VersionReq, String> {
    let text = text.trim();
    let normalized = if text.is_empty() || text == "latest" {
        "*".to_string()
    } else if text.starts_with(|c: char| c.is_ascii_digit()) {
        format!("={}", text)
    } else {
        text.to_string()
    };
    VersionReq::parse(&normalized).map_err(|e| format!("invalid version requirement '{}': {}", text, e))
}

pub fn parse_version(text: &str) -> Result<Version, String> {
    Version::parse(text.trim()).map_err(|e| format!("invalid version '{}': {}", text, e))
}

// Highest version in `available` that satisfies `req`. Unparseable entries
// are skipped rather than failing the whole resolution.
pub fn highest_matching(available: &[String], req: &VersionReq) -> Option<Version> {
    available
        .iter()
        .filter_map(|v| Version::parse(v).ok())
        .filter(|v| req.matches(v))
        .max()
}

// ── liphia.toml ───────────────────────────────────────────────────────────────

pub const PROJECT_FILE: &str = "liphia.toml";
pub const MODULES_DIR: &str = "liphia_modules";

// A liphia.toml has [package] (a project or member), [workspace] (a root),
// or both (a root that is also a member of its own workspace).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectManifest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<ProjectInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceInfo>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub dependencies: BTreeMap<String, Dependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    #[serde(default = "default_project_version")]
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
}

// `members` are folders relative to the root; "libs/*" takes every direct
// subfolder of libs/ that has a liphia.toml. `dependencies` is a catalog:
// members opt in with `name = { workspace = true }`, and an entry no member
// uses is not installed (as in Cargo).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    #[serde(default)]
    pub members: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub dependencies: BTreeMap<String, Dependency>,
}

fn default_project_version() -> String {
    "0.1.0".to_string()
}

// The forms a dependency can take in liphia.toml:
//
//   num    = "^1.0.0"                                   registry, short form
//   db     = { version = "^2.0.0", subpackages = ["sqlite"] }
//   models = { path = "../models" }                     another workspace member
//   db     = { workspace = true, subpackages = ["postgres"] }
//                                                       from [workspace.dependencies]
//
// Unknown keys are rejected, so a typo cannot silently turn one form into
// another (e.g. `{ version = "1.0.0", path = "..." }`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Dependency {
    Req(String),
    Registry {
        version: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        subpackages: Vec<String>,
    },
    Path {
        path: String,
    },
    Workspace {
        workspace: bool,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        subpackages: Vec<String>,
    },
}

impl Dependency {
    // The version requirement of a registry dependency; None for the path
    // and workspace forms, whose requirement lives elsewhere.
    pub fn req(&self) -> Option<&str> {
        match self {
            Dependency::Req(r) => Some(r),
            Dependency::Registry { version, .. } => Some(version),
            Dependency::Path { .. } | Dependency::Workspace { .. } => None,
        }
    }

    pub fn subpackages(&self) -> &[String] {
        match self {
            Dependency::Registry { subpackages, .. } | Dependency::Workspace { subpackages, .. } => {
                subpackages
            }
            Dependency::Req(_) | Dependency::Path { .. } => &[],
        }
    }

    pub fn is_registry(&self) -> bool {
        matches!(self, Dependency::Req(_) | Dependency::Registry { .. })
    }

    // A registry dependency; collapses to the short form when no subpackage
    // is selected.
    pub fn new(req: String, mut subpackages: Vec<String>) -> Self {
        subpackages.sort();
        subpackages.dedup();
        if subpackages.is_empty() {
            Dependency::Req(req)
        } else {
            Dependency::Registry { version: req, subpackages }
        }
    }
}

impl ProjectManifest {
    pub fn new(name: &str) -> Self {
        ProjectManifest {
            package: Some(ProjectInfo {
                name: name.to_string(),
                version: default_project_version(),
                entry: None,
            }),
            workspace: None,
            dependencies: BTreeMap::new(),
        }
    }

    pub fn new_workspace() -> Self {
        ProjectManifest {
            package: None,
            workspace: Some(WorkspaceInfo::default()),
            dependencies: BTreeMap::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {}", path.display(), e))?;
        let manifest: ProjectManifest =
            toml::from_str(&text).map_err(|e| format!("{}: {}", path.display(), e))?;
        manifest.validate().map_err(|e| format!("{}: {}", path.display(), e))?;
        Ok(manifest)
    }

    // Rewrites the whole file; comments in liphia.toml are not preserved.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let text = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, text).map_err(|e| format!("{}: {}", path.display(), e))
    }

    pub fn name(&self) -> Option<&str> {
        self.package.as_ref().map(|p| p.name.as_str())
    }

    // Rules serde cannot express on its own.
    fn validate(&self) -> Result<(), String> {
        if self.package.is_none() && self.workspace.is_none() {
            return Err("needs a [package] or a [workspace] section".to_string());
        }
        if self.package.is_none() && !self.dependencies.is_empty() {
            return Err(
                "a workspace root without [package] cannot have [dependencies]; \
                 put shared requirements in [workspace.dependencies]"
                    .to_string(),
            );
        }
        for (name, dep) in &self.dependencies {
            if let Dependency::Workspace { workspace: false, .. } = dep {
                return Err(format!(
                    "dependency '{}': `workspace = false` is not a form; give a version or a path",
                    name
                ));
            }
        }
        if let Some(ws) = &self.workspace {
            for (name, dep) in &ws.dependencies {
                if let Dependency::Workspace { .. } = dep {
                    return Err(format!(
                        "[workspace.dependencies] '{}' cannot itself use `workspace = true`",
                        name
                    ));
                }
            }
        }
        Ok(())
    }
}

// ── Workspaces ────────────────────────────────────────────────────────────────

// A dependency after `workspace = true` and paths are resolved.
#[derive(Debug, Clone, PartialEq)]
pub enum MemberDep {
    Registry { req: String, subpackages: Vec<String> },
    // Index into Workspace::members.
    Local(usize),
}

#[derive(Debug, Clone)]
pub struct Member {
    pub name: String,
    pub version: String,
    pub dir: PathBuf,
    // Folder relative to the workspace root with '/' separators; "." for a
    // root that is also a member.
    pub rel: String,
    pub entry: Option<PathBuf>,
    pub manifest: ProjectManifest,
    pub deps: BTreeMap<String, MemberDep>,
}

impl Member {
    pub fn manifest_path(&self) -> PathBuf {
        self.dir.join(PROJECT_FILE)
    }

    // "app/liphia.toml", or "liphia.toml" for the root member; used in
    // messages that say who required what.
    pub fn label(&self) -> String {
        if self.rel == "." {
            PROJECT_FILE.to_string()
        } else {
            format!("{}/{}", self.rel, PROJECT_FILE)
        }
    }
}

// A registry package some member asks for, before version resolution.
#[derive(Debug, Clone)]
pub struct Requirement {
    pub name: String,
    pub req: String,
    pub subpackages: Vec<String>,
    pub by: String,
}

// Either a real workspace (root liphia.toml with [workspace]) or a single
// project, which behaves as a workspace of one member. Commands and the
// runner treat both the same way.
#[derive(Debug, Clone)]
pub struct Workspace {
    pub root: PathBuf,
    pub is_workspace: bool,
    pub root_manifest: ProjectManifest,
    pub members: Vec<Member>,
}

impl Workspace {
    // Finds the project that contains `start` (a folder) by walking up to
    // the nearest liphia.toml, then keeps walking to a workspace root that
    // lists it. Ok(None) means no liphia.toml above `start` at all.
    pub fn discover(start: &Path) -> Result<Option<Workspace>, String> {
        let start = normalize(start)?;
        let mut first: Option<(PathBuf, ProjectManifest)> = None;

        for dir in start.ancestors() {
            let path = dir.join(PROJECT_FILE);
            if !path.is_file() {
                continue;
            }
            let manifest = ProjectManifest::load(&path)?;
            let is_root = manifest.workspace.is_some();
            if first.is_none() {
                first = Some((dir.to_path_buf(), manifest.clone()));
            }
            if is_root {
                let ws = Workspace::load(dir, manifest)?;
                let (first_dir, _) = first.as_ref().unwrap();
                if first_dir.as_path() == dir || ws.members.iter().any(|m| &m.dir == first_dir) {
                    return Ok(Some(ws));
                }
                return Err(format!(
                    "{} is inside the workspace at {} but is not one of its members\n  \
                     hint: add its folder to [workspace] members in {}",
                    first_dir.join(PROJECT_FILE).display(),
                    dir.display(),
                    path.display()
                ));
            }
        }

        match first {
            Some((dir, manifest)) => Workspace::single(dir, manifest).map(Some),
            None => Ok(None),
        }
    }

    // A workspace rooted at `root`, whose liphia.toml has [workspace].
    fn load(root: &Path, root_manifest: ProjectManifest) -> Result<Workspace, String> {
        let info = root_manifest.workspace.clone().unwrap_or_default();
        let mut members = vec![];
        if let Some(pkg) = &root_manifest.package {
            members.push(new_member(root, ".".to_string(), pkg, root_manifest.clone())?);
        }
        for pattern in &info.members {
            for dir in expand_member_pattern(root, pattern)? {
                let rel = relative_label(root, &dir);
                let path = dir.join(PROJECT_FILE);
                let manifest = ProjectManifest::load(&path)?;
                if manifest.workspace.is_some() {
                    return Err(format!(
                        "{}: a member cannot declare its own [workspace] (nested workspaces are not supported)",
                        path.display()
                    ));
                }
                let pkg = manifest.package.clone().ok_or_else(|| {
                    format!("{}: a workspace member needs a [package] section", path.display())
                })?;
                if members.iter().any(|m: &Member| m.dir == dir) {
                    continue;
                }
                members.push(new_member(&dir, rel, &pkg, manifest)?);
            }
        }

        let mut seen: HashMap<String, String> = HashMap::new();
        for m in &members {
            if let Some(other) = seen.insert(m.name.clone(), m.label()) {
                return Err(format!(
                    "two workspace members are named '{}': {} and {}",
                    m.name,
                    other,
                    m.label()
                ));
            }
        }

        let mut ws = Workspace {
            root: root.to_path_buf(),
            is_workspace: true,
            root_manifest,
            members,
        };
        ws.expand_all()?;
        Ok(ws)
    }

    fn single(dir: PathBuf, manifest: ProjectManifest) -> Result<Workspace, String> {
        let pkg = manifest
            .package
            .clone()
            .expect("a liphia.toml without [workspace] has [package] (validated on load)");
        let member = new_member(&dir, ".".to_string(), &pkg, manifest.clone())?;
        let mut ws = Workspace {
            root: dir,
            is_workspace: false,
            root_manifest: manifest,
            members: vec![member],
        };
        ws.expand_all()?;
        Ok(ws)
    }

    // Replaces one member's manifest (e.g. after `liphia install <pkg>`
    // added a dependency) and re-validates the whole workspace.
    pub fn set_member_manifest(&mut self, index: usize, manifest: ProjectManifest) -> Result<(), String> {
        self.members[index].manifest = manifest.clone();
        if self.members[index].rel == "." {
            self.root_manifest = manifest;
        }
        self.expand_all()
    }

    // Resolves every member's dependencies to MemberDep and rejects cycles.
    fn expand_all(&mut self) -> Result<(), String> {
        let catalog = self
            .root_manifest
            .workspace
            .as_ref()
            .map(|w| w.dependencies.clone())
            .unwrap_or_default();
        let by_dir: HashMap<PathBuf, usize> = self
            .members
            .iter()
            .enumerate()
            .map(|(i, m)| (m.dir.clone(), i))
            .collect();

        for i in 0..self.members.len() {
            let member = &self.members[i];
            let label = member.label();
            let mut deps = BTreeMap::new();
            for (name, dep) in &member.manifest.dependencies {
                let resolved = match dep {
                    Dependency::Req(_) | Dependency::Registry { .. } => MemberDep::Registry {
                        req: dep.req().unwrap().to_string(),
                        subpackages: dep.subpackages().to_vec(),
                    },
                    Dependency::Path { path } => {
                        self.local_target(&by_dir, &member.dir, path, name, &label)?
                    }
                    Dependency::Workspace { subpackages, .. } => {
                        if !self.is_workspace {
                            return Err(format!(
                                "{}: '{}' uses `workspace = true`, but this project is not in a workspace",
                                label, name
                            ));
                        }
                        let shared = catalog.get(name).ok_or_else(|| {
                            format!(
                                "{}: '{}' uses `workspace = true`, but [workspace.dependencies] in {} has no '{}'",
                                label, name, PROJECT_FILE, name
                            )
                        })?;
                        match shared {
                            Dependency::Path { path } => {
                                self.local_target(&by_dir, &self.root, path, name, PROJECT_FILE)?
                            }
                            _ => {
                                let mut subs = shared.subpackages().to_vec();
                                subs.extend(subpackages.iter().cloned());
                                subs.sort();
                                subs.dedup();
                                MemberDep::Registry {
                                    req: shared.req().unwrap_or("*").to_string(),
                                    subpackages: subs,
                                }
                            }
                        }
                    }
                };
                if resolved == MemberDep::Local(i) {
                    return Err(format!("{}: '{}' depends on itself", label, name));
                }
                deps.insert(name.clone(), resolved);
            }
            self.members[i].deps = deps;
        }

        // Registry names must not shadow members: `import from "x"` would
        // be ambiguous.
        for m in &self.members {
            for (name, dep) in &m.deps {
                if let MemberDep::Registry { .. } = dep {
                    if self.members.iter().any(|o| &o.name == name) {
                        return Err(format!(
                            "{}: '{}' is a workspace member; depend on it with {{ path = \"...\" }} or {{ workspace = true }}",
                            m.label(),
                            name
                        ));
                    }
                }
            }
        }
        self.check_cycles()
    }

    // A path dependency must name a member folder, and its key must be that
    // member's package name, since `import from "<key>"` finds it by name.
    fn local_target(
        &self,
        by_dir: &HashMap<PathBuf, usize>,
        base: &Path,
        path: &str,
        key: &str,
        label: &str,
    ) -> Result<MemberDep, String> {
        if !self.is_workspace {
            return Err(format!(
                "{}: path dependency '{}' needs a workspace; create a root liphia.toml with [workspace] members",
                label, key
            ));
        }
        let dir = normalize(&base.join(path)).map_err(|_| {
            format!("{}: path dependency '{}' points to {}, which does not exist", label, key, path)
        })?;
        let index = *by_dir.get(&dir).ok_or_else(|| {
            format!(
                "{}: path dependency '{}' ({}) is not a member of this workspace\n  \
                 hint: add its folder to [workspace] members in the root {}",
                label, key, path, PROJECT_FILE
            )
        })?;
        let target = &self.members[index];
        if target.name != key {
            return Err(format!(
                "{}: dependency '{}' points to the member named '{}'; use its package name as the key",
                label, key, target.name
            ));
        }
        Ok(MemberDep::Local(index))
    }

    fn check_cycles(&self) -> Result<(), String> {
        // 0 = unvisited, 1 = on the current path, 2 = done.
        fn visit(ws: &Workspace, i: usize, state: &mut Vec<u8>, path: &mut Vec<usize>) -> Result<(), String> {
            state[i] = 1;
            path.push(i);
            for dep in ws.members[i].deps.values() {
                if let MemberDep::Local(j) = dep {
                    match state[*j] {
                        1 => {
                            let start = path.iter().position(|k| k == j).unwrap();
                            let mut names: Vec<&str> =
                                path[start..].iter().map(|k| ws.members[*k].name.as_str()).collect();
                            names.push(&ws.members[*j].name);
                            return Err(format!("dependency cycle between members: {}", names.join(" -> ")));
                        }
                        0 => visit(ws, *j, state, path)?,
                        _ => {}
                    }
                }
            }
            path.pop();
            state[i] = 2;
            Ok(())
        }
        let mut state = vec![0u8; self.members.len()];
        for i in 0..self.members.len() {
            if state[i] == 0 {
                visit(self, i, &mut state, &mut vec![])?;
            }
        }
        Ok(())
    }

    pub fn modules_dir(&self) -> PathBuf {
        self.root.join(MODULES_DIR)
    }

    pub fn lock_path(&self) -> PathBuf {
        self.root.join(LOCK_FILE)
    }

    pub fn member_by_name(&self, name: &str) -> Option<usize> {
        self.members.iter().position(|m| m.name == name)
    }

    // The member whose folder contains `path` (the deepest one, so a member
    // nested under the root member wins over the root).
    pub fn member_at(&self, path: &Path) -> Option<usize> {
        let path = normalize(path).ok()?;
        self.members
            .iter()
            .enumerate()
            .filter(|(_, m)| path.starts_with(&m.dir))
            .max_by_key(|(_, m)| m.dir.components().count())
            .map(|(i, _)| i)
    }

    // `index` plus every member it reaches through local dependencies.
    pub fn local_closure(&self, index: usize) -> Vec<usize> {
        let mut seen = BTreeSet::new();
        let mut stack = vec![index];
        while let Some(i) = stack.pop() {
            if !seen.insert(i) {
                continue;
            }
            for dep in self.members[i].deps.values() {
                if let MemberDep::Local(j) = dep {
                    stack.push(*j);
                }
            }
        }
        seen.into_iter().collect()
    }

    // Every registry package any member asks for, in member order. The
    // installer resolves them together, so conflicting requirements from
    // two members are reported with both labels.
    pub fn registry_requirements(&self) -> Vec<Requirement> {
        let mut out = vec![];
        for m in &self.members {
            for (name, dep) in &m.deps {
                if let MemberDep::Registry { req, subpackages } = dep {
                    out.push(Requirement {
                        name: name.clone(),
                        req: req.clone(),
                        subpackages: subpackages.clone(),
                        by: m.label(),
                    });
                }
            }
        }
        out
    }

    // What a program may import and which native libraries to load when it
    // runs as `member` (None: a file outside every member, which sees the
    // whole workspace).
    pub fn run_plan(&self, member: Option<usize>) -> RunPlan {
        let (scope, members) = match member {
            Some(i) => (
                format!("member '{}' ({})", self.members[i].name, self.members[i].label()),
                self.local_closure(i),
            ),
            None => (
                format!("the workspace at {}", self.root.display()),
                (0..self.members.len()).collect(),
            ),
        };

        let mut packages: BTreeMap<String, PathBuf> = BTreeMap::new();
        let mut scopes: Vec<ImportScope> = vec![];
        let mut registry: Vec<String> = vec![];
        for &i in &members {
            let m = &self.members[i];
            if let Some(entry) = &m.entry {
                packages.insert(m.name.clone(), entry.clone());
            }
            let mut allowed: BTreeSet<String> = m.deps.keys().cloned().collect();
            allowed.insert(m.name.clone());
            scopes.push(ImportScope {
                dir: m.dir.clone(),
                label: format!("member '{}' ({})", m.name, m.label()),
                allowed,
            });
            for (name, dep) in &m.deps {
                if let MemberDep::Registry { .. } = dep {
                    registry.push(name.clone());
                }
            }
        }

        // Transitive registry packages come from the installed package.toml
        // files, which is what `liphia install` resolved.
        let modules = self.modules_dir();
        let mut native_dirs = vec![];
        let mut missing = vec![];
        let mut done: BTreeSet<String> = BTreeSet::new();
        while let Some(name) = registry.pop() {
            if !done.insert(name.clone()) {
                continue;
            }
            let dir = modules.join(&name);
            let manifest = fs::read_to_string(dir.join(PACKAGE_FILE))
                .ok()
                .and_then(|t| PackageManifest::parse(&t).ok());
            let Some(manifest) = manifest else {
                missing.push(name);
                continue;
            };
            packages.insert(name.clone(), dir.join(&manifest.package.entry));
            let mut allowed: BTreeSet<String> = manifest.dependencies.keys().cloned().collect();
            allowed.insert(name.clone());
            scopes.push(ImportScope {
                dir: normalize(&dir).unwrap_or(dir.clone()),
                label: format!("package '{}'", name),
                allowed,
            });
            if manifest.is_native() {
                native_dirs.push(dir.clone());
            }
            registry.extend(manifest.dependencies.keys().cloned());
        }
        missing.sort();

        RunPlan {
            scope,
            packages,
            scopes,
            native_dirs,
            missing,
        }
    }
}

// Which package names the files under `dir` may import: a member's own
// dependencies (plus itself), or an installed package's dependencies.
// The runner picks the deepest scope containing the importing file.
#[derive(Debug, Clone)]
pub struct ImportScope {
    pub dir: PathBuf,
    pub label: String,
    pub allowed: BTreeSet<String>,
}

// What the runner needs from a workspace, as plain data: the pipeline gets
// `packages`, the VM loads `native_dirs`.
#[derive(Debug, Clone)]
pub struct RunPlan {
    // Who declares the dependencies, for error messages.
    pub scope: String,
    // Importable package name -> its entry file (members and installed
    // registry packages in the dependency closure).
    pub packages: BTreeMap<String, PathBuf>,
    // Per-folder visibility inside the closure: a member imports only what
    // its own liphia.toml declares, not what its dependencies declare.
    pub scopes: Vec<ImportScope>,
    // Installed native packages in the closure, in no particular order.
    pub native_dirs: Vec<PathBuf>,
    // Declared registry packages that are not installed yet.
    pub missing: Vec<String>,
}

impl RunPlan {
    // Plan for running `file`: its workspace, restricted to the member that
    // contains it. Ok(None) when the file is not inside any project.
    pub fn for_file(file: &Path) -> Result<Option<RunPlan>, String> {
        let dir = file.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let Some(ws) = Workspace::discover(dir)? else {
            return Ok(None);
        };
        let member = ws.member_at(file);
        Ok(Some(ws.run_plan(member)))
    }
}

fn new_member(dir: &Path, rel: String, pkg: &ProjectInfo, manifest: ProjectManifest) -> Result<Member, String> {
    let entry = match &pkg.entry {
        Some(e) => {
            let path = dir.join(e);
            if !path.is_file() {
                return Err(format!(
                    "{}: entry '{}' does not exist",
                    dir.join(PROJECT_FILE).display(),
                    e
                ));
            }
            Some(path)
        }
        None => None,
    };
    Ok(Member {
        name: pkg.name.clone(),
        version: pkg.version.clone(),
        dir: dir.to_path_buf(),
        rel,
        entry,
        manifest,
        deps: BTreeMap::new(),
    })
}

// Expands one `members` entry to member folders. A segment that is
// exactly "*" matches every subfolder at that level, so "packages/*" and
// "apps/*/api" both work; folders a wildcard reaches without a liphia.toml
// are skipped (in a mixed monorepo they belong to other tools), while a
// literal path without one is an error. Partial wildcards ("api-*") and
// "**" are rejected rather than half-supported.
fn expand_member_pattern(root: &Path, pattern: &str) -> Result<Vec<PathBuf>, String> {
    let pattern = pattern.trim().trim_end_matches('/');
    let segments: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty() && *s != ".").collect();
    let mut has_wildcard = false;
    for seg in &segments {
        if seg.contains('*') {
            if *seg != "*" {
                return Err(format!(
                    "member pattern '{}': a wildcard must be a whole segment (\"apps/*/api\"), not '{}'",
                    pattern, seg
                ));
            }
            has_wildcard = true;
        }
    }

    let mut candidates: Vec<PathBuf> = vec![root.to_path_buf()];
    for seg in &segments {
        let mut next = vec![];
        for dir in &candidates {
            if *seg == "*" {
                let Ok(entries) = fs::read_dir(dir) else {
                    continue;
                };
                let mut subdirs: Vec<PathBuf> = entries
                    .flatten()
                    .map(|e| e.path())
                    .filter(|p| p.is_dir() && !is_skipped_dir(p))
                    .collect();
                subdirs.sort();
                next.extend(subdirs);
            } else {
                let path = dir.join(seg);
                if path.is_dir() || !has_wildcard {
                    next.push(path);
                }
            }
        }
        candidates = next;
    }

    let mut out = vec![];
    for dir in candidates {
        if dir.join(PROJECT_FILE).is_file() {
            out.push(normalize(&dir)?);
        } else if !has_wildcard {
            return Err(format!(
                "workspace member '{}' has no {} ({})",
                pattern,
                PROJECT_FILE,
                dir.display()
            ));
        }
    }
    Ok(out)
}

// Folders a wildcard never descends into: hidden ones and generated ones.
fn is_skipped_dir(path: &Path) -> bool {
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    name.starts_with('.') || name == MODULES_DIR || name == "liphia_cache" || name == "node_modules" || name == "target"
}

fn relative_label(root: &Path, dir: &Path) -> String {
    match dir.strip_prefix(root) {
        Ok(rel) if rel.as_os_str().is_empty() => ".".to_string(),
        Ok(rel) => rel
            .components()
            .filter_map(|c| match c {
                Component::Normal(s) => Some(s.to_string_lossy().to_string()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("/"),
        Err(_) => dir.display().to_string(),
    }
}

// Absolute, symlink-free path, without the \\?\ prefix Windows'
// canonicalize adds, so paths compare equal and print readably.
pub fn normalize(path: &Path) -> Result<PathBuf, String> {
    let canonical = fs::canonicalize(path).map_err(|e| format!("{}: {}", path.display(), e))?;
    let text = canonical.to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\") {
        if !rest.starts_with("UNC\\") {
            return Ok(PathBuf::from(rest));
        }
    }
    Ok(canonical)
}

// ── package.toml ──────────────────────────────────────────────────────────────

pub const PACKAGE_FILE: &str = "package.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    pub package: PackageInfo,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native: Option<NativeInfo>,
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
    #[serde(default)]
    pub subpackages: BTreeMap<String, Subpackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    // Engine versions this package works with. Native packages pin one
    // exact engine ("=2.0.0") because their library uses the Rust ABI.
    pub engine: String,
    pub entry: String,
    pub files: Vec<String>,
}

// Present only in native packages. `lib` is the library name without
// platform prefix or suffix; `abi` names the calling convention.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeInfo {
    pub abi: String,
    pub lib: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subpackage {
    pub files: Vec<String>,
}

impl PackageManifest {
    pub fn parse(text: &str) -> Result<Self, String> {
        toml::from_str(text).map_err(|e| format!("package.toml: {}", e))
    }

    pub fn is_native(&self) -> bool {
        self.native.is_some()
    }

    pub fn engine_req(&self) -> Result<VersionReq, String> {
        parse_req(&self.package.engine)
    }
}

// ── liphia.lock ───────────────────────────────────────────────────────────────

pub const LOCK_FILE: &str = "liphia.lock";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Lockfile {
    #[serde(default = "default_lock_version")]
    pub lock_version: u32,
    #[serde(default)]
    pub engine: String,
    #[serde(default, rename = "package")]
    pub packages: Vec<LockedPackage>,
}

fn default_lock_version() -> u32 {
    1
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub native: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subpackages: Vec<String>,
}

impl Lockfile {
    pub fn load(path: &Path) -> Result<Option<Self>, String> {
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {}", path.display(), e))?;
        toml::from_str(&text)
            .map(Some)
            .map_err(|e| format!("{}: {}", path.display(), e))
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let body = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        let text = format!(
            "# This file is generated by `liphia install`. Do not edit by hand.\n\
             # Commit it so every machine installs exactly the same versions.\n\n{}",
            body
        );
        fs::write(path, text).map_err(|e| format!("{}: {}", path.display(), e))
    }

    pub fn get(&self, name: &str) -> Option<&LockedPackage> {
        self.packages.iter().find(|p| p.name == name)
    }
}

// ── index.toml ────────────────────────────────────────────────────────────────

// Published versions per package, maintained in src/packages/index.toml.
// A version must be listed here before its tag is pushed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Index {
    #[serde(flatten)]
    pub packages: BTreeMap<String, IndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    #[serde(default)]
    pub description: String,
    pub versions: Vec<String>,
}

impl Index {
    pub fn parse(text: &str) -> Result<Self, String> {
        toml::from_str(text).map_err(|e| format!("index.toml: {}", e))
    }

    pub fn latest(&self, name: &str) -> Option<Version> {
        let entry = self.packages.get(name)?;
        highest_matching(&entry.versions, &VersionReq::STAR)
    }
}

// ── Platform naming ───────────────────────────────────────────────────────────

// Platform key used in release asset names: windows-x86_64, linux-x86_64,
// macos-aarch64...
pub fn platform_key() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

// File name the VM loader expects on this platform for a library name:
// liphia_package_num.dll, libliphia_package_num.so, libliphia_package_num.dylib.
pub fn lib_file_name(lib: &str) -> String {
    format!(
        "{}{}{}",
        std::env::consts::DLL_PREFIX,
        lib,
        std::env::consts::DLL_SUFFIX
    )
}

// Release asset name for a library on this platform:
// liphia_package_num-windows-x86_64.dll.
pub fn lib_asset_name(lib: &str) -> String {
    format!("{}-{}{}", lib, platform_key(), std::env::consts::DLL_SUFFIX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_version_is_exact_like_npm() {
        let req = parse_req("1.2.9").unwrap();
        assert!(req.matches(&Version::parse("1.2.9").unwrap()));
        assert!(!req.matches(&Version::parse("1.3.0").unwrap()));
    }

    #[test]
    fn caret_tilde_and_latest() {
        let caret = parse_req("^1.2.0").unwrap();
        assert!(caret.matches(&Version::parse("1.9.0").unwrap()));
        assert!(!caret.matches(&Version::parse("2.0.0").unwrap()));
        let tilde = parse_req("~1.2.0").unwrap();
        assert!(tilde.matches(&Version::parse("1.2.5").unwrap()));
        assert!(!tilde.matches(&Version::parse("1.3.0").unwrap()));
        assert!(parse_req("latest").unwrap().matches(&Version::parse("9.9.9").unwrap()));
    }

    #[test]
    fn highest_matching_picks_the_newest_allowed() {
        let versions: Vec<String> = ["1.0.0", "1.2.9", "1.3.0", "2.0.0"].iter().map(|s| s.to_string()).collect();
        let req = parse_req("^1.0.0").unwrap();
        assert_eq!(highest_matching(&versions, &req), Some(Version::parse("1.3.0").unwrap()));
        let exact = parse_req("1.2.9").unwrap();
        assert_eq!(highest_matching(&versions, &exact), Some(Version::parse("1.2.9").unwrap()));
    }

    #[test]
    fn project_manifest_round_trip() {
        let text = r#"
[package]
name = "demo"

[dependencies]
num = "^1.0.0"
db = { version = "^2.0.0", subpackages = ["sqlite"] }
models = { path = "../models" }
wire = { workspace = true }
"#;
        let m: ProjectManifest = toml::from_str(text).unwrap();
        assert_eq!(m.dependencies["num"].req(), Some("^1.0.0"));
        assert_eq!(m.dependencies["db"].subpackages(), ["sqlite".to_string()]);
        assert_eq!(m.dependencies["models"], Dependency::Path { path: "../models".into() });
        assert!(matches!(m.dependencies["wire"], Dependency::Workspace { workspace: true, .. }));
        let back: ProjectManifest = toml::from_str(&toml::to_string_pretty(&m).unwrap()).unwrap();
        assert_eq!(back.dependencies, m.dependencies);
    }

    #[test]
    fn mixed_dependency_forms_are_rejected() {
        let text = "[package]\nname = \"x\"\n[dependencies]\nnum = { version = \"1.0.0\", path = \"../num\" }\n";
        assert!(toml::from_str::<ProjectManifest>(text).is_err());
    }

    #[test]
    fn package_manifest_and_index_parse() {
        let pkg = PackageManifest::parse(
            r#"
[package]
name = "num"
version = "1.0.0"
engine = "=2.0.0"
entry = "num.lph"
files = ["num.lph"]

[native]
abi = "rust-1"
lib = "liphia_package_num"
"#, 
        )
        .unwrap();
        assert!(pkg.is_native());
        assert!(pkg.engine_req().unwrap().matches(&Version::parse("2.0.0").unwrap()));
        assert!(!pkg.engine_req().unwrap().matches(&Version::parse("2.0.1").unwrap()));

        let index = Index::parse("[num]\nversions = [\"1.0.0\", \"1.1.0\"]\n").unwrap();
        assert_eq!(index.latest("num"), Some(Version::parse("1.1.0").unwrap()));
    }

    // ── Workspace tests, on real folders under the system temp dir ──────────

    struct TempTree(PathBuf);

    impl TempTree {
        fn new(tag: &str) -> TempTree {
            let dir = std::env::temp_dir().join(format!("liphia-manifest-{}-{}", tag, std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            TempTree(dir)
        }

        fn file(&self, rel: &str, text: &str) -> &TempTree {
            let path = self.0.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
            self
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn sample(tag: &str) -> TempTree {
        let t = TempTree::new(tag);
        t.file(
            "liphia.toml",
            "[workspace]\nmembers = [\"app\", \"libs/*\"]\n\n[workspace.dependencies]\nnum = \"^1.0.0\"\ngreet = { path = \"libs/greet\" }\n",
        )
        .file(
            "app/liphia.toml",
            "[package]\nname = \"app\"\nentry = \"main.lph\"\n\n[dependencies]\ngreet = { workspace = true }\nreport = { path = \"../libs/report\" }\n",
        )
        .file("app/main.lph", "print(1)\n")
        .file("libs/greet/liphia.toml", "[package]\nname = \"greet\"\nentry = \"greet.lph\"\n")
        .file("libs/greet/greet.lph", "\n")
        .file(
            "libs/report/liphia.toml",
            "[package]\nname = \"report\"\nentry = \"report.lph\"\n\n[dependencies]\nnum = { workspace = true }\n",
        )
        .file("libs/report/report.lph", "\n");
        t
    }

    #[test]
    fn discovers_workspace_from_a_member_folder() {
        let t = sample("discover");
        let ws = Workspace::discover(&t.0.join("app")).unwrap().unwrap();
        assert!(ws.is_workspace);
        assert_eq!(ws.root, normalize(&t.0).unwrap());
        let names: Vec<&str> = ws.members.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["app", "greet", "report"]);
        let app = ws.member_by_name("app").unwrap();
        assert_eq!(ws.member_at(&t.0.join("app/main.lph")), Some(app));
        assert_eq!(ws.local_closure(app).len(), 3);

        let reqs = ws.registry_requirements();
        assert_eq!(reqs.len(), 1);
        assert_eq!((reqs[0].name.as_str(), reqs[0].req.as_str()), ("num", "^1.0.0"));
        assert_eq!(reqs[0].by, "libs/report/liphia.toml");

        let plan = ws.run_plan(Some(app));
        assert!(plan.packages.contains_key("greet") && plan.packages.contains_key("report"));
        assert_eq!(plan.missing, ["num".to_string()]);
    }

    #[test]
    fn scopes_follow_each_members_own_dependencies() {
        let t = sample("scopes");
        let ws = Workspace::discover(&t.0).unwrap().unwrap();
        let plan = ws.run_plan(ws.member_by_name("app"));
        let app = plan.scopes.iter().find(|s| s.label.starts_with("member 'app'")).unwrap();
        assert!(app.allowed.contains("report") && !app.allowed.contains("num"));
        let report = plan.scopes.iter().find(|s| s.label.starts_with("member 'report'")).unwrap();
        assert!(report.allowed.contains("num"));
    }

    #[test]
    fn a_member_sees_only_its_own_closure() {
        let t = sample("closure");
        let ws = Workspace::discover(&t.0).unwrap().unwrap();
        let greet = ws.member_by_name("greet").unwrap();
        let plan = ws.run_plan(Some(greet));
        assert_eq!(plan.packages.keys().collect::<Vec<_>>(), ["greet"]);
        assert!(plan.missing.is_empty());
    }

    #[test]
    fn wildcard_in_the_middle_of_a_pattern() {
        let t = TempTree::new("glob");
        t.file("liphia.toml", "[workspace]\nmembers = [\"apps/*/api\", \"packages/*\"]\n")
            .file("apps/social/api/liphia.toml", "[package]\nname = \"social-api\"\n")
            .file("apps/search/api/liphia.toml", "[package]\nname = \"search-api\"\n")
            .file("apps/search/web/package.json", "{}\n")
            .file("apps/maps/web/package.json", "{}\n")
            .file("packages/auth/liphia.toml", "[package]\nname = \"lp-auth\"\n")
            .file("packages/ts-only/package.json", "{}\n");
        let ws = Workspace::discover(&t.0).unwrap().unwrap();
        let rels: Vec<&str> = ws.members.iter().map(|m| m.rel.as_str()).collect();
        assert_eq!(rels, ["apps/search/api", "apps/social/api", "packages/auth"]);
    }

    #[test]
    fn partial_wildcards_are_rejected() {
        let t = TempTree::new("partial");
        t.file("liphia.toml", "[workspace]\nmembers = [\"apps/api-*\"]\n");
        let err = Workspace::discover(&t.0).unwrap_err();
        assert!(err.contains("whole segment"), "{}", err);
    }

    #[test]
    fn cycles_between_members_are_rejected() {
        let t = TempTree::new("cycle");
        t.file("liphia.toml", "[workspace]\nmembers = [\"a\", \"b\"]\n")
            .file("a/liphia.toml", "[package]\nname = \"a\"\n[dependencies]\nb = { path = \"../b\" }\n")
            .file("b/liphia.toml", "[package]\nname = \"b\"\n[dependencies]\na = { path = \"../a\" }\n");
        let err = Workspace::discover(&t.0).unwrap_err();
        assert!(err.contains("cycle"), "{}", err);
    }

    #[test]
    fn path_dependency_must_be_a_member() {
        let t = TempTree::new("outside");
        t.file("liphia.toml", "[workspace]\nmembers = [\"a\"]\n")
            .file("a/liphia.toml", "[package]\nname = \"a\"\n[dependencies]\nb = { path = \"../b\" }\n")
            .file("b/liphia.toml", "[package]\nname = \"b\"\n");
        let err = Workspace::discover(&t.0.join("a")).unwrap_err();
        assert!(err.contains("not a member"), "{}", err);
    }

    #[test]
    fn project_not_listed_in_enclosing_workspace_is_an_error() {
        let t = TempTree::new("unlisted");
        t.file("liphia.toml", "[workspace]\nmembers = []\n")
            .file("stray/liphia.toml", "[package]\nname = \"stray\"\n");
        let err = Workspace::discover(&t.0.join("stray")).unwrap_err();
        assert!(err.contains("not one of its members"), "{}", err);
    }

    #[test]
    fn single_project_is_a_workspace_of_one() {
        let t = TempTree::new("single");
        t.file("liphia.toml", "[package]\nname = \"solo\"\n[dependencies]\nnum = \"^1.0.0\"\n");
        let ws = Workspace::discover(&t.0).unwrap().unwrap();
        assert!(!ws.is_workspace);
        assert_eq!(ws.members.len(), 1);
        assert_eq!(ws.registry_requirements()[0].by, "liphia.toml");
    }
}
