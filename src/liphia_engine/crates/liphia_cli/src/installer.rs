// liphia_cli/src/installer.rs
//
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

// ── liphia install (sem args → lê liphia.toml) ───────────────────────────────
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

// ── instala um módulo ─────────────────────────────────────────────────────────
fn do_install(name: &str) -> bool {
    print!("  installing '{}'... ", name);
    io::stdout().flush().unwrap();

    if !KNOWN_MODULES.contains(&name) {
        println!("FAILED");
        eprintln!("    '{}' is not a known stdlib module.", name);
        eprintln!("    known: {}", KNOWN_MODULES.join(", "));
        return false;
    }

    // destino: <cwd>/liphia_modules/<name>/<name>.lph
    let dest_dir  = PathBuf::from(MODULES_DIR).join(name);
    let dest_file = dest_dir.join(format!("{}.lph", name));

    if dest_file.exists() {
        println!("already installed.");
        return true;
    }

    if let Err(e) = fs::create_dir_all(&dest_dir) {
        println!("FAILED");
        eprintln!("    could not create {}: {}", dest_dir.display(), e);
        return false;
    }

    // baixa <name>.lph do GitHub
    let url = format!("{}/{}/{}.lph", REGISTRY_RAW, name, name);
    match http_get(&url) {
        Ok(body) => {
            if let Err(e) = fs::write(&dest_file, &body) {
                println!("FAILED");
                eprintln!("    could not write {}: {}", dest_file.display(), e);
                let _ = fs::remove_dir_all(&dest_dir);
                return false;
            }
        }
        Err(e) => {
            println!("FAILED");
            eprintln!("    download error: {}", e);
            eprintln!("    url tried: {}", url);
            let _ = fs::remove_dir_all(&dest_dir);
            return false;
        }
    }

    // baixa module.toml (opcional)
    let toml_url = format!("{}/{}/module.toml", REGISTRY_RAW, name);
    if let Ok(body) = http_get(&toml_url) {
        let _ = fs::write(dest_dir.join("module.toml"), body);
    }

    add_to_manifest(name);
    println!("ok");
    true
}

// ── HTTP GET via curl ─────────────────────────────────────────────────────────
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
            // tenta wget como fallback
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