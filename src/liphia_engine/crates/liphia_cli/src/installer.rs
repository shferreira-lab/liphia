// liphia_cli/src/installer.rs
// Package manager for Liphia.
//
// Packages are downloaded from src/packages/<name>/ of the Liphia repo into
// liphia_modules/<name>/ of the project. The file list comes from the
// package's module.toml; native packages also fetch the prebuilt library
// for the current platform, declared under [external.libs].

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;

use liphia_virtual_machine::vm::VM;

const REGISTRY_RAW: &str =
    "https://raw.githubusercontent.com/shferreira-lab/liphia/main/src/packages";

const MODULES_DIR: &str = "liphia_modules";
const MANIFEST: &str = "liphia.toml";

// Official packages. Everything else Liphia offers is a core native,
// compiled into the binary and never installed.
const KNOWN_MODULES: &[&str] = &["db", "learn", "num", "stats", "wire"];

// ── liphia init ───────────────────────────────────────────────────────────────
pub fn init_project() {
    if Path::new(MANIFEST).exists() {
        println!("[liphia] liphia.toml already exists.");
        return;
    }
    let project_name = std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "my_project".to_string());

    let content = format!(
        r#"[package]
name    = "{}"
version = "0.1.0"

[dependencies]
# liphia install <package> adds entries here automatically
"#,
        project_name
    );

    fs::write(MANIFEST, content).unwrap_or_else(|e| {
        eprintln!("[liphia] failed to create liphia.toml: {}", e);
        process::exit(1);
    });
    println!("[liphia] created liphia.toml");
    println!("[liphia] run 'liphia install <package>' to add dependencies.");
}

// ── liphia install --list ─────────────────────────────────────────────────────
pub fn list_modules() {
    println!("[liphia] available packages:");
    for m in KNOWN_MODULES {
        let installed = PathBuf::from(MODULES_DIR)
            .join(m)
            .join(format!("{}.lph", m))
            .exists();
        let status = if installed {
            "✓ installed"
        } else {
            "  available"
        };
        println!("  {}  {}", status, m);
    }
}

// ── liphia install <mod> [mod2 ...] ──────────────────────────────────────────
pub fn install_modules(modules: &[&str]) {
    let (mut ok, mut err) = (0usize, 0usize);
    for &spec in modules {
        let success = match spec.split_once(':') {
            Some((module, submodule)) => do_install_submodule(module, submodule),
            None => do_install(spec),
        };
        if success {
            ok += 1;
        } else {
            err += 1;
        }
    }
    println!();
    if err == 0 {
        println!("[liphia] {} package(s) installed.", ok);
    } else {
        println!("[liphia] {} installed, {} failed.", ok, err);
    }
}

// ── Submodule install: liphia install <module>:<submodule> ───────────────────
// Delegates to do_install(module) first, so the module's base files AND its
// platform-specific prebuilt native lib (see do_install) are guaranteed to
// exist before we layer the submodule's own files on top.
fn do_install_submodule(module: &str, submodule: &str) -> bool {
    if !do_install(module) {
        return false;
    }
    print!("  installing '{}:{}'... ", module, submodule);
    io::stdout().flush().unwrap();

    if !KNOWN_MODULES.contains(&module) {
        println!("FAILED");
        eprintln!("    '{}' is not a known package.", module);
        eprintln!("    known: {}", KNOWN_MODULES.join(", "));
        return false;
    }

    let toml_url = format!("{}/{}/module.toml", REGISTRY_RAW, module);
    let module_toml = match http_get(&toml_url) {
        Ok(body) => body,
        Err(e) => {
            println!("FAILED");
            eprintln!("    could not fetch module.toml for '{}': {}", module, e);
            return false;
        }
    };

    let files = match parse_submodule_files(&module_toml, submodule) {
        Some(f) => f,
        None => {
            println!("FAILED");
            eprintln!(
                "    '{}' has no submodule '{}' in its module.toml",
                module, submodule
            );
            return false;
        }
    };

    let dest_dir = PathBuf::from(MODULES_DIR).join(module);
    if let Err(e) = fs::create_dir_all(&dest_dir) {
        println!("FAILED");
        eprintln!("    could not create {}: {}", dest_dir.display(), e);
        return false;
    }
    let _ = fs::write(dest_dir.join("module.toml"), &module_toml);

    let (mut ok_count, mut err_count) = (0usize, 0usize);
    for rel_path in &files {
        let dest_file = dest_dir.join(rel_path);
        if dest_file.exists() {
            ok_count += 1;
            continue;
        }
        if let Some(parent) = dest_file.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                println!("FAILED");
                eprintln!("    could not create {}: {}", parent.display(), e);
                err_count += 1;
                continue;
            }
        }
        let url = format!("{}/{}/{}", REGISTRY_RAW, module, rel_path);
        match http_get(&url) {
            Ok(body) => match fs::write(&dest_file, &body) {
                Ok(_) => ok_count += 1,
                Err(e) => {
                    println!("FAILED");
                    eprintln!("    could not write {}: {}", dest_file.display(), e);
                    err_count += 1;
                }
            },
            Err(e) => {
                println!("FAILED");
                eprintln!("    download error fetching '{}': {}", rel_path, e);
                err_count += 1;
            }
        }
    }

    if err_count > 0 {
        return false;
    }
    add_to_manifest(&format!("{}:{}", module, submodule));
    println!("ok ({} file(s))", ok_count);
    true
}

// Parses `files = [...]` from a `[submodules.<name>]` section of module.toml.
fn parse_submodule_files(toml: &str, submodule: &str) -> Option<Vec<String>> {
    let section = format!("[submodules.{}]", submodule);
    let mut in_section = false;
    for line in toml.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_section = t == section;
            continue;
        }
        if !in_section {
            continue;
        }
        if let Some(rest) = t.strip_prefix("files") {
            let rest = rest.trim_start();
            let rest = rest.strip_prefix('=')?.trim();
            let inner = rest.strip_prefix('[')?.strip_suffix(']')?;
            let list: Vec<String> = inner
                .split(',')
                .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !list.is_empty() {
                return Some(list);
            }
        }
    }
    None
}

// ── liphia install (reading liphia.toml) ───────────────────────────────
pub fn install_from_manifest() {
    let content = fs::read_to_string(MANIFEST).unwrap_or_else(|_| {
        eprintln!("[liphia] liphia.toml not found. Run 'liphia init' first.");
        process::exit(1);
    });
    let deps = parse_dependencies(&content);
    if deps.is_empty() {
        println!("[liphia] no dependencies declared in liphia.toml.");
        return;
    }
    println!("[liphia] installing {} package(s)...", deps.len());
    let (mut ok, mut err) = (0usize, 0usize);
    for name in deps.keys() {
        if do_install(name) {
            ok += 1;
        } else {
            err += 1;
        }
    }
    println!();
    if err == 0 {
        println!("[liphia] all {} package(s) installed.", ok);
    } else {
        println!("[liphia] {} installed, {} failed.", ok, err);
    }
}

// ── install a package ────────────────────────────────────────────────────────
// Installs a package the user asked for: recorded in liphia.toml.
fn do_install(name: &str) -> bool {
    install_package(name, true, &mut HashSet::new())
}

// Installs `name` and, first, every package listed under [dependencies] in
// its module.toml. Only direct installs are recorded in liphia.toml;
// dependencies are pulled in again from each package's own module.toml.
// `seen` stops cycles and repeated work within one install command.
fn install_package(name: &str, direct: bool, seen: &mut HashSet<String>) -> bool {
    if !seen.insert(name.to_string()) {
        return true;
    }
    print!("  installing '{}'... ", name);
    io::stdout().flush().unwrap();

    if !KNOWN_MODULES.contains(&name) {
        println!("FAILED");
        eprintln!("    '{}' is not a known package.", name);
        eprintln!("    known: {}", KNOWN_MODULES.join(", "));
        return false;
    }

    let dest_dir = PathBuf::from(MODULES_DIR).join(name);
    if let Err(e) = fs::create_dir_all(&dest_dir) {
        println!("FAILED");
        eprintln!("    could not create {}: {}", dest_dir.display(), e);
        return false;
    }

    // module.toml first — it tells us which files belong to this module.
    let toml_url = format!("{}/{}/module.toml", REGISTRY_RAW, name);
    let module_toml = http_get(&toml_url).ok();
    if let Some(ref body) = module_toml {
        let _ = fs::write(dest_dir.join("module.toml"), body);
        let deps: Vec<String> = parse_dependencies(body).into_keys().collect();
        if !deps.is_empty() {
            println!("needs {}", deps.join(", "));
            for dep in &deps {
                if !install_package(dep, false, seen) {
                    eprintln!("    dependency '{}' of '{}' failed", dep, name);
                    return false;
                }
            }
            print!("  installing '{}'... ", name);
            io::stdout().flush().unwrap();
        }
    }

    // File list from module.toml's `files = [...]` under [module].
    // Falls back to just "<name>.lph" for modules that haven't published
    // a file list yet — backward compatible with the old single-file layout.
    let mut files = module_toml
        .as_deref()
        .and_then(parse_module_files)
        .unwrap_or_else(|| vec![format!("{}.lph", name)]);

    // Native packages (db, num, stats, learn) also fetch the prebuilt
    // library for the current platform, declared under [external.libs] in
    // module.toml. Pure packages have no such section and get None here.
    if let Some(lib_path) = module_toml.as_deref().and_then(parse_external_lib) {
        files.push(lib_path);
    }

    let mut ok_count = 0usize;
    let mut err_count = 0usize;

    for rel_path in &files {
        let dest_file = dest_dir.join(rel_path);
        // Skip files already present — lets a re-run pick up NEW files
        // added to the module's list without re-downloading everything.
        if dest_file.exists() {
            ok_count += 1;
            continue;
        }
        if let Some(parent) = dest_file.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                println!("FAILED");
                eprintln!("    could not create {}: {}", parent.display(), e);
                err_count += 1;
                continue;
            }
        }
        let url = format!("{}/{}/{}", REGISTRY_RAW, name, rel_path);
        let fetch_result = if is_binary_file(rel_path) {
            http_get_binary(&url).map(WriteBody::Binary)
        } else {
            http_get(&url).map(WriteBody::Text)
        };
        match fetch_result {
            Ok(body) => {
                if let Some(parent) = dest_file.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let write_result = match &body {
                    WriteBody::Text(s) => fs::write(&dest_file, s),
                    WriteBody::Binary(b) => fs::write(&dest_file, b),
                };
                if let Err(e) = write_result {
                    println!("FAILED");
                    eprintln!("    could not write {}: {}", dest_file.display(), e);
                    err_count += 1;
                } else {
                    #[cfg(unix)]
                    if is_binary_file(rel_path) {
                        make_executable(&dest_file);
                    }
                    ok_count += 1;
                }
            }
            Err(e) => {
                println!("FAILED");
                eprintln!("    download error fetching '{}': {}", rel_path, e);
                eprintln!("    url tried: {}", url);
                err_count += 1;
            }
        }
    }

    if err_count > 0 {
        let _ = fs::remove_dir_all(&dest_dir);
        return false;
    }

    if direct {
        add_to_manifest(name);
    }
    println!("ok ({} file(s))", ok_count);
    true
}

// A downloaded file's body, kept as raw bytes for prebuilt native
// libraries (.so/.dll/.dylib) so binary content survives the round trip
// intact — treating it as UTF-8 text (the old behavior) corrupts it.
enum WriteBody {
    Text(String),
    Binary(Vec<u8>),
}

/// Files that must be downloaded and written as raw bytes rather than
/// UTF-8 text — currently just the prebuilt native libraries that
/// native packages ship under their `lib/` directory.
fn is_binary_file(rel_path: &str) -> bool {
    rel_path.ends_with(".so") || rel_path.ends_with(".dll") || rel_path.ends_with(".dylib")
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_mode(perms.mode() | 0o111);
        let _ = fs::set_permissions(path, perms);
    }
}

// Parses `files = ["a.lph", "b.lph"]` from the [module] section of a
// module.toml. Minimal hand-rolled parser — same approach as
// parse_dependencies below, no external toml crate dependency.
fn parse_module_files(toml: &str) -> Option<Vec<String>> {
    let mut in_module = false;
    for line in toml.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_module = t == "[module]";
            continue;
        }
        if !in_module {
            continue;
        }
        if let Some(rest) = t.strip_prefix("files") {
            let rest = rest.trim_start();
            let rest = rest.strip_prefix('=')?.trim();
            let inner = rest.strip_prefix('[')?.strip_suffix(']')?;
            let list: Vec<String> = inner
                .split(',')
                .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !list.is_empty() {
                return Some(list);
            }
        }
    }
    None
}

// Returns "windows" / "macos" / "linux" depending on the platform this CLI
// was compiled for — used to pick the right prebuilt lib entry out of
// module.toml's [external.libs] section.
fn current_platform_lib_key() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

// Parses a relative lib path (e.g. "lib/liphia_module_db.dll") for the
// current platform out of a `[external.libs]` section of module.toml:
//
//   [external.libs]
//   linux   = "lib/libliphia_module_db.so"
//   windows = "lib/liphia_module_db.dll"
//   macos   = "lib/libliphia_module_db.dylib"
//
// Returns None for packages with no such section (pure packages),
// or if this platform has no prebuilt lib listed yet.
fn parse_external_lib(toml: &str) -> Option<String> {
    let key = current_platform_lib_key();
    let mut in_section = false;
    for line in toml.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_section = t == "[external.libs]";
            continue;
        }
        if !in_section {
            continue;
        }
        if let Some((k, v)) = t.split_once('=') {
            if k.trim() == key {
                return Some(v.trim().trim_matches('"').trim_matches('\'').to_string());
            }
        }
    }
    None
}

// ── HTTP GET by curl ─────────────────────────────────────────────────────────
fn http_get(url: &str) -> Result<String, String> {
    let curl = std::process::Command::new("curl")
        .args(["-fsSL", "--max-time", "15", url])
        .output();

    match curl {
        Ok(out) if out.status.success() => {
            String::from_utf8(out.stdout).map_err(|e| format!("invalid utf-8: {}", e))
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            Err(format!("curl error ({}): {}", out.status, stderr.trim()))
        }
        Err(_) => {
            // try wget - fallback
            let wget = std::process::Command::new("wget")
                .args(["-qO-", "--timeout=15", url])
                .output();
            match wget {
                Ok(out) if out.status.success() => {
                    String::from_utf8(out.stdout).map_err(|e| format!("invalid utf-8: {}", e))
                }
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    Err(format!("wget error: {}", stderr.trim()))
                }
                Err(_) => {
                    Err("curl not found. Install from: https://curl.se/download.html".to_string())
                }
            }
        }
    }
}

// ── Loading installed native packages ─────────────────────────────────────────
//
// Native packages ship prebuilt libraries (see liphia_virtual_machine::
// external). This loads every installed package under liphia_modules/ that
// has an index.lph. Failures are warnings, not fatal: a project that never
// calls a package's natives should not be blocked by it failing to load.
pub fn load_installed_packages(vm: &mut VM) {
    for (name, err) in vm.load_installed_external_modules(MODULES_DIR) {
        eprintln!(
            "[liphia] warning: failed to load package '{}': {}",
            name, err.message
        );
    }
}

// ── HTTP GET (binary) by curl ─────────────────────────────────────────────────
// Same as http_get, but returns raw bytes instead of forcing UTF-8 — needed
// for prebuilt native libraries (.so/.dll/.dylib), which are not text.
fn http_get_binary(url: &str) -> Result<Vec<u8>, String> {
    let curl = std::process::Command::new("curl")
        .args(["-fsSL", "--max-time", "30", url])
        .output();

    match curl {
        Ok(out) if out.status.success() => Ok(out.stdout),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            Err(format!("curl error ({}): {}", out.status, stderr.trim()))
        }
        Err(_) => {
            let wget = std::process::Command::new("wget")
                .args(["-qO-", "--timeout=30", url])
                .output();
            match wget {
                Ok(out) if out.status.success() => Ok(out.stdout),
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    Err(format!("wget error: {}", stderr.trim()))
                }
                Err(_) => {
                    Err("curl not found. Install from: https://curl.se/download.html".to_string())
                }
            }
        }
    }
}

// ── liphia.toml helpers ───────────────────────────────────────────────────────
fn parse_dependencies(toml: &str) -> HashMap<String, String> {
    let mut deps = HashMap::new();
    let mut in_deps = false;
    for line in toml.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_deps = t == "[dependencies]";
            continue;
        }
        if !in_deps || t.starts_with('#') || t.is_empty() {
            continue;
        }
        if let Some((k, v)) = t.split_once('=') {
            deps.insert(k.trim().to_string(), v.trim().trim_matches('"').to_string());
        }
    }
    deps
}

fn add_to_manifest(name: &str) {
    let Ok(content) = fs::read_to_string(MANIFEST) else {
        return;
    };
    let already = content.lines().any(|l| {
        let t = l.trim();
        !t.starts_with('#') && t.starts_with(&format!("{} =", name))
    });
    if already {
        return;
    }
    let entry = format!("{} = \"latest\"\n", name);
    let new_content = if content.contains("[dependencies]") {
        content.replacen("[dependencies]\n", &format!("[dependencies]\n{}", entry), 1)
    } else {
        format!("{}\n[dependencies]\n{}", content.trim_end(), entry)
    };
    let _ = fs::write(MANIFEST, new_content);
}
