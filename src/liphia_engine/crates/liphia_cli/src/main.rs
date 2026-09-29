// liphia_cli/src/main.rs
mod cache;
mod installer;
mod repl;

use liphia_core_native;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process;

use liphia_manifest::{RunPlan, Workspace};
use liphia_pipeline::{compile, resolve_project_with, PackageRoots, Visibility};
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
    let use_cache = !flag("--no-cache");

    match args.get(1).map(|s| s.as_str()) {
        Some("init") => {
            installer::init_project(flag("--workspace"));
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
        Some("members") => {
            installer::list_members();
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
        Some("run") => {
            let entry = member_entry(names(2).first().copied());
            run_file(&entry, use_cache);
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

    run_file(&PathBuf::from(&args[1]), use_cache);
}

// The entry file of a workspace member: the one named, or the one the
// current folder belongs to, or the only member there is.
fn member_entry(name: Option<&str>) -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_default();
    let ws = Workspace::discover(&cwd)
        .unwrap_or_else(|e| fail(&e))
        .unwrap_or_else(|| fail("liphia run needs a liphia.toml here or in a parent folder"));

    let index = match name {
        Some(n) => ws.member_by_name(n).unwrap_or_else(|| {
            let known: Vec<&str> = ws.members.iter().map(|m| m.name.as_str()).collect();
            fail(&format!("no member named '{}' (members: {})", n, known.join(", ")))
        }),
        None => match ws.member_at(&cwd) {
            Some(i) => i,
            None if ws.members.len() == 1 => 0,
            None => {
                let runnable: Vec<&str> = ws
                    .members
                    .iter()
                    .filter(|m| m.entry.is_some())
                    .map(|m| m.name.as_str())
                    .collect();
                fail(&format!(
                    "which member? use 'liphia run <member>' (with an entry: {})",
                    runnable.join(", ")
                ))
            }
        },
    };
    let member = &ws.members[index];
    member.entry.clone().unwrap_or_else(|| {
        fail(&format!(
            "member '{}' has no entry; set `entry = \"main.lph\"` under [package] in {}",
            member.name,
            member.label()
        ))
    })
}

// Resolves imports (scoped to the file's project, if any), compiles with
// the bytecode cache, loads the natives the project needs and runs.
fn run_file(source_path: &Path, use_cache: bool) {
    let plan: Option<RunPlan> = RunPlan::for_file(source_path).unwrap_or_else(|e| fail(&e));
    if let Some(plan) = &plan {
        if !plan.missing.is_empty() {
            eprintln!(
                "[liphia] warning: not installed yet: {} (run 'liphia install')",
                plan.missing.join(", ")
            );
        }
    }
    let roots = plan.as_ref().map(package_roots).unwrap_or_default();

    let mut visited = HashSet::new();
    let stmts = resolve_project_with(source_path, source_path, &mut visited, &roots)
        .unwrap_or_else(|e| fail_raw(&e));

    let hash_input = format!("{:?}", stmts);
    let hash = cache::source_hash(&hash_input);
    let stem = source_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let opcodes = if use_cache {
        match cache::load_cache(source_path, hash) {
            Some(cached) => {
                eprintln!("[liphia] using cached bytecode ({}.lbc)", stem);
                cached
            }
            None => {
                let compiled = compile(stmts).unwrap_or_else(|e| fail_raw(&e));
                cache::save_cache(source_path, hash, &compiled);
                eprintln!("[liphia] compiled and cached ({}.lbc)", stem);
                compiled
            }
        }
    } else {
        compile(stmts).unwrap_or_else(|e| fail_raw(&e))
    };

    let mut vm = VM::new();
    liphia_core_native::register(&mut vm);
    installer::load_native_packages(&mut vm, plan.as_ref());
    if let Err(e) = vm.run(opcodes) {
        eprintln!("\n{}\n", e);
        process::exit(1);
    }
}

// The pipeline's view of a run plan: importable packages plus per-folder
// visibility, as plain paths and names.
fn package_roots(plan: &RunPlan) -> PackageRoots {
    let visibility = plan
        .scopes
        .iter()
        .map(|s| Visibility {
            dir: s.dir.clone(),
            label: s.label.clone(),
            allowed: s.allowed.iter().cloned().collect(),
        })
        .collect();
    PackageRoots::scoped(plan.scope.clone(), plan.packages.clone()).with_visibility(visibility)
}

fn fail(message: &str) -> ! {
    eprintln!("[liphia] error: {}", message);
    process::exit(1);
}

// Compiler and resolver errors already carry their own prefix and layout.
fn fail_raw(message: &str) -> ! {
    eprintln!("{}", message);
    process::exit(1);
}

fn print_help() {
    println!(
        "liphia {} — Liphia language runtime

usage:
  liphia <file.lph> [--no-cache]   compile (cached) and run a program
  liphia run [member] [--no-cache] run a project's entry (or a workspace member's)
  liphia                           interactive REPL (also: liphia --repl)

packages:
  liphia init                      create liphia.toml in this folder
  liphia init --workspace          create a workspace root liphia.toml
  liphia install                   install everything the project or workspace needs (uses liphia.lock)
  liphia install --frozen          install exactly liphia.lock, never change it (CI)
  liphia install <pkg>[@req]       add a package: num, num@1.2.9, num@^1.2, num@latest
  liphia install <pkg>:<sub>       add a package with a subpackage: db:sqlite
  liphia update [pkg ...]          move packages to the newest version their requirement allows
  liphia remove <pkg> [pkg ...]    drop packages from liphia.toml and liphia_modules/
  liphia list                      published packages and installed versions
  liphia members                   workspace members and their dependencies

In a workspace, liphia.lock and liphia_modules/ live at the root; install
and remove edit the liphia.toml of the member you are in.

other:
  liphia version                   engine version and native ABI tag
  liphia help                      this message

environment:
  LIPHIA_HOME       cache and global files (default: ~/.liphia)
  LIPHIA_REGISTRY   install from a local packages folder instead of GitHub",
        env!("CARGO_PKG_VERSION")
    );
}
