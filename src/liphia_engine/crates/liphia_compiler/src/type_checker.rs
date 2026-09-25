// liphia_compiler/src/type_checker.rs
use std::collections::{HashMap, HashSet};
use crate::ast::{EnumDef, Expr, Stmt, Type};
use crate::error::{ErrorKind, LiphiaError, LiphiaResult};

#[derive(Debug, Clone)]
struct Symbol {
    ty:       Type,
    is_const: bool,
}

struct Scope {
    vars:  HashMap<String, Symbol>,
    enums: HashMap<String, EnumDef>,
    fns:   HashMap<String, (Vec<Type>, Type)>,
}

impl Scope {
    fn new() -> Self {
        Self { vars: HashMap::new(), enums: HashMap::new(), fns: HashMap::new() }
    }
}

pub struct TypeChecker {
    scopes:              Vec<Scope>,
    current_return_type: Option<Type>,
    in_async_fn:         bool,
    async_fn_names:      HashSet<String>,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut tc = Self {
            scopes:              vec![],
            current_return_type: None,
            in_async_fn:         false,
            async_fn_names:      HashSet::new(),
        };
        tc.push_scope();

        let any = || Type::Named("any".into());

        // ── builtins ──────────────────────────────────────────────────────
        tc.declare_fn("input", vec![Type::Str], Type::Str);

        // ── Core natives ──────────────────────────────────────────────────
        // Signatures of everything registered by liphia_core_native. Keep in
        // sync with that crate and with docs/core.md. Package natives are
        // not declared here: calls to them type-check as "unknown", and the
        // VM resolves them at runtime once the package is loaded.

        // strings and conversions
        tc.declare_fn("len",         vec![any()],                           Type::Int);
        tc.declare_fn("to_int",      vec![any()],                           Type::Int);
        tc.declare_fn("to_float",    vec![any()],                           Type::Float);
        tc.declare_fn("to_str",      vec![any()],                           Type::Str);
        tc.declare_fn("trim",        vec![Type::Str],                       Type::Str);
        tc.declare_fn("upper",       vec![Type::Str],                       Type::Str);
        tc.declare_fn("lower",       vec![Type::Str],                       Type::Str);
        tc.declare_fn("contains",    vec![Type::Str, Type::Str],            Type::Bool);
        tc.declare_fn("starts_with", vec![Type::Str, Type::Str],            Type::Bool);
        tc.declare_fn("ends_with",   vec![Type::Str, Type::Str],            Type::Bool);
        tc.declare_fn("replace",     vec![Type::Str, Type::Str, Type::Str], Type::Str);
        tc.declare_fn("split",       vec![Type::Str, Type::Str],            Type::List);

        // lists and maps
        tc.declare_fn("append",     vec![Type::List, any()], Type::Void);
        tc.declare_fn("pop",        vec![Type::List],        any());
        tc.declare_fn("keys",       vec![Type::List],        Type::List);
        tc.declare_fn("map_keys",   vec![Type::Map],         Type::List);
        tc.declare_fn("map_values", vec![Type::Map],         Type::List);
        tc.declare_fn("map_has",    vec![Type::Map, any()],  Type::Bool);
        tc.declare_fn("map_remove", vec![Type::Map, any()],  Type::Void);

        // math
        tc.declare_fn("sqrt",       vec![any()],               Type::Float);
        tc.declare_fn("pow",        vec![any(), any()],        Type::Float);
        tc.declare_fn("exp",        vec![any()],               Type::Float);
        tc.declare_fn("log",        vec![any()],               Type::Float);
        tc.declare_fn("log10",      vec![any()],               Type::Float);
        tc.declare_fn("log2",       vec![any()],               Type::Float);
        tc.declare_fn("log_base",   vec![any(), any()],        Type::Float);
        tc.declare_fn("sin",        vec![any()],               Type::Float);
        tc.declare_fn("cos",        vec![any()],               Type::Float);
        tc.declare_fn("tan",        vec![any()],               Type::Float);
        tc.declare_fn("asin",       vec![any()],               Type::Float);
        tc.declare_fn("acos",       vec![any()],               Type::Float);
        tc.declare_fn("atan",       vec![any()],               Type::Float);
        tc.declare_fn("atan2",      vec![any(), any()],        Type::Float);
        tc.declare_fn("sinh",       vec![any()],               Type::Float);
        tc.declare_fn("cosh",       vec![any()],               Type::Float);
        tc.declare_fn("tanh",       vec![any()],               Type::Float);
        tc.declare_fn("hypot",      vec![any(), any()],        Type::Float);
        tc.declare_fn("deg_to_rad", vec![any()],               Type::Float);
        tc.declare_fn("rad_to_deg", vec![any()],               Type::Float);
        tc.declare_fn("pi",         vec![],                    Type::Float);
        tc.declare_fn("e",          vec![],                    Type::Float);
        tc.declare_fn("inf",        vec![],                    Type::Float);
        tc.declare_fn("nan",        vec![],                    Type::Float);
        tc.declare_fn("abs",        vec![any()],               any());
        tc.declare_fn("floor",      vec![any()],               Type::Int);
        tc.declare_fn("ceil",       vec![any()],               Type::Int);
        tc.declare_fn("round",      vec![any()],               Type::Int);
        tc.declare_fn("min",        vec![any(), any()],        any());
        tc.declare_fn("max",        vec![any(), any()],        any());
        tc.declare_fn("clamp",      vec![any(), any(), any()], any());
        tc.declare_fn("sign",       vec![any()],               Type::Int);
        tc.declare_fn("factorial",  vec![Type::Int],           Type::Int);
        tc.declare_fn("gcd",        vec![Type::Int, Type::Int], Type::Int);
        tc.declare_fn("lcm",        vec![Type::Int, Type::Int], Type::Int);
        tc.declare_fn("is_nan",     vec![any()],               Type::Bool);
        tc.declare_fn("is_inf",     vec![any()],               Type::Bool);

        // random
        tc.declare_fn("seed",         vec![Type::Int],               Type::Void);
        tc.declare_fn("rand_int",     vec![Type::Int, Type::Int],    Type::Int);
        tc.declare_fn("rand_uniform", vec![Type::Int, any(), any()], Type::List);
        tc.declare_fn("rand_normal",  vec![Type::Int, any(), any()], Type::List);
        tc.declare_fn("shuffle",      vec![Type::List],              Type::List);

        // aggregation (int or float, following the list's element type)
        tc.declare_fn("sum",      vec![Type::List], any());
        tc.declare_fn("mean",     vec![Type::List], Type::Float);
        tc.declare_fn("min_list", vec![Type::List], any());
        tc.declare_fn("max_list", vec![Type::List], any());

        // json
        tc.declare_fn("json_encode", vec![any()],     Type::Str);
        tc.declare_fn("json_decode", vec![Type::Str], any());

        // fs
        tc.declare_fn("read_file",        vec![Type::Str],            Type::Str);
        tc.declare_fn("write_file",       vec![Type::Str, Type::Str], Type::Bool);
        tc.declare_fn("append_file",      vec![Type::Str, Type::Str], Type::Bool);
        tc.declare_fn("file_exists",      vec![Type::Str],            Type::Bool);
        tc.declare_fn("read_json",        vec![Type::Str],            any());
        tc.declare_fn("write_json",       vec![Type::Str, any()],     Type::Bool);
        tc.declare_fn("append_json_line", vec![Type::Str, any()],     Type::Bool);

        // net
        tc.declare_fn("tcp_connect",   vec![Type::Str, Type::Int],            Type::Int);
        tc.declare_fn("tcp_send",      vec![Type::Int, Type::Str],            Type::Bool);
        tc.declare_fn("tcp_recv",      vec![Type::Int],                       Type::Str);
        tc.declare_fn("tcp_recv_all",  vec![Type::Int],                       Type::Str);
        tc.declare_fn("tcp_close",     vec![Type::Int],                       Type::Bool);
        tc.declare_fn("udp_send",      vec![Type::Str, Type::Int, Type::Str], Type::Bool);
        tc.declare_fn("tcp_send_json", vec![Type::Int, any()],                Type::Bool);
        tc.declare_fn("tcp_recv_json", vec![Type::Int],                       any());

        // http server
        tc.declare_fn("http_listen",       vec![Type::Int],            Type::Bool);
        tc.declare_fn("http_accept",       vec![],                     Type::Bool);
        tc.declare_fn("http_method",       vec![],                     Type::Str);
        tc.declare_fn("http_path",         vec![],                     Type::Str);
        tc.declare_fn("http_query",        vec![],                     Type::Str);
        tc.declare_fn("http_body",         vec![],                     Type::Str);
        tc.declare_fn("http_header",       vec![Type::Str],            Type::Str);
        tc.declare_fn("http_respond",      vec![Type::Int, Type::Str], Type::Bool);
        tc.declare_fn("http_respond_json", vec![Type::Int, Type::Str], Type::Bool);

        // http client
        tc.declare_fn("http_get",    vec![Type::Str],            Type::Str);
        tc.declare_fn("http_post",   vec![Type::Str, Type::Str], Type::Str);
        tc.declare_fn("http_put",    vec![Type::Str, Type::Str], Type::Str);
        tc.declare_fn("http_patch",  vec![Type::Str, Type::Str], Type::Str);
        tc.declare_fn("http_delete", vec![Type::Str],            Type::Str);
        tc.declare_fn("http_status", vec![],                     Type::Int);

        // ws
        tc.declare_fn("ws_listen",         vec![Type::Int],            Type::Bool);
        tc.declare_fn("ws_accept",         vec![],                     Type::Int);
        tc.declare_fn("ws_clients",        vec![],                     Type::List);
        tc.declare_fn("ws_send",           vec![Type::Int, Type::Str], Type::Bool);
        tc.declare_fn("ws_recv",           vec![Type::Int],            Type::Str);
        tc.declare_fn("ws_broadcast",      vec![Type::Str],            Type::Bool);
        tc.declare_fn("ws_close",          vec![Type::Int],            Type::Bool);
        tc.declare_fn("ws_send_json",      vec![Type::Int, any()],     Type::Bool);
        tc.declare_fn("ws_broadcast_json", vec![any()],                Type::Bool);

        tc
    }

    pub fn declare_var_external(&mut self, name: &str, ty: Type) {
        self.declare(name, ty, false);
    }

    pub fn declare_fn_external(&mut self, name: &str, params: Vec<Type>, ret: Type) {
        self.declare_fn(name, params, ret);
    }
    
    fn push_scope(&mut self) { self.scopes.push(Scope::new()); }
    fn pop_scope(&mut self)  { self.scopes.pop(); }

    fn declare(&mut self, name: &str, ty: Type, is_const: bool) {
        self.scopes.last_mut().unwrap().vars
            .insert(name.to_string(), Symbol { ty, is_const });
    }

    fn lookup(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(sym) = scope.vars.get(name) { return Some(sym); }
        }
        None
    }

    fn lookup_enum(&self, name: &str) -> Option<&EnumDef> {
        for scope in self.scopes.iter().rev() {
            if let Some(e) = scope.enums.get(name) { return Some(e); }
        }
        None
    }

    fn lookup_fn(&self, name: &str) -> Option<&(Vec<Type>, Type)> {
        for scope in self.scopes.iter().rev() {
            if let Some(f) = scope.fns.get(name) { return Some(f); }
        }
        None
    }

    fn declare_fn(&mut self, name: &str, param_types: Vec<Type>, return_type: Type) {
        self.scopes.last_mut().unwrap().fns
            .insert(name.to_string(), (param_types, return_type));
    }

    fn declare_enum(&mut self, def: EnumDef) {
        self.scopes.last_mut().unwrap().enums.insert(def.name.clone(), def);
    }

    // ── Type inference ────────────────────────────────────────────────────
    fn infer(&self, expr: &Expr) -> Type {
        match expr {
            Expr::Int(_)    => Type::Int,
            Expr::Float(_)  => Type::Float,
            Expr::Str(_)    => Type::Str,
            Expr::Bool(_)   => Type::Bool,
            Expr::Null      => Type::Null,
            Expr::List(_)   => Type::List,
            Expr::MapLiteral(_) => Type::Map,
            Expr::Variable(name) => {
                self.lookup(name).map(|s| s.ty.clone())
                    .unwrap_or(Type::Named("unknown".into()))
            }
            Expr::Add(a, _) | Expr::Sub(a, _) |
            Expr::Mul(a, _) | Expr::Div(a, _) => self.infer(a),
            Expr::Eq(..)  | Expr::NotEq(..) |
            Expr::Gt(..)  | Expr::Lt(..)    |
            Expr::Gte(..) | Expr::Lte(..)   |
            Expr::And(..) | Expr::Or(..)    | Expr::Not(..) => Type::Bool,
            Expr::Index(list, _) => {
                    match self.infer(list) {
                Type::List => Type::Named("any".into()),
                Type::Str  => Type::Str,
                    other      => other,
                    }
            }
            Expr::EnumVariant { enum_name, .. } => Type::Named(enum_name.clone()),
            Expr::FunctionCall { name, .. } => {
                self.lookup_fn(name).map(|(_, r)| r.clone())
                    .unwrap_or(Type::Named("unknown".into()))
            }
            Expr::ModuleCall { .. } => Type::Named("unknown".into()),
            Expr::Await(inner) => self.infer(inner),
            Expr::Spawn { .. } => Type::Null,
        }
    }

    // ── Public entry ──────────────────────────────────────────────────────
    pub fn check(&mut self, stmts: &[Stmt]) -> LiphiaResult<()> {
        for stmt in stmts {
            match stmt {
                Stmt::Fn { name, params, return_type, is_async, .. } => {
                    let ptypes: Vec<Type> = params.iter().map(|p| p.ty.clone()).collect();
                    self.declare_fn(name, ptypes, return_type.clone());
                    if *is_async { self.async_fn_names.insert(name.clone()); }
                }
                Stmt::Enum(def) => { self.declare_enum(def.clone()); }
                _ => {}
            }
        }
        for stmt in stmts { self.check_stmt(stmt)?; }
        Ok(())
    }

    fn check_stmt(&mut self, stmt: &Stmt) -> LiphiaResult<()> {
        match stmt {
            Stmt::VarDecl { name, ty, value } => {
                self.check_expr(value)?;
                let vty = self.infer(value);
                if !ty.is_compatible(&vty) {
                    return Err(LiphiaError::new(
                        ErrorKind::TypeError,
                        format!("variable '{}' declared as {:?} but assigned {:?}", name, ty, vty),
                    ));
                }
                self.declare(name, ty.clone(), false);
            }
            Stmt::Var { name, value } => {
                self.check_expr(value)?;
                let ty = self.infer(value);
                self.declare(name, ty, false);
            }
            Stmt::Const { name, value } => {
                self.check_expr(value)?;
                let ty = self.infer(value);
                self.declare(name, ty, true);
            }
            Stmt::Assign { name, value } => {
                self.check_expr(value)?;
                if let Some(sym) = self.lookup(name).cloned() {
                    if sym.is_const {
                      return Err(LiphiaError::new(
                          ErrorKind::TypeError,
                            format!("cannot reassign const '{}'", name),
                       ));
                    }
        
               let new_ty = self.infer(value);
               if !sym.ty.is_compatible(&new_ty) {
                        return Err(LiphiaError::new(
                            ErrorKind::TypeError,
                           format!(
                               "cannot assign {:?} to '{}' of type {:?}",
                                new_ty, name, sym.ty
                            ),
                      ));
                    }
                }
            }
            Stmt::AssignIndex { index, value, .. } => {
                self.check_expr(index)?;
                self.check_expr(value)?;
            }
            Stmt::Print(args) => {
                for arg in args { self.check_expr(arg)?; }
            }
            Stmt::If { condition, block, branches, else_block } => {
                self.check_expr(condition)?;
                let cty = self.infer(condition);
                if cty != Type::Bool && !matches!(cty, Type::Named(_)) {
                    return Err(LiphiaError::new(
                        ErrorKind::TypeError,
                        format!("if condition must be bool, found {:?}", cty),
                    ));
                }
                self.push_scope();
                for s in block { self.check_stmt(s)?; }
                self.pop_scope();
                for (cond, blk) in branches {
                    self.check_expr(cond)?;
                    self.push_scope();
                    for s in blk { self.check_stmt(s)?; }
                    self.pop_scope();
                }
                if let Some(blk) = else_block {
                    self.push_scope();
                    for s in blk { self.check_stmt(s)?; }
                    self.pop_scope();
                }
            }
            Stmt::While { condition, body } => {
                self.check_expr(condition)?;
                self.push_scope();
                for s in body { self.check_stmt(s)?; }
                self.pop_scope();
            }
            Stmt::For { var, from, to, step, body } => {
                self.check_expr(from)?;
                self.check_expr(to)?;
                if let Some(s) = step { self.check_expr(s)?; }
                self.push_scope();
                self.declare(var, Type::Int, false);
                for s in body { self.check_stmt(s)?; }
                self.pop_scope();
            }
            Stmt::Fn { name, params, return_type, body, is_async } => {
                if *is_async { self.async_fn_names.insert(name.clone()); }
                let prev_return = self.current_return_type.replace(return_type.clone());
                let prev_async  = self.in_async_fn;
                self.in_async_fn = *is_async;
                self.push_scope();
                for p in params { self.declare(&p.name, p.ty.clone(), false); }
                for s in body   { self.check_stmt(s)?; }
                self.pop_scope();
                self.in_async_fn         = prev_async;
                self.current_return_type = prev_return;
            }
            Stmt::Return(expr) => {
                self.check_expr(expr)?;
                if let Some(expected) = &self.current_return_type.clone() {
                    let actual = self.infer(expr);
                    if expected != &Type::Void && !expected.is_compatible(&actual) {
                        return Err(LiphiaError::new(
                            ErrorKind::TypeError,
                            format!("function returns {:?} but got {:?}", expected, actual),
                        ));
                    }
                }
            }
            Stmt::Try { try_block, catch_var, catch_block } => {
                self.push_scope();
                for s in try_block { self.check_stmt(s)?; }
                self.pop_scope();

                self.push_scope();
                self.declare(catch_var, Type::Str, false);
                for s in catch_block { self.check_stmt(s)?; }
                self.pop_scope();
            }
            Stmt::Enum(def)      => { self.declare_enum(def.clone()); }
            Stmt::ExprStmt(expr) => { self.check_expr(expr)?; }
            Stmt::Break | Stmt::Continue => {}
        }
        Ok(())
    }

    fn check_expr(&self, expr: &Expr) -> LiphiaResult<()> {
        match expr {
            Expr::EnumVariant { enum_name, variant } => {
                if let Some(def) = self.lookup_enum(enum_name) {
                    if !def.variants.contains(variant) {
                        return Err(LiphiaError::new(
                            ErrorKind::TypeError,
                            format!("enum '{}' has no variant '{}'", enum_name, variant),
                        ));
                    }
                }
                Ok(())
            }
            Expr::FunctionCall { name, args } => {
                if let Some((param_types, _)) = self.lookup_fn(name) {
                    if args.len() != param_types.len() {
                        return Err(LiphiaError::new(
                            ErrorKind::TypeError,
                            format!(
                                "function '{}' expects {} argument(s), got {}",
                                name, param_types.len(), args.len()
                            ),
                        ));
                    }
                }
                for arg in args { self.check_expr(arg)?; }
                Ok(())
            }
            Expr::ModuleCall { module, name, args } => {
                for arg in args { self.check_expr(arg)?; }
                Err(LiphiaError::new(
                    ErrorKind::InvalidExpression,
                    format!(
                        "internal error: unresolved module call '{}.{}()' reached the type checker — this should have been rewritten by the import resolver",
                        module, name
                    ),
                ))
            }
            Expr::Await(inner) => {
                if !self.in_async_fn {
                    return Err(LiphiaError::new(
                        ErrorKind::AsyncAwaitOutsideAsync,
                        "'await' can only be used inside an 'async fn'",
                    ));
                }
                self.check_expr(inner)
            }
            Expr::Spawn { name, args } => {
                if !self.async_fn_names.contains(name) {
                    return Err(LiphiaError::new(
                        ErrorKind::SpawnNonAsync,
                        format!("spawn: '{}' must be declared as 'async fn'", name),
                    ));
                }
                for arg in args { self.check_expr(arg)?; }
                Ok(())
            }
            Expr::Index(list, idx) => {
                self.check_expr(list)?;
                self.check_expr(idx)
            }
            Expr::List(items) => {
                for item in items { self.check_expr(item)?; }
                Ok(())
            }
            Expr::MapLiteral(pairs) => {
                for (k,v) in pairs {
                    self.check_expr(k)?;
                    self.check_expr(v)?;
                }
                Ok(())
            }
            // No implicit int/float coercion: when both operand types are
            // known, mixing them is a compile error. Unknown types (e.g.
            // values read from a list or map) are left to the VM check.
            Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) |
            Expr::Gt(a, b)  | Expr::Lt(a, b) | Expr::Gte(a, b) | Expr::Lte(a, b) => {
                self.check_expr(a)?;
                self.check_expr(b)?;
                let (ta, tb) = (self.infer(a), self.infer(b));
                let mixed = matches!(
                    (&ta, &tb),
                    (Type::Int, Type::Float) | (Type::Float, Type::Int)
                );
                if mixed {
                    return Err(LiphiaError::new(
                        ErrorKind::TypeError,
                        "cannot mix int and float in an arithmetic or comparison expression",
                    ).with_context("convert one side with to_float() or to_int()"));
                }
                Ok(())
            }
            Expr::Eq(a, b)  | Expr::NotEq(a, b) |
            Expr::And(a, b) | Expr::Or(a, b) => {
                self.check_expr(a)?;
                self.check_expr(b)
            }
            Expr::Not(e) => self.check_expr(e),
            _ => Ok(()),
        }
    }
}