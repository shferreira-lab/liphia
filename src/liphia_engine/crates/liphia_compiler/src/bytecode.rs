// liphia_compiler/src/bytecode.rs
use std::collections::HashMap;
use crate::ast::{Expr, Stmt};
use crate::error::{ErrorKind, LiphiaError, LiphiaResult};
pub use liphia_virtual_machine::opcode::Opcode;

#[derive(Debug)]
pub struct BytecodeProgram {
    pub instructions: Vec<Opcode>,
}

// ── Loop context ──────────────────────────────────────────────────────────────

struct LoopContext {
    break_patches:    Vec<usize>,
    continue_patches: Vec<usize>,
}
impl LoopContext {
    fn new() -> Self { Self { break_patches: vec![], continue_patches: vec![] } }
}

// ── Compiler ──────────────────────────────────────────────────────────────────

struct Compiler {
    instructions: Vec<Opcode>,
    functions:    HashMap<String, usize>,   // name → address
    async_fns:    std::collections::HashSet<String>, // which fns are async
    local_scope:  Option<HashMap<String, u16>>,
    loop_stack:   Vec<LoopContext>,
    consts:       std::collections::HashSet<String>,
    in_async_fn:  bool,  // true while compiling an async fn body
}

impl Compiler {
    fn new() -> Self {
        Self {
            instructions: vec![],
            functions:    HashMap::new(),
            async_fns:    std::collections::HashSet::new(),
            local_scope:  None,
            loop_stack:   vec![],
            consts:       std::collections::HashSet::new(),
            in_async_fn:  false,
        }
    }

    fn enter_scope(&mut self) { self.local_scope = Some(HashMap::new()); }
    fn exit_scope(&mut self)  { self.local_scope = None; }

    fn declare_local(&mut self, name: &str) -> u16 {
        let scope = self.local_scope.as_mut().unwrap();
        let idx   = scope.len() as u16;
        scope.insert(name.to_string(), idx);
        idx
    }

    fn resolve_var(&self, name: &str) -> Opcode {
        if let Some(scope) = &self.local_scope {
            if let Some(&idx) = scope.get(name) { return Opcode::LoadVar(idx); }
        }
        Opcode::LoadGlobal(name.to_string())
    }

    fn store_existing(&self, name: &str) -> Opcode {
        if let Some(scope) = &self.local_scope {
            if let Some(&idx) = scope.get(name) { return Opcode::StoreVar(idx); }
        }
        Opcode::StoreGlobal(name.to_string())
    }

    fn store_new(&mut self, name: &str) -> Opcode {
        if self.local_scope.is_some() {
            let idx = self.declare_local(name);
            Opcode::StoreVar(idx)
        } else {
            Opcode::StoreGlobal(name.to_string())
        }
    }

    // ── Statements ────────────────────────────────────────────────────────

    fn compile_stmt(&mut self, stmt: Stmt) -> LiphiaResult<()> {
        match stmt {
            Stmt::ExprStmt(expr) => {
                self.compile_expr(expr)?;
                self.instructions.push(Opcode::Pop);
            }
            Stmt::VarDecl { name, value, .. } => {
                self.compile_expr(value)?;
                let op = self.store_new(&name);
                self.instructions.push(op);
            }
            Stmt::Var { name, value } => {
                self.compile_expr(value)?;
                let op = self.store_new(&name);
                self.instructions.push(op);
            }
            Stmt::Const { name, value } => {
                self.compile_expr(value)?;
                self.consts.insert(name.clone());
                let op = self.store_new(&name);
                self.instructions.push(op);
            }
            Stmt::Assign { name, value } => {
                if self.consts.contains(&name) {
                    return Err(LiphiaError::new(
                        ErrorKind::TypeError,
                        format!("cannot reassign const '{}'", name),
                    ));
                }
                self.compile_expr(value)?;
                let op = self.store_existing(&name);
                self.instructions.push(op);
            }
            Stmt::AssignIndex { name, index, value } => {
                let load_op = self.resolve_var(&name);
                self.instructions.push(load_op);
                self.compile_expr(index)?;
                self.compile_expr(value)?;
                self.instructions.push(Opcode::SetIndex);
            }
            Stmt::Print(args) => {
                let count = args.len();
                for arg in args { self.compile_expr(arg)?; }
                self.instructions.push(Opcode::Print(count));
            }
            Stmt::Return(expr) => {
                self.compile_expr(expr)?;
                self.instructions.push(Opcode::Return);
            }
            Stmt::Enum(_) => {}

            Stmt::Try { try_block, catch_var, catch_block } => {
                let pos_push = self.instructions.len();
                self.instructions.push(Opcode::PushHandler(0));
                for s in try_block { self.compile_stmt(s)?; }
                self.instructions.push(Opcode::PopHandler);
                let pos_jmp_end = self.instructions.len();
                self.instructions.push(Opcode::Jump(0));

                let catch_addr = self.instructions.len();
                self.instructions[pos_push] = Opcode::PushHandler(catch_addr);
                
                let store_op = self.store_new(&catch_var);
                self.instructions.push(store_op);
                for s in catch_block { self.compile_stmt(s)?; }

                let end = self.instructions.len();
                self.instructions[pos_jmp_end] = Opcode::Jump(end);
            }

            Stmt::If { condition, block, branches, else_block } => {
                self.compile_expr(condition)?;
                let pos_jif = self.instructions.len();
                self.instructions.push(Opcode::JumpIfFalse(0));
                for s in block { self.compile_stmt(s)?; }
                let pos_jmp = self.instructions.len();
                self.instructions.push(Opcode::Jump(0));
                let mut end_jumps = vec![pos_jmp];
                let next = self.instructions.len();
                self.instructions[pos_jif] = Opcode::JumpIfFalse(next);
                for (cond, blk) in branches {
                    self.compile_expr(cond)?;
                    let pj = self.instructions.len();
                    self.instructions.push(Opcode::JumpIfFalse(0));
                    for s in blk { self.compile_stmt(s)?; }
                    let pjf = self.instructions.len();
                    self.instructions.push(Opcode::Jump(0));
                    end_jumps.push(pjf);
                    let nx = self.instructions.len();
                    self.instructions[pj] = Opcode::JumpIfFalse(nx);
                }
                if let Some(blk) = else_block {
                    for s in blk { self.compile_stmt(s)?; }
                }
                let end = self.instructions.len();
                for pos in end_jumps { self.instructions[pos] = Opcode::Jump(end); }
            }

            Stmt::While { condition, body } => {
                let loop_start = self.instructions.len();
                self.compile_expr(condition)?;
                let pos_jif = self.instructions.len();
                self.instructions.push(Opcode::JumpIfFalse(0));
                self.loop_stack.push(LoopContext::new());
                for s in body { self.compile_stmt(s)?; }
                self.instructions.push(Opcode::Jump(loop_start));
                let loop_end = self.instructions.len();
                self.instructions[pos_jif] = Opcode::JumpIfFalse(loop_end);
                let ctx = self.loop_stack.pop().unwrap();
                for p in ctx.break_patches    { self.instructions[p] = Opcode::Jump(loop_end); }
                for p in ctx.continue_patches { self.instructions[p] = Opcode::Jump(loop_start); }
            }

            Stmt::For { var, from, to, step, body } => {
                self.compile_expr(from)?;
                let init_op = self.store_new(&var);
                self.instructions.push(init_op);
                let loop_start = self.instructions.len();
                let load_op    = self.resolve_var(&var);
                self.instructions.push(load_op);
                self.compile_expr(to)?;
                self.instructions.push(Opcode::Lt);
                let pos_jif = self.instructions.len();
                self.instructions.push(Opcode::JumpIfFalse(0));
                self.loop_stack.push(LoopContext::new());
                for s in body { self.compile_stmt(s)?; }
                let step_target = self.instructions.len();
                let load_step   = self.resolve_var(&var);
                self.instructions.push(load_step);
                match step {
                    Some(s) => self.compile_expr(s)?,
                    None    => self.instructions.push(Opcode::PushInt(1)),
                }
                self.instructions.push(Opcode::Add);
                let store_op = self.store_existing(&var);
                self.instructions.push(store_op);
                self.instructions.push(Opcode::Jump(loop_start));
                let loop_end = self.instructions.len();
                self.instructions[pos_jif] = Opcode::JumpIfFalse(loop_end);
                let ctx = self.loop_stack.pop().unwrap();
                for p in ctx.break_patches    { self.instructions[p] = Opcode::Jump(loop_end); }
                for p in ctx.continue_patches { self.instructions[p] = Opcode::Jump(step_target); }
            }

            Stmt::Break => {
                if self.loop_stack.is_empty() {
                    return Err(LiphiaError::new(ErrorKind::BreakOutsideLoop, "'break' outside loop"));
                }
                let pos = self.instructions.len();
                self.instructions.push(Opcode::Jump(0));
                self.loop_stack.last_mut().unwrap().break_patches.push(pos);
            }

            Stmt::Continue => {
                if self.loop_stack.is_empty() {
                    return Err(LiphiaError::new(ErrorKind::ContinueOutsideLoop, "'continue' outside loop"));
                }
                let pos = self.instructions.len();
                self.instructions.push(Opcode::Jump(0));
                self.loop_stack.last_mut().unwrap().continue_patches.push(pos);
            }

            Stmt::Fn { .. } => {} // compiled in second pass
        }
        Ok(())
    }

    // ── Expressions ───────────────────────────────────────────────────────

    fn compile_expr(&mut self, expr: Expr) -> LiphiaResult<()> {
        match expr {
            Expr::Int(v)    => self.instructions.push(Opcode::PushInt(v)),
            Expr::Float(v)  => self.instructions.push(Opcode::PushFloat(v)),
            Expr::Str(v)    => self.instructions.push(Opcode::PushString(v)),
            Expr::Bool(v)   => self.instructions.push(Opcode::PushBool(v)),
            Expr::Null      => self.instructions.push(Opcode::PushNull),
            Expr::EnumVariant { enum_name, variant } => {
                self.instructions.push(Opcode::PushEnum(enum_name, variant));
            }
            Expr::Variable(name) => {
                let op = self.resolve_var(&name);
                self.instructions.push(op);
            }
            Expr::List(items) => {
                let count = items.len();
                for item in items { self.compile_expr(item)?; }
                self.instructions.push(Opcode::BuildList(count));
            }
            Expr::MapLiteral(pairs) => {
                let count = pairs.len();
                for (k, v) in pairs {
                    self.compile_expr(k)?;
                    self.compile_expr(v)?;
                }
                self.instructions.push(Opcode::BuildMap(count));
            }
            Expr::Index(list_expr, index_expr) => {
                self.compile_expr(*list_expr)?;
                self.compile_expr(*index_expr)?;
                self.instructions.push(Opcode::GetIndex);
            }
            Expr::Add(a,b)   => { self.compile_expr(*a)?; self.compile_expr(*b)?; self.instructions.push(Opcode::Add); }
            Expr::Sub(a,b)   => { self.compile_expr(*a)?; self.compile_expr(*b)?; self.instructions.push(Opcode::Sub); }
            Expr::Mul(a,b)   => { self.compile_expr(*a)?; self.compile_expr(*b)?; self.instructions.push(Opcode::Mul); }
            Expr::Div(a,b)   => { self.compile_expr(*a)?; self.compile_expr(*b)?; self.instructions.push(Opcode::Div); }
            Expr::Eq(a,b)    => { self.compile_expr(*a)?; self.compile_expr(*b)?; self.instructions.push(Opcode::Eq); }
            Expr::NotEq(a,b) => { self.compile_expr(*a)?; self.compile_expr(*b)?; self.instructions.push(Opcode::Neq); }
            Expr::Gt(a,b)    => { self.compile_expr(*a)?; self.compile_expr(*b)?; self.instructions.push(Opcode::Gt); }
            Expr::Lt(a,b)    => { self.compile_expr(*a)?; self.compile_expr(*b)?; self.instructions.push(Opcode::Lt); }
            Expr::Gte(a,b)   => { self.compile_expr(*a)?; self.compile_expr(*b)?; self.instructions.push(Opcode::Gte); }
            Expr::Lte(a,b)   => { self.compile_expr(*a)?; self.compile_expr(*b)?; self.instructions.push(Opcode::Lte); }
            Expr::And(a,b)   => { self.compile_expr(*a)?; self.compile_expr(*b)?; self.instructions.push(Opcode::And); }
            Expr::Or(a,b)    => { self.compile_expr(*a)?; self.compile_expr(*b)?; self.instructions.push(Opcode::Or); }
            Expr::Not(e)     => { self.compile_expr(*e)?; self.instructions.push(Opcode::Not); }

            // ── await <inner_expr> ────────────────────────────────────────
            // If inner is a native call (e.g. http_accept()), it stays as
            // CallNamed.  The VM's Suspend handler will re-run it each tick
            // until the native returns a non-null/non-false "ready" value.
            //
            // If inner is a call to a user async fn, it becomes a Call to
            // that fn's address.  After the call the scheduler sees Suspend
            // and yields — the callee task runs, and when it returns the
            // caller is resumed with the result on the stack.
            Expr::Await(inner) => {
                if !self.in_async_fn {
                    return Err(LiphiaError::new(
                        ErrorKind::InvalidStatement,
                        "'await' can only be used inside an 'async fn'",
                    ));
                }
                self.compile_expr(*inner)?;
                self.instructions.push(Opcode::Suspend);
            }

            // ── spawn name(args) ──────────────────────────────────────────
            // Push args, emit Spawn(address_placeholder, arg_count).
            // Address is patched in the resolution pass below.
            Expr::Spawn { name, args } => {
                let arg_count = args.len();
                for arg in args { self.compile_expr(arg)?; }
                // Placeholder — resolved in the same pass as CallNamed → Call
                self.instructions.push(Opcode::CallNamed(
                    format!("__spawn__{}", name),
                    arg_count,
                ));
            }

            Expr::FunctionCall { name, args } => {
                if name == "input" {
                    if args.len() != 1 {
                        return Err(LiphiaError::new(
                            ErrorKind::TypeError,
                            format!("input() requires 1 argument, got {}", args.len()),
                        ));
                    }
                    self.compile_expr(args.into_iter().next().unwrap())?;
                    self.instructions.push(Opcode::Print(1));
                    self.instructions.push(Opcode::Input);
                } else {
                    let count = args.len();
                    for arg in args { self.compile_expr(arg)?; }
                    self.instructions.push(Opcode::CallNamed(name, count));
                }
            }
            Expr::ModuleCall { module, name, .. } => {
                Err(LiphiaError::new(
                    ErrorKind::InvalidExpression,
                    format!(
                        "internal error: unresolved module call '{}.{}()' reached the bytecode compiler — this should have been rewritten by the import resolver",
                        module, name
                    ),
                ))?
            }
        }
        Ok(())
    }
}



// ── Public entry ──────────────────────────────────────────────────────────────

pub fn generate_bytecode(stmts: Vec<Stmt>) -> LiphiaResult<BytecodeProgram> {
    let mut c = Compiler::new();

    // Split top-level statements from function definitions
    let mut main_stmts = vec![];
    let mut fn_stmts   = vec![];
    for stmt in stmts {
        match stmt {
            Stmt::Fn { .. } => fn_stmts.push(stmt),
            _               => main_stmts.push(stmt),
        }
    }

    // ── 1. Compile main body ──────────────────────────────────────────────
    for stmt in main_stmts { c.compile_stmt(stmt)?; }
    c.instructions.push(Opcode::Halt);

    // ── 2. Compile function bodies ────────────────────────────────────────
    for stmt in fn_stmts {
        if let Stmt::Fn { name, params, body, is_async, .. } = stmt {
            let address = c.instructions.len();
            c.functions.insert(name.clone(), address);
            if is_async { c.async_fns.insert(name.clone()); }

            c.enter_scope();
            let prev_async  = c.in_async_fn;
            c.in_async_fn   = is_async;

            // Store parameters from stack (pushed left-to-right by caller)
            let pnames: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
            for pn in pnames.iter().rev() {
                let idx = c.declare_local(pn);
                c.instructions.push(Opcode::StoreVar(idx));
            }

            for s in body { c.compile_stmt(s)?; }

            // Implicit return null for void / async fns
            c.instructions.push(Opcode::PushNull);
            c.instructions.push(Opcode::Return);

            c.in_async_fn = prev_async;
            c.exit_scope();
        }
    }

    // ── 3. Resolve CallNamed → Call / Spawn ───────────────────────────────
    let functions = c.functions.clone();
    let async_fns = c.async_fns.clone();

    for op in c.instructions.iter_mut() {
        match op {
            // Regular call to a user-defined function
            Opcode::CallNamed(name, count) if !name.starts_with("__spawn__") => {
                if let Some(&address) = functions.get(name) {
                    *op = Opcode::Call(address, *count);
                }
                // else: native — stays as CallNamed
            }
            // spawn name(args) placeholder
            Opcode::CallNamed(name, count) if name.starts_with("__spawn__") => {
                let fn_name = name.strip_prefix("__spawn__").unwrap().to_string();
                if let Some(&address) = functions.get(&fn_name) {
                    if !async_fns.contains(&fn_name) {
                        return Err(LiphiaError::new(
                            ErrorKind::TypeError,
                            format!("spawn: '{}' must be declared as 'async fn'", fn_name),
                        ));
                    }
                    *op = Opcode::Spawn(address, *count);
                } else {
                    return Err(LiphiaError::new(
                        ErrorKind::UndefinedFunction,
                        format!("spawn: function '{}' not defined", fn_name),
                    ));
                }
            }
            _ => {}
        }
    }

    Ok(BytecodeProgram { instructions: c.instructions })
}
