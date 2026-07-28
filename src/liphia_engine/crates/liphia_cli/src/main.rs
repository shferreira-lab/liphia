// liphia_cli/src/main.rs
mod cache;
mod installer;
mod repl;

use liphia_core_native;
use liphia_stdlib_native;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use liphia_compiler::ast::{Expr, Stmt};
use liphia_compiler::bytecode::generate_bytecode;
use liphia_compiler::lexer::Lexer;
use liphia_compiler::parser::Parser;
use liphia_compiler::type_checker::TypeChecker;
use liphia_virtual_machine::opcode::Opcode;
use liphia_virtual_machine::vm::VM;

// ── Import directives ──────────────────────────────────────────────────────
//
// Recognized forms:
//
//   import "file.lph"                     → BareLocal   (unqualified, all names)
//   import from "module"                  → BareStdlib  (unqualified, stdlib)
//   import { a, b } from "file.lph"       → Selective   (unqualified, only listed names)
//   import alias from "file.lph"          → Qualified   (all names, mangled under `alias::`)
//
// Qualified/Selective apply only to local .lph files. Stdlib modules keep
// the old flat/unqualified resolution.

#[derive(Debug, Clone)]
enum ImportKind {
    BareLocal,
    BareStdlib,
    Qualified(String),
    Selective(Vec<String>),
}

#[derive(Debug, Clone)]
struct ImportDirective {
    kind:   ImportKind,
    target: String,
}

fn strip_quotes(s: &str) -> String {
    s.trim().trim_matches('"').trim_matches('\'').to_string()
}

/// Parses a single trimmed line as an import directive, or returns None
/// if it isn't one. Tokenizes explicitly instead of relying on chained
/// substring checks, so unusual spacing can't silently fall through to
/// the wrong branch.
fn parse_import_line(trimmed: &str) -> Option<ImportDirective> {
    if !trimmed.starts_with("import") { return None; }
    let rest = trimmed["import".len()..].trim_start();
    if rest.is_empty() { return None; }

    // import { a, b } from "target"
    if let Some(after_brace) = rest.strip_prefix('{') {
        let close = after_brace.find('}')?;
        let names: Vec<String> = after_brace[..close]
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let after   = after_brace[close + 1..].trim();
        let after   = after.strip_prefix("from")?.trim();
        let target  = strip_quotes(after);
        if target.is_empty() { return None; }
        return Some(ImportDirective { kind: ImportKind::Selective(names), target });
    }

    // import from "target"   (bare stdlib)
    if let Some(after) = rest.strip_prefix("from") {
        let target = strip_quotes(after);
        if target.is_empty() { return None; }
        return Some(ImportDirective { kind: ImportKind::BareStdlib, target });
    }

    // import alias from "target"   (qualified local import)
    let mut parts = rest.splitn(2, char::is_whitespace);
    let first     = parts.next().unwrap_or("");
    let remainder = parts.next().unwrap_or("").trim_start();
    if !first.is_empty()
        && !first.starts_with('"')
        && !first.starts_with('\'')
        && remainder.starts_with("from")
    {
        let after_from = remainder["from".len()..].trim_start();
        let target = strip_quotes(after_from);
        if !target.is_empty() {
            return Some(ImportDirective { kind: ImportKind::Qualified(first.to_string()), target });
        }
    }

    // import "file.lph"   (bare local import) — fallback
    let target = strip_quotes(rest);
    if target.is_empty() { return None; }
    Some(ImportDirective { kind: ImportKind::BareLocal, target })
}

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

    let source_dir     = source_root.parent().unwrap_or(Path::new("."));
    let local_modules  = source_dir
        .join("liphia_modules")
        .join(name)
        .join(&filename);
    if local_modules.exists() {
        return Some(local_modules);
    }

    if let Ok(cwd) = std::env::current_dir() {
        let cwd_modules = cwd.join("liphia_modules").join(name).join(&filename);
        if cwd_modules.exists() {
            return Some(cwd_modules);
        }
    }

    if let Ok(std_path) = std::env::var("LIPHIA_STDLIB_PATH") {
        let candidate = PathBuf::from(&std_path).join(&filename);
        if candidate.exists() {
            return Some(candidate);
        }
    }

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

    let cwd = std::env::current_dir().unwrap_or_default();
    let dev_candidates = [
        cwd.join("stdlib/modules").join(name).join(&filename),
        cwd.join("../src/stdlib/modules").join(name).join(&filename),
        cwd.join("../../stdlib/modules").join(name).join(&filename),
        cwd.join("../../../stdlib/modules").join(name).join(&filename),
        cwd.join("../../../../stdlib/modules").join(name).join(&filename),
        cwd.join("liphia_modules").join(name).join(&filename),
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

    eprintln!("[liphia] error: stdlib module '{}' not found.", name);
    eprintln!("  hint: run 'liphia install {}' to install it.", name);
    eprintln!("        or set LIPHIA_STDLIB_PATH=/path/to/stdlib/lph");
    eprintln!("  cwd:  {:?}", std::env::current_dir().unwrap_or_default());
    None
}

// ── Per-file parse: strips import lines, records them structurally,
//    then lexes+parses the remaining source into that file's own AST ───────────

fn parse_own_source(path: &Path) -> (Vec<Stmt>, Vec<ImportDirective>) {
    let source = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("error: could not read {:?}: {}", path, e);
        process::exit(1);
    });

    let mut imports = vec![];
    let mut clean_source = String::new();

    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(directive) = parse_import_line(trimmed) {
            imports.push(directive);
            clean_source.push('\n');
            continue;
        }
        clean_source.push_str(line);
        clean_source.push('\n');
    }

    let lexer = Lexer::new(&clean_source);
    let mut parser = Parser::new(lexer).unwrap_or_else(|e| {
        eprintln!("\n{}\n", e);
        process::exit(1);
    });
    let stmts = parser.parse().unwrap_or_else(|e| {
        eprintln!("\n{}\n", e);
        process::exit(1);
    });

    (stmts, imports)
}

// ── Top-level name helpers (used for selective filtering & collision check) ────

fn stmt_name(stmt: &Stmt) -> Option<String> {
    match stmt {
        Stmt::Fn { name, .. }    => Some(name.clone()),
        Stmt::Const { name, .. } => Some(name.clone()),
        Stmt::Enum(def)          => Some(def.name.clone()),
        _ => None,
    }
}

fn mangle_stmt_name(stmt: &mut Stmt, alias: &str) {
    match stmt {
        Stmt::Fn { name, .. }    => *name = format!("{}::{}", alias, name),
        Stmt::Const { name, .. } => *name = format!("{}::{}", alias, name),
        Stmt::Enum(def)          => def.name = format!("{}::{}", alias, def.name),
        _ => {}
    }
}

fn check_no_duplicate_top_level_names(stmts: &[Stmt]) {
    let mut seen: HashMap<String, ()> = HashMap::new();
    for stmt in stmts {
        if let Some(name) = stmt_name(stmt) {
            if seen.contains_key(&name) {
                eprintln!(
                    "error: '{}' is declared more than once across imported files.",
                    name
                );
                eprintln!(
                    "hint: use a qualified import (`import alias from \"...\"`) to avoid name collisions."
                );
                process::exit(1);
            }
            seen.insert(name, ());
        }
    }
}

// ── Rewrite module.fn(...) into a mangled FunctionCall, using this file's
//    own alias table (built from its Qualified imports) ───────────────────────

fn resolve_module_calls_in_stmt(stmt: &mut Stmt, aliases: &HashMap<String, String>) {
    match stmt {
        Stmt::VarDecl { value, .. } => resolve_module_calls_in_expr(value, aliases),
        Stmt::Var { value, .. }     => resolve_module_calls_in_expr(value, aliases),
        Stmt::Const { value, .. }   => resolve_module_calls_in_expr(value, aliases),
        Stmt::Assign { value, .. }  => resolve_module_calls_in_expr(value, aliases),
        Stmt::AssignIndex { index, value, .. } => {
            resolve_module_calls_in_expr(index, aliases);
            resolve_module_calls_in_expr(value, aliases);
        }
        Stmt::Print(args) => {
            for a in args { resolve_module_calls_in_expr(a, aliases); }
        }
        Stmt::If { condition, block, branches, else_block } => {
            resolve_module_calls_in_expr(condition, aliases);
            for s in block { resolve_module_calls_in_stmt(s, aliases); }
            for (cond, blk) in branches {
                resolve_module_calls_in_expr(cond, aliases);
                for s in blk { resolve_module_calls_in_stmt(s, aliases); }
            }
            if let Some(blk) = else_block {
                for s in blk { resolve_module_calls_in_stmt(s, aliases); }
            }
        }
        Stmt::While { condition, body } => {
            resolve_module_calls_in_expr(condition, aliases);
            for s in body { resolve_module_calls_in_stmt(s, aliases); }
        }
        Stmt::For { from, to, step, body, .. } => {
            resolve_module_calls_in_expr(from, aliases);
            resolve_module_calls_in_expr(to, aliases);
            if let Some(s) = step { resolve_module_calls_in_expr(s, aliases); }
            for s in body { resolve_module_calls_in_stmt(s, aliases); }
        }
        Stmt::Fn { body, .. } => {
            for s in body { resolve_module_calls_in_stmt(s, aliases); }
        }
        Stmt::Try { try_block, catch_block, .. } => {
            for s in try_block { resolve_module_calls_in_stmt(s, aliases); }
            for s in catch_block { resolve_module_calls_in_stmt(s, aliases); }
        }
        Stmt::ExprStmt(expr) => resolve_module_calls_in_expr(expr, aliases),
        Stmt::Return(expr)   => resolve_module_calls_in_expr(expr, aliases),
        Stmt::Break | Stmt::Continue | Stmt::Enum(_) => {}
    }
}

fn resolve_module_calls_in_expr(expr: &mut Expr, aliases: &HashMap<String, String>) {
    match expr {
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) |
        Expr::Eq(a, b)  | Expr::NotEq(a, b) |
        Expr::Gt(a, b)  | Expr::Lt(a, b) | Expr::Gte(a, b) | Expr::Lte(a, b) |
        Expr::And(a, b) | Expr::Or(a, b) => {
            resolve_module_calls_in_expr(a, aliases);
            resolve_module_calls_in_expr(b, aliases);
        }
        Expr::Not(inner) | Expr::Await(inner) => resolve_module_calls_in_expr(inner, aliases),
        Expr::List(items) => {
            for item in items { resolve_module_calls_in_expr(item, aliases); }
        }
        Expr::MapLiteral(pairs) => {
            for (k, v) in pairs {
                resolve_module_calls_in_expr(k, aliases);
                resolve_module_calls_in_expr(v, aliases);
            }
        }
        Expr::Index(a, b) => {
            resolve_module_calls_in_expr(a, aliases);
            resolve_module_calls_in_expr(b, aliases);
        }
        Expr::FunctionCall { args, .. } | Expr::Spawn { args, .. } => {
            for a in args { resolve_module_calls_in_expr(a, aliases); }
        }
        Expr::ModuleCall { module, name, args } => {
            for a in args.iter_mut() { resolve_module_calls_in_expr(a, aliases); }
            if let Some(prefix) = aliases.get(module) {
                let mangled    = format!("{}::{}", prefix, name);
                let taken_args = std::mem::take(args);
                *expr = Expr::FunctionCall { name: mangled, args: taken_args };
            } else {
                eprintln!(
                    "error: '{}' is not an imported module alias (used as '{}.{}(...)')",
                    module, module, name
                );
                eprintln!("hint: add `import {} from \"...\"` at the top of the file.", module);
                process::exit(1);
            }
        }
        Expr::Int(_) | Expr::Float(_) | Expr::Str(_) | Expr::Bool(_) | Expr::Null |
        Expr::Variable(_) | Expr::EnumVariant { .. } => {}
    }
}

// ── Recursive project resolution: parses each reachable file separately,
//    applies its import directives, and returns one merged, flat Vec<Stmt> ─────

fn resolve_project(
    entry_path:  &Path,
    source_root: &Path,
    visited:     &mut HashSet<PathBuf>,
) -> Vec<Stmt> {
    let abs = fs::canonicalize(entry_path).unwrap_or_else(|_| {
        eprintln!("error: file not found: {:?}", entry_path);
        process::exit(1);
    });
    if visited.contains(&abs) {
        return vec![];
    }
    visited.insert(abs.clone());

    let (mut own_stmts, imports) = parse_own_source(&abs);
    let base_dir = abs.parent().unwrap_or(Path::new("."));

    let mut merged: Vec<Stmt> = vec![];
    let mut aliases: HashMap<String, String> = HashMap::new();

    for imp in &imports {
        match &imp.kind {
            ImportKind::BareStdlib => {
                if let Some(resolved) = resolve_stdlib_module(&imp.target, source_root) {
                    let (stmts, _) = parse_own_source(&resolved);
                    merged.extend(stmts);
                }
            }
            ImportKind::BareLocal => {
                let resolved = resolve_import_file(base_dir, &imp.target).unwrap_or_else(|| {
                    eprintln!("error: could not resolve import '{}'", imp.target);
                    eprintln!("hint: imports are relative to the current file, or use absolute paths.");
                    process::exit(1);
                });
                merged.extend(resolve_project(&resolved, source_root, visited));
            }
            ImportKind::Selective(names) => {
                let resolved = resolve_import_file(base_dir, &imp.target).unwrap_or_else(|| {
                    eprintln!("error: could not resolve import '{}'", imp.target);
                    process::exit(1);
                });
                let module_stmts = resolve_project(&resolved, source_root, visited);
                for stmt in module_stmts {
                    if stmt_name(&stmt).map_or(false, |n| names.contains(&n)) {
                        merged.push(stmt);
                    }
                }
            }
            ImportKind::Qualified(alias) => {
                let resolved = resolve_import_file(base_dir, &imp.target).unwrap_or_else(|| {
                    eprintln!("error: could not resolve import '{}'", imp.target);
                    process::exit(1);
                });
                let mut module_stmts = resolve_project(&resolved, source_root, visited);
                for stmt in module_stmts.iter_mut() {
                    mangle_stmt_name(stmt, alias);
                }
                aliases.insert(alias.clone(), alias.clone());
                merged.extend(module_stmts);
            }
        }
    }

    for stmt in own_stmts.iter_mut() {
        resolve_module_calls_in_stmt(stmt, &aliases);
    }

    merged.extend(own_stmts);
    check_no_duplicate_top_level_names(&merged);
    merged
}

// ── Compilation pipeline ──────────────────────────────────────────────────────

fn compile(stmts: Vec<Stmt>) -> Vec<Opcode> {
    let mut checker = TypeChecker::new();
    if let Err(e) = checker.check(&stmts) {
        eprintln!("\n{}\n", e);
        process::exit(1);
    }
    let program = generate_bytecode(stmts).unwrap_or_else(|e| {
        eprintln!("\n{}\n", e);
        process::exit(1);
    });
    program.instructions
}

// ── Entry point ───────────────────────────────────────────────────────────────

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

    // Cache invalidation hashes the resolved AST's Debug representation,
    // since imports are no longer textually inlined before hashing.
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