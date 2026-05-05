// liphia_cli/src/main.rs
mod cache;
mod installer;
mod repl;

use liphia_core_native;
use liphia_stdlib_native;

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use liphia_compiler::bytecode::generate_bytecode;
use liphia_compiler::lexer::Lexer;
use liphia_compiler::parser::Parser;
use liphia_compiler::type_checker::TypeChecker;
use liphia_virtual_machine::opcode::Opcode;
use liphia_virtual_machine::vm::VM;

// ── Local file import resolution ──────────────────────────────────────────────
fn resolve_import_file(base_dir: &Path, import_path: &str) -> Option<PathBuf> {
    let path = PathBuf::from(import_path);
    if path.is_absolute() && path.exists() {
        return Some(path);
    }
    let local = base_dir.join(import_path);
    if local.exists() {
        return Some(local);
    }
    None
}


// ── Stdlib module resolution ──────────────────────────────────────────────────
//
// Resolution order:
//   1. ./liphia_modules/<name>/<name>.lph — installed using `liphia install`
//   2. LIPHIA_STDLIB_PATH env var         — global install / CI
//   3. Paths relative to exe              — distributed binary
//   4. Paths relative to cwd              — development layout


fn resolve_stdlib_module(module_name: &str, source_root: &Path) -> Option<PathBuf> {
    let name     = module_name.trim_end_matches(".lph");
    let filename = format!("{}.lph", name);

    // 1) liphia_modules/ next to the source file being run  ← liphia install
    let source_dir     = source_root.parent().unwrap_or(Path::new("."));
    let local_modules  = source_dir
        .join("liphia_modules")
        .join(name)
        .join(&filename);
    if local_modules.exists() {
        return Some(local_modules);
    }

    // 2) liphia_modules/ in current working directory
    if let Ok(cwd) = std::env::current_dir() {
        let cwd_modules = cwd.join("liphia_modules").join(name).join(&filename);
        if cwd_modules.exists() {
            return Some(cwd_modules);
        }
    }

    // 3) LIPHIA_STDLIB_PATH environment variable
    if let Ok(std_path) = std::env::var("LIPHIA_STDLIB_PATH") {
        let candidate = PathBuf::from(&std_path).join(&filename);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    // 4) Relative to the compiled binary (distributed install)
    if let Ok(exe) = std::env::current_exe() {
        let exe_dir = exe.parent();
        let candidates = [
            exe_dir.map(|p| p.join("stdlib/lph").join(&filename)),
            exe_dir.and_then(|p| p.parent()).map(|p| p.join("stdlib/lph").join(&filename)),
            exe_dir.and_then(|p| p.parent()).and_then(|p| p.parent())
                   .map(|p| p.join("stdlib/lph").join(&filename)),
        ];
        for c in candidates.into_iter().flatten() {
            if c.exists() { return Some(c); }
        }
    }

    // 5) Relative to cwd — development layout
    //    Running from liphia_engine/crates/ → ../../.. reaches liphia/
    let cwd = std::env::current_dir().unwrap_or_default();
    let dev_candidates = [
    cwd.join("stdlib/modules").join(name).join(&filename),
    cwd.join("../src/stdlib/modules").join(name).join(&filename),
    // going up from crates/liphia_cli to src
    cwd.join("../../stdlib/modules").join(name).join(&filename),
    cwd.join("../../../stdlib/modules").join(name).join(&filename),
    cwd.join("../../../../stdlib/modules").join(name).join(&filename),
    // liphia_modules/ installed by liphia install
    cwd.join("liphia_modules").join(name).join(&filename),
    // older candidates
    cwd.join("../../../liphia-stdlib/lph").join(&filename),
    cwd.join("../../liphia-stdlib/lph").join(&filename),
    cwd.join("../../stdlib/lph").join(&filename),
    cwd.join("../../stdlib").join(&filename),
    cwd.join("../stdlib/lph").join(&filename),
    cwd.join("stdlib/lph").join(&filename),
    ];
    for c in &dev_candidates {
        if c.exists() { return Some(c.clone()); }
    }

    // Nothing found — print diagnostics
    eprintln!("[liphia] error: stdlib module '{}' not found.", name);
    eprintln!("  hint: run 'liphia install {}' to install it.", name);
    eprintln!("        or set LIPHIA_STDLIB_PATH=/path/to/stdlib/lph");
    eprintln!("  cwd:  {:?}", std::env::current_dir().unwrap_or_default());

    None
}

// ── Source loading ────────────────────────────────────────────────────────────
fn load_file_recursive(
    path:        &Path,
    visited:     &mut HashSet<PathBuf>,
    source_root: &Path,
) -> String {
    let abs = fs::canonicalize(path).unwrap_or_else(|_| {
        eprintln!("error: file not found: {:?}", path);
        process::exit(1);
    });
    if visited.contains(&abs) {
        return String::new();
    }
    visited.insert(abs.clone());

    let source = fs::read_to_string(&abs).unwrap_or_else(|e| {
        eprintln!("error: could not read {:?}: {}", abs, e);
        process::exit(1);
    });
    let base_dir = abs.parent().unwrap_or(Path::new("."));
    let mut output = String::new();

    for line in source.lines() {
        let trimmed = line.trim();

        // import from "http"  →  stdlib / liphia_modules
        if trimmed.starts_with("import from ") {
            let rest        = trimmed.strip_prefix("import from").unwrap().trim();
            let module_name = rest.trim_matches('"').trim_matches('\'');
            let resolved    = resolve_stdlib_module(module_name, source_root)
                .unwrap_or_else(|| process::exit(1));
            output.push_str(&load_file_recursive(&resolved, visited, source_root));
            output.push('\n');
            continue;
        }

        // import "utils.lph"  →  local file (relative or absolute)
        if trimmed.starts_with("import ") {
            let rest        = trimmed.strip_prefix("import").unwrap().trim();
            let import_path = rest.trim_matches('"').trim_matches('\'');
            let resolved    = resolve_import_file(base_dir, import_path).unwrap_or_else(|| {
                eprintln!("error: could not resolve import '{}'", import_path);
                eprintln!("hint: imports are relative to the current file, or use absolute paths.");
                process::exit(1);
            });
            output.push_str(&load_file_recursive(&resolved, visited, source_root));
            output.push('\n');
            continue;
        }

        output.push_str(line);
        output.push('\n');
    }
    output
}

// ── Compilation pipeline ──────────────────────────────────────────────────────
fn compile(full_source: &str) -> Vec<Opcode> {
    let lexer = Lexer::new(full_source);
    let mut parser = Parser::new(lexer).unwrap_or_else(|e| {
        eprintln!("\n{}\n", e);
        process::exit(1);
    });
    let ast = parser.parse().unwrap_or_else(|e| {
        eprintln!("\n{}\n", e);
        process::exit(1);
    });
    let mut checker = TypeChecker::new();
    if let Err(e) = checker.check(&ast) {
        eprintln!("\n{}\n", e);
        process::exit(1);
    }
    let program = generate_bytecode(ast).unwrap_or_else(|e| {
        eprintln!("\n{}\n", e);
        process::exit(1);
    });
    program.instructions
}

// ── Entry point ───────────────────────────────────────────────────────────────
fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        // liphia init
        Some("init") => {
            installer::init_project();
            return;
        }
        // liphia install [mod1 mod2 ...]
        Some("install") => {
            let modules: Vec<&str> = args[2..]
                .iter()
                .map(|s| s.as_str())
                .filter(|s| !s.starts_with('-'))
                .collect();
            if args.contains(&"--list".to_string()) {
                installer::list_modules();
            } else if modules.is_empty() {
                installer::install_from_manifest();
            } else {
                installer::install_modules(&modules);
            }
            return;
        }
        // liphia --repl  or  no args
        None | Some("--repl") => {
            repl::start();
            return;
        }
        // liphia <file.lph> [--no-cache]
        _ => {}
    }

    let source_path = PathBuf::from(&args[1]);
    let use_cache   = !args.contains(&"--no-cache".to_string());

    let mut visited = HashSet::new();
    let full_source = load_file_recursive(&source_path, &mut visited, &source_path);

    let hash    = cache::source_hash(&full_source);
    let opcodes = if use_cache {
        match cache::load_cache(&source_path, hash) {
            Some(cached) => {
                eprintln!(
                    "[liphia] using cached bytecode ({}.lbc)",
                    source_path.file_stem().unwrap_or_default().to_string_lossy()
                );
                cached
            }
            None => {
                let compiled = compile(&full_source);
                cache::save_cache(&source_path, hash, &compiled);
                eprintln!(
                    "[liphia] compiled and cached ({}.lbc)",
                    source_path.file_stem().unwrap_or_default().to_string_lossy()
                );
                compiled
            }
        }
    } else {
        compile(&full_source)
    };

    let mut vm = VM::new();
    liphia_core_native::register(&mut vm);
    liphia_stdlib_native::register_all(&mut vm);

    if let Err(e) = vm.run(opcodes) {
        eprintln!("\n{}\n", e);
        process::exit(1);
    }
}