// liphia_cli/src/main.rs
mod cache;
mod installer;
mod repl;

use liphia_core_native;
use std::collections::HashSet;
use std::path::PathBuf;
use std::process;

use liphia_pipeline::{compile, resolve_project};
use liphia_virtual_machine::vm::VM;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let names = |from: usize| -> Vec<&str> {
        args.iter()
            .skip(from)
            .map(|s| s.as_str())
            .filter(|s| !s.starts_with('-'))
            .collect()
    };
    let flag = |f: &str| args.iter().any(|a| a == f);

    match args.get(1).map(|s| s.as_str()) {
        Some("init") => {
            installer::init_project();
            return;
        }
        Some("install") => {
            if flag("--list") {
                installer::list_packages();
            } else {
                installer::install(&names(2), flag("--frozen"));
            }
            return;
        }
        Some("list") => {
            installer::list_packages();
            return;
        }
        Some("update") => {
            installer::update(&names(2));
            return;
        }
        Some("remove") => {
            installer::remove(&names(2));
            return;
        }
        Some("version") | Some("--version") | Some("-V") => {
            println!("liphia {}", env!("CARGO_PKG_VERSION"));
            println!("abi    {}", liphia_virtual_machine::external::abi_tag());
            return;
        }
        Some("help") | Some("--help") | Some("-h") => {
            print_help();
            return;
        }
        None | Some("--repl") => {
            repl::start();
            return;
        }
        _ => {}
    }

    let source_path = PathBuf::from(&args[1]);
    let use_cache = !args.contains(&"--no-cache".to_string());
    let mut visited = HashSet::new();

    let stmts = resolve_project(&source_path, &source_path, &mut visited).unwrap_or_else(|e| {
        eprintln!("{}", e);
        process::exit(1);
    });

    let hash_input = format!("{:?}", stmts);
    let hash = cache::source_hash(&hash_input);

    let opcodes = if use_cache {
        match cache::load_cache(&source_path, hash) {
            Some(cached) => {
                eprintln!(
                    "[liphia] using cached bytecode ({}.lbc)",
                    source_path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                );
                cached
            }
            None => {
                let compiled = compile(stmts).unwrap_or_else(|e| {
                    eprintln!("{}", e);
                    process::exit(1);
                });
                cache::save_cache(&source_path, hash, &compiled);
                eprintln!(
                    "[liphia] compiled and cached ({}.lbc)",
                    source_path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                );
                compiled
            }
        }
    } else {
        compile(stmts).unwrap_or_else(|e| {
            eprintln!("{}", e);
            process::exit(1);
        })
    };

    let mut vm = VM::new();
    liphia_core_native::register(&mut vm);
    installer::load_installed_packages(&mut vm);
    if let Err(e) = vm.run(opcodes) {
        eprintln!("\n{}\n", e);
        process::exit(1);
    }
}

fn print_help() {
    println!(
        "liphia {} — Liphia language runtime

usage:
  liphia <file.lph> [--no-cache]   compile (cached) and run a program
  liphia                           interactive REPL (also: liphia --repl)

packages:
  liphia init                      create liphia.toml in this folder
  liphia install                   install everything in liphia.toml (uses liphia.lock)
  liphia install --frozen          install exactly liphia.lock, never change it (CI)
  liphia install <pkg>[@req]       add a package: num, num@1.2.9, num@^1.2, num@latest
  liphia install <pkg>:<sub>       add a package with a subpackage: db:sqlite
  liphia update [pkg ...]          move packages to the newest version their requirement allows
  liphia remove <pkg> [pkg ...]    drop packages from liphia.toml and liphia_modules/
  liphia list                      published packages and installed versions

other:
  liphia version                   engine version and native ABI tag
  liphia help                      this message

environment:
  LIPHIA_HOME       cache and global files (default: ~/.liphia)
  LIPHIA_REGISTRY   install from a local packages folder instead of GitHub",
        env!("CARGO_PKG_VERSION")
    );
}
