// liphia_cli/src/installer.rs
// Package manager for Liphia.

use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;

const REGISTRY_RAW: &str =
    "https://raw.githubusercontent.com/shferreira-lab/liphia/main/src/stdlib/modules";

const MODULES_DIR: &str = "liphia_modules";
const MANIFEST:    &str = "liphia.toml";

const KNOWN_MODULES: &[&str] = &[
    "http", "db", "ws", "net", "fs", "math", "json", "ai", "stats",
];

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
# liphia install <module> adds entries here automatically
"#,
        project_name
    );

    fs::write(MANIFEST, content).unwrap_or_else(|e| {
        eprintln!("[liphia] failed to create liphia.toml: {}", e);
        process::exit(1);
    });
    println!("[liphia] created liphia.toml");
    println!("[liphia] run 'liphia install <module>' to add dependencies.");
}

// ── liphia install --list ─────────────────────────────────────────────────────
pub fn list_modules() {
    println!("[liphia] available stdlib modules:");
    for m in KNOWN_MODULES {
        let installed = PathBuf::from(MODULES_DIR)
            .join(m)
            .join(format!("{}.lph", m))
            .exists();
        let status = if installed { "✓ installed" } else { "  available" };
        println!("  {}  {}", status, m);
    }
}

// ── liphia install <mod> [mod2 ...] ──────────────────────────────────────────
pub fn install_modules(modules: &[&str]) {
    let (mut ok, mut err) = (0usize, 0usize);
    for &name in modules {
        if do_install(name) { ok += 1; } else { err += 1; }
    }
    println!();
    if err == 0 {
        println!("[liphia] {} module(s) installed.", ok);
    } else {
        println!("[liphia] {} installed, {} failed.", ok, err);
    }
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
    println!("[liphia] installing {} module(s)...", deps.len());
    let (mut ok, mut err) = (0usize, 0usize);
    for name in deps.keys() {
        if do_install(name) { ok += 1; } else { err += 1; }
    }
    println!();
    if err == 0 {
        println!("[liphia] all {} module(s) installed.", ok);
    } else {
        println!("[liphia] {} installed, {} failed.", ok, err);
    }
}

// ── install a module ─────────────────────────────────────────────────────────
fn do_install(name: &str) -> bool {
    print!("  installing '{}'... ", name);
    io::stdout().flush().unwrap();

    if !KNOWN_MODULES.contains(&name) {
        println!("FAILED");
        eprintln!("    '{}' is not a known stdlib module.", name);
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
    let toml_url    = format!("{}/{}/module.toml", REGISTRY_RAW, name);
    let module_toml = http_get(&toml_url).ok();
    if let Some(ref body) = module_toml {
        let _ = fs::write(dest_dir.join("module.toml"), body);
    }

    // File list from module.toml's `files = [...]` under [module].
    // Falls back to just "<name>.lph" for modules that haven't published
    // a file list yet — backward compatible with the old single-file layout.
    let files = module_toml
        .as_deref()
        .and_then(parse_module_files)
        .unwrap_or_else(|| vec![format!("{}.lph", name)]);

    let mut ok_count  = 0usize;
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
        match http_get(&url) {
            Ok(body) => {
                if let Err(e) = fs::write(&dest_file, &body) {
                    println!("FAILED");
                    eprintln!("    could not write {}: {}", dest_file.display(), e);
                    err_count += 1;
                } else {
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

    add_to_manifest(name);
    println!("ok ({} file(s))", ok_count);
    true
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
        if !in_module { continue; }
        if let Some(rest) = t.strip_prefix("files") {
            let rest  = rest.trim_start();
            let rest  = rest.strip_prefix('=')?.trim();
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
// ── HTTP GET by curl ─────────────────────────────────────────────────────────
fn http_get(url: &str) -> Result<String, String> {
    let curl = std::process::Command::new("curl")
        .args(["-fsSL", "--max-time", "15", url])
        .output();

    match curl {
        Ok(out) if out.status.success() => {
            String::from_utf8(out.stdout)
                .map_err(|e| format!("invalid utf-8: {}", e))
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
                    String::from_utf8(out.stdout)
                        .map_err(|e| format!("invalid utf-8: {}", e))
                }
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    Err(format!("wget error: {}", stderr.trim()))
                }
                Err(_) => Err(
                    "curl not found. Install from: https://curl.se/download.html".to_string()
                ),
            }
        }
    }
}

// ── liphia.toml helpers ───────────────────────────────────────────────────────
fn parse_dependencies(toml: &str) -> HashMap<String, String> {
    let mut deps    = HashMap::new();
    let mut in_deps = false;
    for line in toml.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_deps = t == "[dependencies]";
            continue;
        }
        if !in_deps || t.starts_with('#') || t.is_empty() { continue; }
        if let Some((k, v)) = t.split_once('=') {
            deps.insert(k.trim().to_string(), v.trim().trim_matches('"').to_string());
        }
    }
    deps
}

fn add_to_manifest(name: &str) {
    let Ok(content) = fs::read_to_string(MANIFEST) else { return };
    let already = content.lines().any(|l| {
        let t = l.trim();
        !t.starts_with('#') && t.starts_with(&format!("{} =", name))
    });
    if already { return; }
    let entry       = format!("{} = \"latest\"\n", name);
    let new_content = if content.contains("[dependencies]") {
        content.replacen("[dependencies]\n", &format!("[dependencies]\n{}", entry), 1)
    } else {
        format!("{}\n[dependencies]\n{}", content.trim_end(), entry)
    };
    let _ = fs::write(MANIFEST, new_content);
}