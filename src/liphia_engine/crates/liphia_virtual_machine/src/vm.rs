// liphia_virtual_machine/src/vm.rs
//
// Event-loop VM — supports async/await via cooperative task scheduling,
// plus try/catch via a per-task handler stack.
//
// Architecture:
//   VM holds a VecDeque<Task>. Each Task is an independent coroutine with
//   its own pc, stack, locals, call-frame stack, and error-handler stack.
//   Globals and native_fns are shared across all tasks.
//
// Opcodes that affect scheduling:
//   Opcode::Suspend  — the current task yields.
//                      If the top-of-stack is Value::Null or Value::Bool(false)
//                      the result is not yet ready: the task is re-queued and
//                      the Suspend instruction is re-executed on the next tick
//                      (polling model — native must be idempotent / non-blocking).
//                      Any other value is "ready": execution continues normally.
//   Opcode::Spawn(address, arg_count)
//                    — pops `arg_count` args, creates a new Task at `address`,
//                      pushes Value::Null on the spawner's stack (fire-and-forget).
//
// Opcodes that affect error handling:
//   Opcode::PushHandler(catch_pc) — registers a catch target. Records the
//                      current stack length and frame depth so they can be
//                      restored if an error occurs before the matching
//                      PopHandler.
//   Opcode::PopHandler — removes the most recently pushed handler once the
//                      try block completes normally.
//
//   Any VmError raised while a handler is active is caught instead of
//   propagated: the stack/frames are unwound back to the state saved by
//   PushHandler, the error message is pushed as a Value::Str, and pc jumps
//   to catch_pc. If no handler is active, the error propagates exactly as
//   before (task aborts, error surfaces to the caller of vm.run()).
//
// Single-threaded: all tasks run on the same OS thread in round-robin order.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::io::{self, Write};
use std::rc::Rc;

use crate::opcode::Opcode;
use crate::value::Value;

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct VmError {
    pub message: String,
}

impl VmError {
    pub fn new(msg: impl Into<String>) -> Self { Self { message: msg.into() } }
}

impl std::fmt::Display for VmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "runtime error: {}", self.message)
    }
}

pub type VmResult<T> = Result<T, VmError>;

// ── Frame ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Frame {
    return_pc: usize,
    base:      usize,
}

// ── Error handler ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Handler {
    catch_pc:   usize,
    stack_base: usize,
    frame_base: usize,
}

// ── Task ──────────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct Task {
    pc:       usize,
    stack:    Vec<Value>,
    locals:   Vec<Value>,
    frames:   Vec<Frame>,
    handlers: Vec<Handler>,
}

impl Task {
    fn new(pc: usize, args: Vec<Value>) -> Self {
        let mut stack = args;
        stack.reverse();
        Self { pc, stack, locals: vec![], frames: vec![], handlers: vec![] }
    }

    fn pop(&mut self, ctx: &str) -> VmResult<Value> {
        self.stack.pop().ok_or_else(|| VmError::new(format!("stack underflow in {}", ctx)))
    }

    fn pop2(&mut self, ctx: &str) -> VmResult<(Value, Value)> {
        let b = self.pop(ctx)?;
        let a = self.pop(ctx)?;
        Ok((a, b))
    }

    /// Drops handlers whose enclosing frame has just been left (used after
    /// Return, so a try/catch that never reached PopHandler because the
    /// function returned early doesn't leak into the caller's scope).
    fn prune_stale_handlers(&mut self) {
        while self.handlers.last().map_or(false, |h| h.frame_base >= self.frames.len()) {
            self.handlers.pop();
        }
    }
}

// ── Native function type ──────────────────────────────────────────────────────

pub type NativeFn = fn(Vec<Value>) -> VmResult<Value>;

// ── VM ────────────────────────────────────────────────────────────────────────

pub struct VM {
    globals:    HashMap<String, Value>,
    native_fns: HashMap<String, NativeFn>,
}

impl VM {
    pub fn new() -> Self {
        Self {
            globals:    HashMap::new(),
            native_fns: HashMap::new(),
        }
    }

    pub fn register_native(&mut self, name: &str, f: NativeFn) {
        self.native_fns.insert(name.to_string(), f);
    }

    // ── Entry point ───────────────────────────────────────────────────────

    pub fn run(&mut self, program: Vec<Opcode>) -> VmResult<()> {
        let program = Rc::new(program);
        let main_task = Task::new(0, vec![]);
        let mut queue: VecDeque<Task> = VecDeque::new();
        queue.push_back(main_task);

        // Tracks how many consecutive suspend rounds occurred.
        // When all tasks in the queue are suspended, sleep briefly
        // to avoid burning 100% CPU on an empty request queue.
        let mut all_suspended_rounds = 0usize;

        while let Some(mut task) = queue.pop_front() {
            match self.step(&program, &mut task, &mut queue)? {
                StepResult::Halt => {
                    all_suspended_rounds = 0;
                }
                StepResult::Suspend => {
                    queue.push_back(task);
                    all_suspended_rounds += 1;
                    if all_suspended_rounds >= queue.len() {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                        all_suspended_rounds = 0;
                    }
                }
                StepResult::Continue => {
                    all_suspended_rounds = 0;
                    queue.push_back(task);
                }
            }
        }

        Ok(())
    }

    // ── Execute one quantum of a task ─────────────────────────────────────
    //
    // Returns:
    //   Continue — task ran some instructions, not done yet
    //   Suspend  — task hit Opcode::Suspend and needs to yield
    //   Halt     — task hit Opcode::Halt or returned from top frame
    //
    // Errors raised by individual instructions are intercepted here: if the
    // task has an active handler, the error is caught (stack/frames unwound,
    // message pushed, pc jumps to catch_pc) instead of propagating out of
    // step(). Only truly uncaught errors reach the `?` at the call site.

    fn step(
        &mut self,
        program: &Rc<Vec<Opcode>>,
        task:    &mut Task,
        queue:   &mut VecDeque<Task>,
    ) -> VmResult<StepResult> {
        // Global-scope tasks (server loop) use a smaller quantum so they
        // yield faster and keep the event loop responsive.
        let quantum = if task.frames.is_empty() { 32 } else { 256 };

        for _ in 0..quantum {
            if task.pc >= program.len() {
                return Ok(StepResult::Halt);
            }

            let op = program[task.pc].clone();
            match self.exec_instruction(&op, task, queue) {
                Ok(InstrFlow::Next) => { task.pc += 1; }
                Ok(InstrFlow::Jumped) => {}
                Ok(InstrFlow::Suspend) => return Ok(StepResult::Suspend),
                Ok(InstrFlow::Halt)    => return Ok(StepResult::Halt),
                Err(e) => {
                    if let Some(h) = task.handlers.pop() {
                        task.stack.truncate(h.stack_base);
                        task.frames.truncate(h.frame_base);
                        task.stack.push(Value::Str(Rc::new(e.message.clone())));
                        task.pc = h.catch_pc;
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Ok(StepResult::Continue)
    }

    // ── Execute a single instruction ────────────────────────────────────────

    fn exec_instruction(
        &mut self,
        op:    &Opcode,
        task:  &mut Task,
        queue: &mut VecDeque<Task>,
    ) -> VmResult<InstrFlow> {
        match op {
            Opcode::PushInt(v)    => task.stack.push(Value::Int(*v)),
            Opcode::PushFloat(v)  => task.stack.push(Value::Float(*v)),
            Opcode::PushString(v) => task.stack.push(Value::Str(Rc::new(v.clone()))),
            Opcode::PushBool(v)   => task.stack.push(Value::Bool(*v)),
            Opcode::PushNull      => task.stack.push(Value::Null),
            Opcode::PushEnum(en, vn) => {
                task.stack.push(Value::EnumVariant {
                    enum_name: Rc::new(en.clone()),
                    variant:   Rc::new(vn.clone()),
                });
            }

            // ── Variables ─────────────────────────────────────────────

            Opcode::StoreVar(idx) => {
                let idx  = *idx as usize;
                let base = task.frames.last().map(|f| f.base).unwrap_or(0);
                let pos  = base + idx;
                let val  = task.pop("StoreVar")?;
                if pos < task.locals.len() {
                    task.locals[pos] = val;
                } else {
                    while task.locals.len() < pos { task.locals.push(Value::Null); }
                    task.locals.push(val);
                }
            }

            Opcode::LoadVar(idx) => {
                let idx  = *idx as usize;
                let base = task.frames.last().map(|f| f.base).unwrap_or(0);
                let val  = task.locals.get(base + idx)
                    .ok_or_else(|| VmError::new(format!("local slot {} out of range", idx)))?
                    .clone();
                task.stack.push(val);
            }

            Opcode::StoreGlobal(name) => {
                let val = task.pop("StoreGlobal")?;
                self.globals.insert(name.clone(), val);
            }

            Opcode::LoadGlobal(name) => {
                let val = self.globals.get(name)
                    .ok_or_else(|| VmError::new(format!("undefined variable '{}'", name)))?
                    .clone();
                task.stack.push(val);
            }

            // ── Arithmetic ────────────────────────────────────────────

            Opcode::Add  => self.op_add(task)?,
            Opcode::Sub  => self.op_sub(task)?,
            Opcode::Mul  => self.op_mul(task)?,
            Opcode::Div  => self.op_div(task)?,

            // ── Comparison ────────────────────────────────────────────

            Opcode::Eq  => { let (a,b) = task.pop2("Eq")?;  task.stack.push(Value::Bool(a == b)); }
            Opcode::Neq => { let (a,b) = task.pop2("Neq")?; task.stack.push(Value::Bool(a != b)); }
            Opcode::Gt  => self.op_cmp(task, |a,b| a>b,  |a,b| a>b)?,
            Opcode::Lt  => self.op_cmp(task, |a,b| a<b,  |a,b| a<b)?,
            Opcode::Gte => self.op_cmp(task, |a,b| a>=b, |a,b| a>=b)?,
            Opcode::Lte => self.op_cmp(task, |a,b| a<=b, |a,b| a<=b)?,

            // ── Logical ───────────────────────────────────────────────

            Opcode::And => {
                let (a,b) = task.pop2("And")?;
                match (a,b) {
                    (Value::Bool(x), Value::Bool(y)) => task.stack.push(Value::Bool(x && y)),
                    _ => return Err(VmError::new("'and' requires two bool values")),
                }
            }

            Opcode::Or => {
                let (a,b) = task.pop2("Or")?;
                match (a,b) {
                    (Value::Bool(x), Value::Bool(y)) => task.stack.push(Value::Bool(x || y)),
                    _ => return Err(VmError::new("'or' requires two bool values")),
                }
            }

            Opcode::Not => {
                let v = task.pop("Not")?;
                match v {
                    Value::Bool(x) => task.stack.push(Value::Bool(!x)),
                    _ => return Err(VmError::new("'not' requires a bool value")),
                }
            }

            // ── I/O ───────────────────────────────────────────────────

            Opcode::Input => {
                let mut buf = String::new();
                io::stdout().flush().unwrap();
                io::stdin().read_line(&mut buf)
                    .map_err(|e| VmError::new(format!("failed to read input: {}", e)))?;
                task.stack.push(Value::Str(Rc::new(buf.trim_end().to_string())));
            }

            Opcode::Print(count) => {
                let count = *count;
                let mut args = Vec::with_capacity(count);
                for _ in 0..count { args.push(task.pop("Print")?); }
                args.reverse();
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { print!(" "); }
                    print!("{}", arg);
                }
                println!();
            }

            // ── Control flow ──────────────────────────────────────────

            Opcode::Jump(dest) => {
                task.pc = *dest;
                return Ok(InstrFlow::Jumped);
            }

            Opcode::JumpIfFalse(dest) => {
                match task.pop("JumpIfFalse")? {
                    Value::Bool(false) => { task.pc = *dest; return Ok(InstrFlow::Jumped); }
                    Value::Bool(true)  => {}
                    _ => return Err(VmError::new("condition must be a bool value")),
                }
            }

            // ── Function calls ────────────────────────────────────────

            Opcode::Call(address, _) => {
                let base = task.locals.len();
                task.frames.push(Frame { return_pc: task.pc + 1, base });
                task.pc = *address;
                return Ok(InstrFlow::Jumped);
            }

            Opcode::CallNamed(name, arg_count) => {
                let arg_count = *arg_count;
                if let Some(&native_fn) = self.native_fns.get(name) {
                    let mut args = Vec::with_capacity(arg_count);
                    for _ in 0..arg_count { args.push(task.pop("CallNamed")?); }
                    args.reverse();
                    let result = native_fn(args)?;
                    task.stack.push(result);
                } else {
                    return Err(VmError::new(format!(
                        "unresolved call to '{}' — not defined and not in stdlib", name
                    )));
                }
            }

            Opcode::Return => {
                let ret   = task.stack.pop().unwrap_or(Value::Null);
                let frame = task.frames.pop()
                    .ok_or_else(|| VmError::new("return without active call frame"))?;
                task.locals.truncate(frame.base);
                task.pc = frame.return_pc;
                task.stack.push(ret);
                task.prune_stale_handlers();
                return Ok(InstrFlow::Jumped);
            }

            // ── Collections ───────────────────────────────────────────

            Opcode::BuildList(count) => {
                let count = *count;
                let mut items = Vec::with_capacity(count);
                for _ in 0..count { items.push(task.pop("BuildList")?); }
                items.reverse();
                task.stack.push(Value::List(Rc::new(RefCell::new(items))));
            }

            Opcode::BuildMap(count) => {
                let count = *count;
                let mut flat = Vec::with_capacity(count * 2);
                for _ in 0..count * 2 { flat.push(task.pop("BuildMap")?); }
                flat.reverse();
                let mut pairs = Vec::with_capacity(count);
                let mut it = flat.into_iter();
                while let (Some(k), Some(v)) = (it.next(), it.next()) {
                    pairs.push((k, v));
                }
                task.stack.push(Value::Map(Rc::new(RefCell::new(pairs))));
            }

            Opcode::GetIndex => {
                let index = task.pop("GetIndex")?;
                let list  = task.pop("GetIndex")?;
                match (list, index) {
                    (Value::List(rc), Value::Int(i)) => {
                        let items = rc.borrow();
                        let i = if i < 0 { items.len() as i64 + i } else { i };
                        if i < 0 || i as usize >= items.len() {
                            return Err(VmError::new(format!(
                                "index {} out of bounds (list length {})", i, items.len())));
                        }
                        task.stack.push(items[i as usize].clone());
                    }
                    (Value::Str(s), Value::Int(i)) => {
                        let chars: Vec<char> = s.chars().collect();
                        let i = if i < 0 { chars.len() as i64 + i } else { i };
                        if i < 0 || i as usize >= chars.len() {
                            return Err(VmError::new(format!(
                                "string index {} out of bounds (length {})", i, chars.len())));
                        }
                        task.stack.push(Value::Str(Rc::new(chars[i as usize].to_string())));
                    }
                    (Value::Map(rc), key) => {
                        let items = rc.borrow();
                        match items.iter().find(|(k, _)| *k == key) {
                            Some((_, v)) => task.stack.push(v.clone()),
                            None => return Err(VmError::new("key not found in map")),
                        }
                    }
                    (_, Value::Int(_)) => return Err(VmError::new("index operator requires a list or str")),
                    _ => return Err(VmError::new("list index must be an int")),
                }
            }

            Opcode::SetIndex => {
                let value = task.pop("SetIndex")?;
                let index = task.pop("SetIndex")?;
                let list  = task.pop("SetIndex")?;
                match (list, index) {
                    (Value::List(rc), Value::Int(i)) => {
                        let mut items = rc.borrow_mut();
                        let i = if i < 0 { items.len() as i64 + i } else { i };
                        if i < 0 || i as usize >= items.len() {
                            return Err(VmError::new(format!(
                                "index {} out of bounds (list length {})", i, items.len())));
                        }
                        items[i as usize] = value;
                    }
                    (Value::Map(rc), key) => {
                        let mut items = rc.borrow_mut();
                        if let Some(entry) = items.iter_mut().find(|(k, _)| *k == key) {
                            entry.1 = value;
                        } else {
                            items.push((key, value));
                        }
                    }
                    _ => return Err(VmError::new("list index must be an int")),
                }
            }

            Opcode::Pop => { task.stack.pop(); }

            // ── Error handling ────────────────────────────────────────

            Opcode::PushHandler(catch_pc) => {
                task.handlers.push(Handler {
                    catch_pc:   *catch_pc,
                    stack_base: task.stack.len(),
                    frame_base: task.frames.len(),
                });
            }

            Opcode::PopHandler => {
                task.handlers.pop();
            }

            // ── Async ─────────────────────────────────────────────────
            //
            // Suspend: if top-of-stack is Null or Bool(false) the awaited
            // value is not ready. DO NOT advance pc — re-queue so the
            // CallNamed above re-executes on the next tick.
            // Any other value means ready; advance normally.

            Opcode::Suspend => {
                let top = task.stack.last().cloned().unwrap_or(Value::Null);
                match top {
                    Value::Null | Value::Bool(false) => {
                        task.stack.pop();
                        return Ok(InstrFlow::Suspend);
                    }
                    _ => {
                        return Ok(InstrFlow::Next);
                    }
                }
            }

            Opcode::Spawn(address, arg_count) => {
                let arg_count = *arg_count;
                let mut args  = Vec::with_capacity(arg_count);
                for _ in 0..arg_count { args.push(task.pop("Spawn")?); }
                args.reverse();
                let new_task = Task::new(*address, args);
                queue.push_back(new_task);
                task.stack.push(Value::Null);
            }

            // ── Halt ──────────────────────────────────────────────────

            Opcode::Halt => {
                #[cfg(debug_assertions)]
                if !task.stack.is_empty() {
                    eprintln!(
                        "[liphia/vm] warning: task ended with {} value(s) on stack",
                        task.stack.len()
                    );
                }
                task.frames.clear();
                task.locals.clear();
                task.stack.clear();
                return Ok(InstrFlow::Halt);
            }
        }

        Ok(InstrFlow::Next)
    }

    // ── Arithmetic helpers ────────────────────────────────────────────────

    fn op_add(&self, task: &mut Task) -> VmResult<()> {
        let (a,b) = task.pop2("Add")?;
        let r = match (a,b) {
            (Value::Int(x),   Value::Int(y))   => Value::Int(x+y),
            (Value::Float(x), Value::Float(y)) => Value::Float(x+y),
            (Value::Str(x),   Value::Str(y))   => Value::Str(Rc::new((*x).clone() + &*y)),
            (Value::Int(x),   Value::Str(y))   => Value::Str(Rc::new(format!("{}{}", x, y))),
            (Value::Str(x),   Value::Int(y))   => Value::Str(Rc::new(format!("{}{}", x, y))),
            _ => return Err(VmError::new("'+' requires int, float, or str values")),
        };
        task.stack.push(r); Ok(())
    }

    fn op_sub(&self, task: &mut Task) -> VmResult<()> {
        let (a,b) = task.pop2("Sub")?;
        let r = match (a,b) {
            (Value::Int(x),   Value::Int(y))   => Value::Int(x-y),
            (Value::Float(x), Value::Float(y)) => Value::Float(x-y),
            _ => return Err(VmError::new("'-' requires int or float values")),
        };
        task.stack.push(r); Ok(())
    }

    fn op_mul(&self, task: &mut Task) -> VmResult<()> {
        let (a,b) = task.pop2("Mul")?;
        let r = match (a,b) {
            (Value::Int(x),   Value::Int(y))   => Value::Int(x*y),
            (Value::Float(x), Value::Float(y)) => Value::Float(x*y),
            _ => return Err(VmError::new("'*' requires int or float values")),
        };
        task.stack.push(r); Ok(())
    }

    fn op_div(&self, task: &mut Task) -> VmResult<()> {
        let (a,b) = task.pop2("Div")?;
        let r = match (a,b) {
            (Value::Int(x),   Value::Int(y))   => {
                if y == 0 { return Err(VmError::new("division by zero")); }
                Value::Int(x/y)
            }
            (Value::Float(x), Value::Float(y)) => {
                if y == 0.0 { return Err(VmError::new("division by zero")); }
                Value::Float(x/y)
            }
            _ => return Err(VmError::new("'/' requires int or float values")),
        };
        task.stack.push(r); Ok(())
    }

    fn op_cmp(
        &self,
        task:      &mut Task,
        int_cmp:   impl Fn(i64,i64) -> bool,
        float_cmp: impl Fn(f64,f64) -> bool,
    ) -> VmResult<()> {
        let (a,b) = task.pop2("cmp")?;
        let r = match (a,b) {
            (Value::Int(x),   Value::Int(y))   => Value::Bool(int_cmp(x,y)),
            (Value::Float(x), Value::Float(y)) => Value::Bool(float_cmp(x,y)),
            _ => return Err(VmError::new("comparison requires int or float values")),
        };
        task.stack.push(r); Ok(())
    }
}

// ── Internal scheduler signals ─────────────────────────────────────────────────

enum StepResult {
    Halt,
    Suspend,
    Continue,
}

/// Signal returned by exec_instruction to tell step() whether to advance
/// pc normally, whether pc was already set by the instruction itself
/// (jumps, calls, returns), or whether the quantum loop must exit early.
enum InstrFlow {
    Next,
    Jumped,
    Suspend,
    Halt,
}