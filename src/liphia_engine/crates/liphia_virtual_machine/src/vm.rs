// liphia_virtual_machine/src/vm.rs
//
// Event-loop VM — supports async/await via cooperative task scheduling.
//
// Architecture:
//   VM holds a VecDeque<Task>.  Each Task is an independent coroutine with
//   its own pc, stack, locals, and call-frame stack.
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
// Single-threaded: all tasks run on the same OS thread in round-robin order.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::io::{self, Write};
use std::rc::Rc;

use crate::opcode::Opcode;
use crate::value::Value;

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
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

// ── Task ──────────────────────────────────────────────────────────────────────

/// A single coroutine / green thread.
#[derive(Debug)]
struct Task {
    pc:     usize,
    stack:  Vec<Value>,
    locals: Vec<Value>,
    frames: Vec<Frame>,
}

impl Task {
    fn new(pc: usize, args: Vec<Value>) -> Self {
        // args are already on a fresh stack so the first instructions of the
        // callee (StoreVar) will consume them.
        let mut stack = args;
        stack.reverse(); // match the Call convention: last arg on top
        Self {
            pc,
            stack,
            locals: vec![],
            frames: vec![],
        }
    }

    fn pop(&mut self, ctx: &str) -> VmResult<Value> {
        self.stack.pop().ok_or_else(|| VmError::new(format!("stack underflow in {}", ctx)))
    }

    fn pop2(&mut self, ctx: &str) -> VmResult<(Value, Value)> {
        let b = self.pop(ctx)?;
        let a = self.pop(ctx)?;
        Ok((a, b))
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
        // Wrap program in Rc so all tasks share the same instruction slice.
        let program = Rc::new(program);

        // The "main" task starts at pc = 0 with no arguments.
        let main_task = Task::new(0, vec![]);
        let mut queue: VecDeque<Task> = VecDeque::new();
        queue.push_back(main_task);

        // Round-robin event loop ─────────────────────────────────────────
        while let Some(mut task) = queue.pop_front() {
            match self.step(&program, &mut task, &mut queue)? {
                StepResult::Halt     => { /* task done — drop it */ }
                StepResult::Suspend  => { queue.push_back(task); }
                StepResult::Continue => { queue.push_back(task); }
            }
        }

        Ok(())
    }

    // ── Execute one "quantum" of a task ───────────────────────────────────
    //
    // Returns:
    //   Continue — task ran some instructions, not done yet
    //   Suspend  — task hit Opcode::Suspend and needs to yield
    //   Halt     — task hit Opcode::Halt or returned from top frame

    fn step(
        &mut self,
        program: &Rc<Vec<Opcode>>,
        task:    &mut Task,
        queue:   &mut VecDeque<Task>,
    ) -> VmResult<StepResult> {
        // Each quantum runs up to QUANTUM instructions before yielding to
        // let other tasks run.  This prevents a tight while-loop in one task
        // from starving all others.
        const QUANTUM: usize = 256;

        for _ in 0..QUANTUM {
            if task.pc >= program.len() {
                return Ok(StepResult::Halt);
            }

            match &program[task.pc].clone() {
                // ── Push ──────────────────────────────────────────────────
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
                    let name = name.clone();
                    let val  = task.pop("StoreGlobal")?;
                    self.globals.insert(name, val);
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
                    continue;
                }
                Opcode::JumpIfFalse(dest) => {
                    let dest = *dest;
                    match task.pop("JumpIfFalse")? {
                        Value::Bool(false) => { task.pc = dest; continue; }
                        Value::Bool(true)  => {}
                        _ => return Err(VmError::new("condition must be a bool value")),
                    }
                }

                // ── Function calls ────────────────────────────────────────
                Opcode::Call(address, _) => {
                    let address = *address;
                    let base    = task.locals.len();
                    task.frames.push(Frame { return_pc: task.pc + 1, base });
                    task.pc = address;
                    continue;
                }
                Opcode::CallNamed(name, arg_count) => {
                    let arg_count = *arg_count;
                    let name      = name.clone();
                    if let Some(&native_fn) = self.native_fns.get(&name) {
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
                    continue;
                }

                // ── Collections ───────────────────────────────────────────
                Opcode::BuildList(count) => {
                    let count = *count;
                    let mut items = Vec::with_capacity(count);
                    for _ in 0..count { items.push(task.pop("BuildList")?); }
                    items.reverse();
                    task.stack.push(Value::List(Rc::new(RefCell::new(items))));
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
                        _ => return Err(VmError::new("list index must be an int")),
                    }
                }

                Opcode::Pop => { task.stack.pop(); }

                // ── Async ─────────────────────────────────────────────────
                //
                // Suspend: check the top-of-stack value.
                //   • Null or Bool(false) → not ready yet.
                //     DO NOT advance pc — stay on Suspend so next tick
                //     the CallNamed above it re-executes.  Re-queue task.
                //   • Any other value    → ready.  Advance normally,
                //     leave value on stack as the result of the await expr.
                Opcode::Suspend => {
                    let top = task.stack.last().cloned().unwrap_or(Value::Null);
                    match top {
                        Value::Null | Value::Bool(false) => {
                            // Pop the not-ready value — will be re-pushed
                            // when the preceding call re-runs next tick.
                            task.stack.pop();
                            // Do NOT advance pc — re-run from Suspend next tick.
                            return Ok(StepResult::Suspend);
                        }
                        _ => {
                            // Value is ready — keep it on stack, advance past Suspend.
                            task.pc += 1;
                            return Ok(StepResult::Continue);
                        }
                    }
                }

                // Spawn(address, arg_count): create new task, fire-and-forget.
                Opcode::Spawn(address, arg_count) => {
                    let address   = *address;
                    let arg_count = *arg_count;
                    let mut args  = Vec::with_capacity(arg_count);
                    for _ in 0..arg_count { args.push(task.pop("Spawn")?); }
                    args.reverse();
                    let new_task = Task::new(address, args);
                    queue.push_back(new_task);
                    // Spawner gets null — it doesn't wait for the child.
                    task.stack.push(Value::Null);
                }

                // ── Halt ──────────────────────────────────────────────────
                Opcode::Halt => return Ok(StepResult::Halt),
            }

            task.pc += 1;
        }

        // Quantum exhausted — yield so other tasks can run.
        Ok(StepResult::Continue)
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

// ── Internal scheduler signal ─────────────────────────────────────────────────

enum StepResult {
    /// Task completed (Halt reached or top-level Return).
    Halt,
    /// Task yielded because an await result wasn't ready yet.
    Suspend,
    /// Task ran a quantum and should continue on next round.
    Continue,
}
