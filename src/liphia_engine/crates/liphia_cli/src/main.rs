// liphia_cli/src/main.rs
mod cache;
mod installer;
mod repl;

use liphia_core_native;
use liphia_stdlib_native;
use std::collections::HashSet;
use std::path::PathBuf;
use std::process;

use liphia_pipeline::{compile, resolve_project};
use liphia_virtual_machine::vm::VM;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("init") => {
            installer::init_project();
            return;
        }
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
        None | Some("--repl") => {
            repl::start();
            return;
        }
        _ => {}
    }

    let source_path = PathBuf::from(&args[1]);
    let use_cache   = !args.contains(&"--no-cache".to_string());
    let mut visited = HashSet::new();
    let stmts = resolve_project(&source_path, &source_path, &mut visited);

    let hash_input = format!("{:?}", stmts);
    let hash = cache::source_hash(&hash_input);

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
                let compiled = compile(stmts);
                cache::save_cache(&source_path, hash, &compiled);
                eprintln!(
                    "[liphia] compiled and cached ({}.lbc)",
                    source_path.file_stem().unwrap_or_default().to_string_lossy()
                );
                compiled
            }
        }
    } else {
        compile(stmts)
    };

    let mut vm = VM::new();
    liphia_core_native::register(&mut vm);
    liphia_stdlib_native::register_all(&mut vm);
    if let Err(e) = vm.run(opcodes) {
        eprintln!("\n{}\n", e);
        process::exit(1);
    }
}