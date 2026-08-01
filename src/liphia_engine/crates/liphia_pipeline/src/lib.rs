// liphia_engine/crates/liphia_pipeline/src/lib.rs
//
// Shared compilation pipeline: import resolution + type checking + bytecode
// generation. Used by both liphia_cli (runs to completion via vm.run()) and
// liphia_cli_gui (runs incrementally via VmSession::tick(), driven by the
// window's own event loop). Extracted from liphia_cli's main.rs — behavior
// unchanged.
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
fn parse_import_line(trimmed: &str) -> Option<ImportDirective> {
    if !trimmed.starts_with("import") { return None; }
    let rest = trimmed["import".len()..].trim_start();
    if rest.is_empty() { return None; }
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
    if let Some(after) = rest.strip_prefix("from") {
        let target = strip_quotes(after);
        if target.is_empty() { return None; }
        return Some(ImportDirective { kind: ImportKind::BareStdlib, target });
    }
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
    let target = strip_quotes(rest);
    if target.is_empty() { return None; }
    Some(ImportDirective { kind: ImportKind::BareLocal, target })
}
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
fn find_in_stdlib_roots(relative: &Path, source_root: &Path) -> Option<PathBuf> {
    let source_dir = source_root.parent().unwrap_or(Path::new("."));
    let mut candidates: Vec<PathBuf> = vec![
        source_dir.join("liphia_modules").join(relative),
    ];
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("liphia_modules").join(relative));
    }
    if let Ok(std_path) = std::env::var("LIPHIA_STDLIB_PATH") {
        candidates.push(PathBuf::from(&std_path).join(relative));
    }
    if let Ok(exe) = std::env::current_exe() {
        let exe_dir = exe.parent();
        if let Some(d) = exe_dir {
            candidates.push(d.join("stdlib/lph").join(relative));
        }
        if let Some(d) = exe_dir.and_then(|p| p.parent()) {
            candidates.push(d.join("stdlib/lph").join(relative));
        }
        if let Some(d) = exe_dir.and_then(|p| p.parent()).and_then(|p| p.parent()) {
            candidates.push(d.join("stdlib/lph").join(relative));
        }
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    for base in [
        "stdlib/modules",
        "../src/stdlib/modules",
        "../../stdlib/modules",
        "../../../stdlib/modules",
        "../../../../stdlib/modules",
        "liphia_modules",
    ] {
        candidates.push(cwd.join(base).join(relative));
    }
    candidates.into_iter().find(|c| c.exists())
}
fn resolve_stdlib_module(module_name: &str, source_root: &Path) -> Option<PathBuf> {
    let name = module_name.trim_end_matches(".lph");
    let rel  = PathBuf::from(name).join(format!("{}.lph", name));
    let found = find_in_stdlib_roots(&rel, source_root);
    if found.is_none() {
        eprintln!("[liphia] error: stdlib module '{}' not found.", name);
        eprintln!("  hint: run 'liphia install {}' to install it.", name);
        eprintln!("        or set LIPHIA_STDLIB_PATH=/path/to/stdlib/lph");
        eprintln!("  cwd:  {:?}", std::env::current_dir().unwrap_or_default());
    }
    found
}
fn resolve_stdlib_submodule(module_name: &str, submodule_name: &str, source_root: &Path) -> Option<PathBuf> {
    let rel = PathBuf::from(module_name)
        .join(submodule_name)
        .join(format!("{}.lph", submodule_name));
    find_in_stdlib_roots(&rel, source_root)
}
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

// ── Public entry point 1: resolve a project into a merged, flat Vec<Stmt> ──
pub fn resolve_project(
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
                    merged.extend(resolve_project(&resolved, source_root, visited));
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
                if let Some(resolved) = resolve_import_file(base_dir, &imp.target) {
                    let module_stmts = resolve_project(&resolved, source_root, visited);
                    for stmt in module_stmts {
                        if stmt_name(&stmt).map_or(false, |n| names.contains(&n)) {
                            merged.push(stmt);
                        }
                    }
                } else {
                    let mut remaining: Vec<String> = vec![];
                    for name in names {
                        if let Some(sub_path) = resolve_stdlib_submodule(&imp.target, name, source_root) {
                            merged.extend(resolve_project(&sub_path, source_root, visited));
                        } else {
                            remaining.push(name.clone());
                        }
                    }
                    if !remaining.is_empty() {
                        let entry = resolve_stdlib_module(&imp.target, source_root)
                            .unwrap_or_else(|| process::exit(1));
                        let module_stmts = resolve_project(&entry, source_root, visited);
                        for stmt in module_stmts {
                            if stmt_name(&stmt).map_or(false, |n| remaining.contains(&n)) {
                                merged.push(stmt);
                            }
                        }
                    }
                }
            }
            ImportKind::Qualified(alias) => {
                let resolved = resolve_import_file(base_dir, &imp.target)
                    .or_else(|| resolve_stdlib_module(&imp.target, source_root))
                    .unwrap_or_else(|| process::exit(1));
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

// ── Public entry point 2: type-check + generate bytecode ──────────────────
pub fn compile(stmts: Vec<Stmt>) -> Vec<Opcode> {
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