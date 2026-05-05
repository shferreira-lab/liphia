// liphia_virtual_machine/src/opcode.rs

#[derive(Debug, Clone)]
pub enum Opcode {
    // ── Stack ─────────────────────────────────────────────────────────────
    PushInt(i64),
    PushFloat(f64),
    PushString(String),
    PushBool(bool),
    PushNull,
    PushEnum(String, String),

    // ── Variables ─────────────────────────────────────────────────────────
    LoadVar(u16),
    StoreVar(u16),
    LoadGlobal(String),
    StoreGlobal(String),

    // ── Arithmetic ────────────────────────────────────────────────────────
    Add, Sub, Mul, Div,

    // ── Comparison ────────────────────────────────────────────────────────
    Eq, Neq, Gt, Lt, Gte, Lte,

    // ── Logical ───────────────────────────────────────────────────────────
    And, Or, Not,

    // ── I/O ───────────────────────────────────────────────────────────────
    Input,
    Print(usize),

    // ── Control flow ──────────────────────────────────────────────────────
    Jump(usize),
    JumpIfFalse(usize),

    // ── Functions ─────────────────────────────────────────────────────────
    CallNamed(String, usize),
    Call(usize, usize),
    Return,

    // ── Collections ───────────────────────────────────────────────────────
    BuildList(usize),
    GetIndex,
    SetIndex,

    // ── Stack util ────────────────────────────────────────────────────────
    Pop,

    // ── Async / concurrency ───────────────────────────────────────────────
    /// Suspend the current task.
    /// The top-of-stack value is the "pending result" — for native async
    /// calls the native pushes Value::Null and the scheduler re-calls it
    /// on the next tick until it returns a real value.
    /// For user-defined async functions the scheduler resumes the callee
    /// task and wakes the caller when the callee is done.
    Suspend,

    /// Spawn a new independent task starting at `address` with `arg_count`
    /// arguments already on the stack.  The new task runs concurrently;
    /// the spawning task does NOT wait for it (fire-and-forget).
    /// Spawn(address, arg_count)
    Spawn(usize, usize),

    Halt,
}
