// liphia_engine/crates/liphia_pipeline/src/lib.rs
//
// Shared compilation pipeline: import resolution + type checking + bytecode
// generation. Used by both liphia_cli (runs to completion via vm.run()) and
// liphia_cli_gui (runs incrementally via VmSession::tick(), driven by the
// window's own event loop). Extracted from liphia_cli's main.rs — behavior
// unchanged, except errors are now returned as Result instead of exiting the
// process directly, so GUI hosts (which have no terminal to print to and
// must not just vanish on a compile error) can display them instead.
//
// Package lookup has two modes. With a `PackageRoots` scope (a file inside
// a project or workspace), `import from "<name>"` resolves only to the
// packages that project declares, as computed by liphia_manifest from
// liphia.toml; anything else is an error naming who declares dependencies.
// Without a scope (a loose file, the conformance suite), the older search
// through liphia_modules/, LIPHIA_PACKAGES_PATH and the repo's own
// src/packages/ applies. This crate never reads liphia.toml itself.
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

// Where `import from "<name>"` may resolve. `packages` maps a package
// name to its entry file; `scope` describes who declares the dependencies
// (e.g. "member 'app' (app/liphia.toml)"). A scope turns off the legacy
// search, so an undeclared package is an error instead of a lucky find.
// `visibility` narrows it per folder: a file under one of those folders may
// import only the names listed for the deepest folder that contains it, so
// a member cannot reach a package that only one of its dependencies
// declares.
#[derive(Debug, Clone, Default)]
pub struct PackageRoots {
    pub packages: HashMap<String, PathBuf>,
    pub scope: Option<String>,
    pub visibility: Vec<Visibility>,
}

#[derive(Debug, Clone)]
pub struct Visibility {
    pub dir: PathBuf,
    pub label: String,
    pub allowed: HashSet<String>,
}

impl PackageRoots {
    pub fn scoped(scope: String, packages: impl IntoIterator<Item = (String, PathBuf)>) -> Self {
        PackageRoots {
            packages: packages.into_iter().collect(),
            scope: Some(scope),
            visibility: vec![],
        }
    }

    pub fn with_visibility(mut self, visibility: Vec<Visibility>) -> Self {
        self.visibility = visibility;
        self
    }

    // The deepest visibility folder containing `file`.
    fn visibility_for(&self, file: &Path) -> Option<&Visibility> {
        let file = plain_path(file);
        self.visibility
            .iter()
            .filter(|v| file.starts_with(plain_path(&v.dir)))
            .max_by_key(|v| v.dir.components().count())
    }
}

// Drops the \\?\ prefix Windows' canonicalize adds, so paths from the
// manifest (already stripped) and from fs::canonicalize compare equal.
fn plain_path(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC\\") => PathBuf::from(rest),
        _ => path.to_path_buf(),
    }
}

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

fn resolve_package(
    package_name: &str,
    source_root: &Path,
    importer: &Path,
    roots: &PackageRoots,
) -> Result<PathBuf, String> {
    let name = package_name.trim_end_matches(".lph");
    if let Some(vis) = roots.visibility_for(importer) {
        if !vis.allowed.contains(name) && roots.packages.contains_key(name) {
            let mut declared: Vec<&str> = vis.allowed.iter().map(|s| s.as_str()).collect();
            declared.sort();
            return Err(format!(
                "[liphia] error: {} imports '{}' but does not declare it.\n  hint: add '{}' to its liphia.toml; a dependency's own dependencies are not visible.\n  declared: {}",
                vis.label, name, name, declared.join(", ")
            ));
        }
    }
    if let Some(entry) = roots.packages.get(name) {
        return Ok(entry.clone());
    }
    if let Some(scope) = &roots.scope {
        let mut known: Vec<&str> = roots.packages.keys().map(|k| k.as_str()).collect();
        known.sort();
        let known = if known.is_empty() { "none".to_string() } else { known.join(", ") };
        return Err(format!(
            "[liphia] error: package '{}' is not a dependency of {}.\n  hint: add it to that liphia.toml and run 'liphia install'.\n  available: {}",
            name, scope, known
        ));
    }
    let rel = PathBuf::from(name).join(format!("{}.lph", name));
    find_in_package_roots(&rel, source_root).ok_or_else(|| format!(
        "[liphia] error: package '{}' not found.\n  hint: run 'liphia install {}' to install it,\n        or set LIPHIA_PACKAGES_PATH to a folder containing it.\n  cwd:  {:?}",
        name, name, std::env::current_dir().unwrap_or_default()
    ))
}

// A subpackage lives in <package folder>/<sub>/<sub>.lph.
fn resolve_subpackage(
    package_name: &str,
    subpackage_name: &str,
    source_root: &Path,
    roots: &PackageRoots,
) -> Option<PathBuf> {
    if let Some(entry) = roots.packages.get(package_name) {
        let path = entry
            .parent()?
            .join(subpackage_name)
            .join(format!("{}.lph", subpackage_name));
        return path.exists().then_some(path);
    }
    if roots.scope.is_some() {
        return None;
    }
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
// ── Module renaming (qualified and selective imports) ────────────────────────
//
// A qualified import (`import m from "x"`) renames every top-level fn,
// const and enum of the module to `m::name`; a selective import
// (`import { a } from "x"`) keeps the selected names and renames the rest
// to a hidden prefix, so they exist but cannot be called from outside.
// Renaming the definitions alone is not enough: every reference inside the
// module (calls, spawns, const reads, enum variants and enum types) must
// follow, or the module's own code calls names that no longer exist.

// Old name -> new name, per kind of declaration.
#[derive(Default)]
struct Renames {
    fns: HashMap<String, String>,
    consts: HashMap<String, String>,
    enums: HashMap<String, String>,
}

impl Renames {
    // `rename(name)` gives the new name for each top-level declaration.
    fn build(stmts: &[Stmt], rename: impl Fn(&str) -> String) -> Renames {
        let mut r = Renames::default();
        for stmt in stmts {
            match stmt {
                Stmt::Fn { name, .. } => {
                    r.fns.insert(name.clone(), rename(name));
                }
                Stmt::Const { name, .. } => {
                    r.consts.insert(name.clone(), rename(name));
                }
                Stmt::Enum(def) => {
                    r.enums.insert(def.name.clone(), rename(&def.name));
                }
                _ => {}
            }
        }
        r
    }
}

// Unique prefix for the private names of one selective import.
fn hidden_prefix() -> String {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!("__import{}", NEXT.fetch_add(1, Ordering::Relaxed))
}

fn rename_module(stmts: &mut [Stmt], r: &Renames) {
    let none = HashSet::new();
    for stmt in stmts.iter_mut() {
        rename_stmt(stmt, r, &none);
    }
}

// Names a function body declares itself (params, var, for, catch); inside
// that body they shadow a module const of the same name.
fn collect_locals(stmts: &[Stmt], out: &mut HashSet<String>) {
    for stmt in stmts {
        match stmt {
            Stmt::VarDecl { name, .. } | Stmt::Var { name, .. } => {
                out.insert(name.clone());
            }
            Stmt::For { var, body, .. } => {
                out.insert(var.clone());
                collect_locals(body, out);
            }
            Stmt::If { block, branches, else_block, .. } => {
                collect_locals(block, out);
                for (_, b) in branches {
                    collect_locals(b, out);
                }
                if let Some(b) = else_block {
                    collect_locals(b, out);
                }
            }
            Stmt::While { body, .. } => collect_locals(body, out),
            Stmt::Try { try_block, catch_var, catch_block } => {
                out.insert(catch_var.clone());
                collect_locals(try_block, out);
                collect_locals(catch_block, out);
            }
            _ => {}
        }
    }
}

fn rename_type(ty: &mut Type, r: &Renames) {
    match ty {
        Type::Named(n) => {
            if let Some(new) = r.enums.get(n) {
                *n = new.clone();
            }
        }
        Type::Optional(inner) => rename_type(inner, r),
        _ => {}
    }
}

fn rename_stmts(stmts: &mut [Stmt], r: &Renames, shadowed: &HashSet<String>) {
    for stmt in stmts.iter_mut() {
        rename_stmt(stmt, r, shadowed);
    }
}

fn rename_stmt(stmt: &mut Stmt, r: &Renames, shadowed: &HashSet<String>) {
    match stmt {
        Stmt::Fn { name, params, return_type, body, .. } => {
            if let Some(new) = r.fns.get(name) {
                *name = new.clone();
            }
            let mut locals = shadowed.clone();
            for p in params.iter_mut() {
                rename_type(&mut p.ty, r);
                locals.insert(p.name.clone());
            }
            rename_type(return_type, r);
            collect_locals(body, &mut locals);
            rename_stmts(body, r, &locals);
        }
        Stmt::Const { name, value } => {
            if let Some(new) = r.consts.get(name) {
                *name = new.clone();
            }
            rename_expr(value, r, shadowed);
        }
        Stmt::Enum(def) => {
            if let Some(new) = r.enums.get(&def.name) {
                def.name = new.clone();
            }
        }
        Stmt::VarDecl { ty, value, .. } => {
            rename_type(ty, r);
            rename_expr(value, r, shadowed);
        }
        Stmt::Var { value, .. } | Stmt::Assign { value, .. } => rename_expr(value, r, shadowed),
        Stmt::AssignIndex { index, value, .. } => {
            rename_expr(index, r, shadowed);
            rename_expr(value, r, shadowed);
        }
        Stmt::Print(args) => {
            for a in args {
                rename_expr(a, r, shadowed);
            }
        }
        Stmt::If { condition, block, branches, else_block } => {
            rename_expr(condition, r, shadowed);
            rename_stmts(block, r, shadowed);
            for (cond, blk) in branches {
                rename_expr(cond, r, shadowed);
                rename_stmts(blk, r, shadowed);
            }
            if let Some(blk) = else_block {
                rename_stmts(blk, r, shadowed);
            }
        }
        Stmt::While { condition, body } => {
            rename_expr(condition, r, shadowed);
            rename_stmts(body, r, shadowed);
        }
        Stmt::For { from, to, step, body, .. } => {
            rename_expr(from, r, shadowed);
            rename_expr(to, r, shadowed);
            if let Some(s) = step {
                rename_expr(s, r, shadowed);
            }
            rename_stmts(body, r, shadowed);
        }
        Stmt::Try { try_block, catch_block, .. } => {
            rename_stmts(try_block, r, shadowed);
            rename_stmts(catch_block, r, shadowed);
        }
        Stmt::ExprStmt(e) | Stmt::Return(e) => rename_expr(e, r, shadowed),
        Stmt::Break | Stmt::Continue => {}
    }
}

fn rename_expr(expr: &mut Expr, r: &Renames, shadowed: &HashSet<String>) {
    match expr {
        Expr::Variable(n) => {
            if !shadowed.contains(n.as_str()) {
                if let Some(new) = r.consts.get(n) {
                    *n = new.clone();
                }
            }
        }
        Expr::FunctionCall { name, args } | Expr::Spawn { name, args } => {
            if let Some(new) = r.fns.get(name) {
                *name = new.clone();
            }
            for a in args {
                rename_expr(a, r, shadowed);
            }
        }
        Expr::EnumVariant { enum_name, .. } => {
            if let Some(new) = r.enums.get(enum_name) {
                *enum_name = new.clone();
            }
        }
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
        | Expr::Or(a, b)
        | Expr::Index(a, b) => {
            rename_expr(a, r, shadowed);
            rename_expr(b, r, shadowed);
        }
        Expr::Not(inner) | Expr::Await(inner) => rename_expr(inner, r, shadowed),
        Expr::List(items) => {
            for item in items {
                rename_expr(item, r, shadowed);
            }
        }
        Expr::MapLiteral(pairs) => {
            for (k, v) in pairs {
                rename_expr(k, r, shadowed);
                rename_expr(v, r, shadowed);
            }
        }
        // Resolved into FunctionCall before a module is ever renamed.
        Expr::ModuleCall { args, .. } => {
            for a in args {
                rename_expr(a, r, shadowed);
            }
        }
        Expr::Int(_) | Expr::Float(_) | Expr::Str(_) | Expr::Bool(_) | Expr::Null => {}
    }
}

// Declarations a selective import keeps: fn, const and enum (renamed as
// needed) plus the module's global variables its functions may read. Other
// top-level statements (prints, calls, spawns) do not run on a selective
// import, as before.
fn is_module_declaration(stmt: &Stmt) -> bool {
    matches!(
        stmt,
        Stmt::Fn { .. } | Stmt::Const { .. } | Stmt::Enum(_) | Stmt::VarDecl { .. } | Stmt::Var { .. }
    )
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
//
// Without a package scope: the legacy package search (see PackageRoots).
pub fn resolve_project(
    entry_path: &Path,
    source_root: &Path,
    visited: &mut HashSet<PathBuf>,
) -> Result<Vec<Stmt>, String> {
    resolve_project_with(entry_path, source_root, visited, &PackageRoots::default())
}

// With the packages a project or workspace member declares.
pub fn resolve_project_with(
    entry_path: &Path,
    source_root: &Path,
    visited: &mut HashSet<PathBuf>,
    roots: &PackageRoots,
) -> Result<Vec<Stmt>, String> {
    let mut stack = vec![];
    resolve_file(entry_path, source_root, visited, roots, &mut stack)
}

// Resolves a module in isolation for a qualified or selective import: it
// gets its own copy of every file it imports, so its code never depends on
// what the importer happened to import first. Files on the current import
// chain are pre-visited so an import cycle still terminates.
fn resolve_isolated(
    path: &Path,
    source_root: &Path,
    roots: &PackageRoots,
    stack: &mut Vec<PathBuf>,
) -> Result<Vec<Stmt>, String> {
    let mut visited: HashSet<PathBuf> = stack.iter().cloned().collect();
    resolve_file(path, source_root, &mut visited, roots, stack)
}

fn resolve_file(
    entry_path: &Path,
    source_root: &Path,
    visited: &mut HashSet<PathBuf>,
    roots: &PackageRoots,
    stack: &mut Vec<PathBuf>,
) -> Result<Vec<Stmt>, String> {
    let abs = fs::canonicalize(entry_path)
        .map_err(|_| format!("error: file not found: {:?}", entry_path))?;
    if visited.contains(&abs) {
        return Ok(vec![]);
    }
    visited.insert(abs.clone());
    stack.push(abs.clone());
    let result = resolve_file_body(&abs, source_root, visited, roots, stack);
    stack.pop();
    result
}

fn resolve_file_body(
    abs: &Path,
    source_root: &Path,
    visited: &mut HashSet<PathBuf>,
    roots: &PackageRoots,
    stack: &mut Vec<PathBuf>,
) -> Result<Vec<Stmt>, String> {
    let (mut own_stmts, imports) = parse_own_source(abs)?;
    let base_dir = abs.parent().unwrap_or(Path::new("."));
    let mut merged: Vec<Stmt> = vec![];
    let mut aliases: HashMap<String, String> = HashMap::new();
    for imp in &imports {
        match &imp.kind {
            ImportKind::BarePackage => {
                let resolved = resolve_package(&imp.target, source_root, abs, roots)?;
                merged.extend(resolve_file(&resolved, source_root, visited, roots, stack)?);
            }
            ImportKind::BareLocal => {
                let resolved = resolve_import_file(base_dir, &imp.target).ok_or_else(|| format!(
                    "error: could not resolve import '{}'\nhint: imports are relative to the current file, or use absolute paths.",
                    imp.target
                ))?;
                merged.extend(resolve_file(&resolved, source_root, visited, roots, stack)?);
            }
            ImportKind::Selective(names) => {
                if let Some(resolved) = resolve_import_file(base_dir, &imp.target) {
                    merged.extend(select_from_module(&resolved, names, source_root, visited, roots, stack)?);
                } else {
                    let mut remaining: Vec<String> = vec![];
                    for name in names {
                        if let Some(sub_path) =
                            resolve_subpackage(&imp.target, name, source_root, roots)
                        {
                            merged.extend(resolve_file(&sub_path, source_root, visited, roots, stack)?);
                        } else {
                            remaining.push(name.clone());
                        }
                    }
                    if !remaining.is_empty() {
                        let entry = resolve_package(&imp.target, source_root, abs, roots)?;
                        merged.extend(select_from_module(&entry, &remaining, source_root, visited, roots, stack)?);
                    }
                }
            }
            ImportKind::Qualified(alias) => {
                let resolved = match resolve_import_file(base_dir, &imp.target) {
                    Some(p) => p,
                    None => resolve_package(&imp.target, source_root, abs, roots)?,
                };
                let mut module_stmts = resolve_isolated(&resolved, source_root, roots, stack)?;
                let renames = Renames::build(&module_stmts, |n| format!("{}::{}", alias, n));
                rename_module(&mut module_stmts, &renames);
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

// `import { a, b } from "<file>"`: a and b keep their names; everything
// else the module declares is renamed to a hidden prefix, so a and b can
// still call it but the importer cannot. Selecting the same name from the
// same file twice in one program (app.lph and routes.lph both importing
// { reply }) yields it once: the pair is recorded in `visited`.
fn select_from_module(
    path: &Path,
    names: &[String],
    source_root: &Path,
    visited: &mut HashSet<PathBuf>,
    roots: &PackageRoots,
    stack: &mut Vec<PathBuf>,
) -> Result<Vec<Stmt>, String> {
    let abs = fs::canonicalize(path).map_err(|_| format!("error: file not found: {:?}", path))?;
    let key = |name: &str| PathBuf::from(format!("{}#select:{}", abs.display(), name));
    let names: Vec<String> = names
        .iter()
        .filter(|n| !visited.contains(&key(n)))
        .cloned()
        .collect();
    if names.is_empty() {
        return Ok(vec![]);
    }
    let mut module_stmts: Vec<Stmt> = resolve_isolated(path, source_root, roots, stack)?
        .into_iter()
        .filter(is_module_declaration)
        .collect();
    for name in &names {
        if !module_stmts.iter().any(|s| stmt_name(s).as_deref() == Some(name.as_str())) {
            return Err(format!(
                "error: '{}' is not declared in {}\nhint: a selective import lists fn, const or enum names the module declares.",
                name,
                path.display()
            ));
        }
    }
    for name in &names {
        visited.insert(key(name));
    }
    let prefix = hidden_prefix();
    let renames = Renames::build(&module_stmts, |n| {
        if names.iter().any(|x| x == n) {
            n.to_string()
        } else {
            format!("{}::{}", prefix, n)
        }
    });
    rename_module(&mut module_stmts, &renames);
    Ok(module_stmts)
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
