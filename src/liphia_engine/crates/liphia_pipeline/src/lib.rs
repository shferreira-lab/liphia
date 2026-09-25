// liphia_engine/crates/liphia_pipeline/src/lib.rs
//
// Shared compilation pipeline: import resolution + type checking + bytecode
// generation. Used by both liphia_cli (runs to completion via vm.run()) and
// liphia_cli_gui (runs incrementally via VmSession::tick(), driven by the
// window's own event loop). Extracted from liphia_cli's main.rs — behavior
// unchanged, except errors are now returned as Result instead of exiting the
// process directly, so GUI hosts (which have no terminal to print to and
// must not just vanish on a compile error) can display them instead.
use liphia_compiler::ast::Type;
use liphia_compiler::ast::{Expr, Stmt};
use liphia_compiler::bytecode::generate_bytecode;
use liphia_compiler::lexer::Lexer;
use liphia_compiler::parser::Parser;
use liphia_compiler::type_checker::TypeChecker;
use liphia_virtual_machine::opcode::Opcode;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
enum ImportKind {
    BareLocal,
    BarePackage,
    Qualified(String),
    Selective(Vec<String>),
}
#[derive(Debug, Clone)]
struct ImportDirective {
    kind: ImportKind,
    target: String,
}
fn strip_quotes(s: &str) -> String {
    s.trim().trim_matches('"').trim_matches('\'').to_string()
}
fn parse_import_line(trimmed: &str) -> Option<ImportDirective> {
    if !trimmed.starts_with("import") {
        return None;
    }
    let rest = trimmed["import".len()..].trim_start();
    if rest.is_empty() {
        return None;
    }
    if let Some(after_brace) = rest.strip_prefix('{') {
        let close = after_brace.find('}')?;
        let names: Vec<String> = after_brace[..close]
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let after = after_brace[close + 1..].trim();
        let after = after.strip_prefix("from")?.trim();
        let target = strip_quotes(after);
        if target.is_empty() {
            return None;
        }
        return Some(ImportDirective {
            kind: ImportKind::Selective(names),
            target,
        });
    }
    if let Some(after) = rest.strip_prefix("from") {
        let target = strip_quotes(after);
        if target.is_empty() {
            return None;
        }
        return Some(ImportDirective {
            kind: ImportKind::BarePackage,
            target,
        });
    }
    let mut parts = rest.splitn(2, char::is_whitespace);
    let first = parts.next().unwrap_or("");
    let remainder = parts.next().unwrap_or("").trim_start();
    if !first.is_empty()
        && !first.starts_with('"')
        && !first.starts_with('\'')
        && remainder.starts_with("from")
    {
        let after_from = remainder["from".len()..].trim_start();
        let target = strip_quotes(after_from);
        if !target.is_empty() {
            return Some(ImportDirective {
                kind: ImportKind::Qualified(first.to_string()),
                target,
            });
        }
    }
    let target = strip_quotes(rest);
    if target.is_empty() {
        return None;
    }
    Some(ImportDirective {
        kind: ImportKind::BareLocal,
        target,
    })
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
// Where `import from "<name>"` looks for a package, in order:
//   1. liphia_modules/ next to the entry file (installed with `liphia install`)
//   2. liphia_modules/ in the current directory
//   3. $LIPHIA_PACKAGES_PATH (a directory with one folder per package)
//   4. src/packages/ of the Liphia repo, found by walking up from the current
//      directory (lets the repo's own tests and examples use packages
//      without installing them)
fn find_in_package_roots(relative: &Path, source_root: &Path) -> Option<PathBuf> {
    let source_dir = source_root.parent().unwrap_or(Path::new("."));
    let mut candidates: Vec<PathBuf> = vec![source_dir.join("liphia_modules").join(relative)];
    let cwd = std::env::current_dir().unwrap_or_default();
    candidates.push(cwd.join("liphia_modules").join(relative));
    if let Ok(path) = std::env::var("LIPHIA_PACKAGES_PATH") {
        candidates.push(PathBuf::from(path).join(relative));
    }
    for dir in cwd.ancestors() {
        candidates.push(dir.join("src/packages").join(relative));
        candidates.push(dir.join("packages").join(relative));
    }
    candidates.into_iter().find(|c| c.exists())
}

fn resolve_package(package_name: &str, source_root: &Path) -> Result<PathBuf, String> {
    let name = package_name.trim_end_matches(".lph");
    let rel = PathBuf::from(name).join(format!("{}.lph", name));
    find_in_package_roots(&rel, source_root).ok_or_else(|| format!(
        "[liphia] error: package '{}' not found.\n  hint: run 'liphia install {}' to install it,\n        or set LIPHIA_PACKAGES_PATH to a folder containing it.\n  cwd:  {:?}",
        name, name, std::env::current_dir().unwrap_or_default()
    ))
}

fn resolve_subpackage(
    package_name: &str,
    subpackage_name: &str,
    source_root: &Path,
) -> Option<PathBuf> {
    let rel = PathBuf::from(package_name)
        .join(subpackage_name)
        .join(format!("{}.lph", subpackage_name));
    find_in_package_roots(&rel, source_root)
}
fn parse_own_source(path: &Path) -> Result<(Vec<Stmt>, Vec<ImportDirective>), String> {
    let source =
        fs::read_to_string(path).map_err(|e| format!("error: could not read {:?}: {}", path, e))?;
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
    let mut parser = Parser::new(lexer).map_err(|e| format!("\n{}\n", e))?;
    let stmts = parser.parse().map_err(|e| format!("\n{}\n", e))?;
    Ok((stmts, imports))
}
fn stmt_name(stmt: &Stmt) -> Option<String> {
    match stmt {
        Stmt::Fn { name, .. } => Some(name.clone()),
        Stmt::Const { name, .. } => Some(name.clone()),
        Stmt::Enum(def) => Some(def.name.clone()),
        _ => None,
    }
}
fn mangle_stmt_name(stmt: &mut Stmt, alias: &str) {
    match stmt {
        Stmt::Fn { name, .. } => *name = format!("{}::{}", alias, name),
        Stmt::Const { name, .. } => *name = format!("{}::{}", alias, name),
        Stmt::Enum(def) => def.name = format!("{}::{}", alias, def.name),
        _ => {}
    }
}
fn check_no_duplicate_top_level_names(stmts: &[Stmt]) -> Result<(), String> {
    let mut seen: HashMap<String, ()> = HashMap::new();
    for stmt in stmts {
        if let Some(name) = stmt_name(stmt) {
            if seen.contains_key(&name) {
                return Err(format!(
                    "error: '{}' is declared more than once across imported files.\nhint: use a qualified import (`import alias from \"...\"`) to avoid name collisions.",
                    name
                ));
            }
            seen.insert(name, ());
        }
    }
    Ok(())
}
fn resolve_module_calls_in_stmt(
    stmt: &mut Stmt,
    aliases: &HashMap<String, String>,
) -> Result<(), String> {
    match stmt {
        Stmt::VarDecl { value, .. } => resolve_module_calls_in_expr(value, aliases)?,
        Stmt::Var { value, .. } => resolve_module_calls_in_expr(value, aliases)?,
        Stmt::Const { value, .. } => resolve_module_calls_in_expr(value, aliases)?,
        Stmt::Assign { value, .. } => resolve_module_calls_in_expr(value, aliases)?,
        Stmt::AssignIndex { index, value, .. } => {
            resolve_module_calls_in_expr(index, aliases)?;
            resolve_module_calls_in_expr(value, aliases)?;
        }
        Stmt::Print(args) => {
            for a in args {
                resolve_module_calls_in_expr(a, aliases)?;
            }
        }
        Stmt::If {
            condition,
            block,
            branches,
            else_block,
        } => {
            resolve_module_calls_in_expr(condition, aliases)?;
            for s in block {
                resolve_module_calls_in_stmt(s, aliases)?;
            }
            for (cond, blk) in branches {
                resolve_module_calls_in_expr(cond, aliases)?;
                for s in blk {
                    resolve_module_calls_in_stmt(s, aliases)?;
                }
            }
            if let Some(blk) = else_block {
                for s in blk {
                    resolve_module_calls_in_stmt(s, aliases)?;
                }
            }
        }
        Stmt::While { condition, body } => {
            resolve_module_calls_in_expr(condition, aliases)?;
            for s in body {
                resolve_module_calls_in_stmt(s, aliases)?;
            }
        }
        Stmt::For {
            from,
            to,
            step,
            body,
            ..
        } => {
            resolve_module_calls_in_expr(from, aliases)?;
            resolve_module_calls_in_expr(to, aliases)?;
            if let Some(s) = step {
                resolve_module_calls_in_expr(s, aliases)?;
            }
            for s in body {
                resolve_module_calls_in_stmt(s, aliases)?;
            }
        }
        Stmt::Fn { body, .. } => {
            for s in body {
                resolve_module_calls_in_stmt(s, aliases)?;
            }
        }
        Stmt::Try {
            try_block,
            catch_block,
            ..
        } => {
            for s in try_block {
                resolve_module_calls_in_stmt(s, aliases)?;
            }
            for s in catch_block {
                resolve_module_calls_in_stmt(s, aliases)?;
            }
        }
        Stmt::ExprStmt(expr) => resolve_module_calls_in_expr(expr, aliases)?,
        Stmt::Return(expr) => resolve_module_calls_in_expr(expr, aliases)?,
        Stmt::Break | Stmt::Continue | Stmt::Enum(_) => {}
    }
    Ok(())
}
fn resolve_module_calls_in_expr(
    expr: &mut Expr,
    aliases: &HashMap<String, String>,
) -> Result<(), String> {
    match expr {
        Expr::Add(a, b)
        | Expr::Sub(a, b)
        | Expr::Mul(a, b)
        | Expr::Div(a, b)
        | Expr::Eq(a, b)
        | Expr::NotEq(a, b)
        | Expr::Gt(a, b)
        | Expr::Lt(a, b)
        | Expr::Gte(a, b)
        | Expr::Lte(a, b)
        | Expr::And(a, b)
        | Expr::Or(a, b) => {
            resolve_module_calls_in_expr(a, aliases)?;
            resolve_module_calls_in_expr(b, aliases)?;
        }
        Expr::Not(inner) | Expr::Await(inner) => resolve_module_calls_in_expr(inner, aliases)?,
        Expr::List(items) => {
            for item in items {
                resolve_module_calls_in_expr(item, aliases)?;
            }
        }
        Expr::MapLiteral(pairs) => {
            for (k, v) in pairs {
                resolve_module_calls_in_expr(k, aliases)?;
                resolve_module_calls_in_expr(v, aliases)?;
            }
        }
        Expr::Index(a, b) => {
            resolve_module_calls_in_expr(a, aliases)?;
            resolve_module_calls_in_expr(b, aliases)?;
        }
        Expr::FunctionCall { args, .. } | Expr::Spawn { args, .. } => {
            for a in args {
                resolve_module_calls_in_expr(a, aliases)?;
            }
        }
        Expr::ModuleCall { module, name, args } => {
            for a in args.iter_mut() {
                resolve_module_calls_in_expr(a, aliases)?;
            }
            if let Some(prefix) = aliases.get(module) {
                let mangled = format!("{}::{}", prefix, name);
                let taken_args = std::mem::take(args);
                *expr = Expr::FunctionCall {
                    name: mangled,
                    args: taken_args,
                };
            } else {
                return Err(format!(
                    "error: '{}' is not an imported module alias (used as '{}.{}(...)')\nhint: add `import {} from \"...\"` at the top of the file.",
                    module, module, name, module
                ));
            }
        }
        Expr::Int(_)
        | Expr::Float(_)
        | Expr::Str(_)
        | Expr::Bool(_)
        | Expr::Null
        | Expr::Variable(_)
        | Expr::EnumVariant { .. } => {}
    }
    Ok(())
}

// ── Public entry point 1: resolve a project into a merged, flat Vec<Stmt> ──
pub fn resolve_project(
    entry_path: &Path,
    source_root: &Path,
    visited: &mut HashSet<PathBuf>,
) -> Result<Vec<Stmt>, String> {
    let abs = fs::canonicalize(entry_path)
        .map_err(|_| format!("error: file not found: {:?}", entry_path))?;
    if visited.contains(&abs) {
        return Ok(vec![]);
    }
    visited.insert(abs.clone());
    let (mut own_stmts, imports) = parse_own_source(&abs)?;
    let base_dir = abs.parent().unwrap_or(Path::new("."));
    let mut merged: Vec<Stmt> = vec![];
    let mut aliases: HashMap<String, String> = HashMap::new();
    for imp in &imports {
        match &imp.kind {
            ImportKind::BarePackage => {
                let resolved = resolve_package(&imp.target, source_root)?;
                merged.extend(resolve_project(&resolved, source_root, visited)?);
            }
            ImportKind::BareLocal => {
                let resolved = resolve_import_file(base_dir, &imp.target).ok_or_else(|| format!(
                    "error: could not resolve import '{}'\nhint: imports are relative to the current file, or use absolute paths.",
                    imp.target
                ))?;
                merged.extend(resolve_project(&resolved, source_root, visited)?);
            }
            ImportKind::Selective(names) => {
                if let Some(resolved) = resolve_import_file(base_dir, &imp.target) {
                    let module_stmts = resolve_project(&resolved, source_root, visited)?;
                    for stmt in module_stmts {
                        if stmt_name(&stmt).map_or(false, |n| names.contains(&n)) {
                            merged.push(stmt);
                        }
                    }
                } else {
                    let mut remaining: Vec<String> = vec![];
                    for name in names {
                        if let Some(sub_path) =
                            resolve_subpackage(&imp.target, name, source_root)
                        {
                            merged.extend(resolve_project(&sub_path, source_root, visited)?);
                        } else {
                            remaining.push(name.clone());
                        }
                    }
                    if !remaining.is_empty() {
                        let entry = resolve_package(&imp.target, source_root)?;
                        let module_stmts = resolve_project(&entry, source_root, visited)?;
                        for stmt in module_stmts {
                            if stmt_name(&stmt).map_or(false, |n| remaining.contains(&n)) {
                                merged.push(stmt);
                            }
                        }
                    }
                }
            }
            ImportKind::Qualified(alias) => {
                let resolved = match resolve_import_file(base_dir, &imp.target) {
                    Some(p) => p,
                    None => resolve_package(&imp.target, source_root)?,
                };
                let mut module_stmts = resolve_project(&resolved, source_root, visited)?;
                for stmt in module_stmts.iter_mut() {
                    mangle_stmt_name(stmt, alias);
                }
                aliases.insert(alias.clone(), alias.clone());
                merged.extend(module_stmts);
            }
        }
    }
    for stmt in own_stmts.iter_mut() {
        resolve_module_calls_in_stmt(stmt, &aliases)?;
    }
    merged.extend(own_stmts);
    check_no_duplicate_top_level_names(&merged)?;
    Ok(merged)
}

// ── Public entry point 2: type-check + generate bytecode ──────────────────
pub fn compile(stmts: Vec<Stmt>) -> Result<Vec<Opcode>, String> {
    compile_with_externals(stmts, &[])
}

pub fn compile_with_externals(
    stmts: Vec<Stmt>,
    externals: &[(&str, Vec<Type>, Type)],
) -> Result<Vec<Opcode>, String> {
    let mut checker = TypeChecker::new();
    for (name, params, ret) in externals {
        checker.declare_fn_external(name, params.clone(), ret.clone());
    }
    checker.check(&stmts).map_err(|e| format!("\n{}\n", e))?;
    let program = generate_bytecode(stmts).map_err(|e| format!("\n{}\n", e))?;
    Ok(program.instructions)
}
