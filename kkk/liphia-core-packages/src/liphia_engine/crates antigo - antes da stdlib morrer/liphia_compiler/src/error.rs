// liphia_compiler/src/error.rs
//
// Unified error type for the Liphia compiler and VM — Engine 0.9.0
//
// Error code ranges:
//   1000-1999  Lexer errors
//   2000-2999  Parser errors
//   3000-3999  Compiler errors
//   4000-4999  VM / runtime errors
//   5000-5999  Async / concurrency errors

use std::fmt;

// ── Error kind ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum ErrorKind {
    // ── Lexer (1000) ──────────────────────────────────────────────────────
    UnexpectedChar,
    UnterminatedString,

    // ── Parser (2000) ─────────────────────────────────────────────────────
    UnexpectedToken,
    InvalidStatement,
    InvalidExpression,
    InvalidType,
    ExpectedIdent,
    UnclosedDelimiter,   // unclosed (, [, or argument list

    // ── Compiler (3000) ───────────────────────────────────────────────────
    UndefinedFunction,
    BreakOutsideLoop,
    ContinueOutsideLoop,
    ReturnOutsideFunction,
    ConstReassignment,

    // ── VM / runtime (4000) ───────────────────────────────────────────────
    UndefinedVariable,
    TypeError,
    IndexOutOfBounds,
    InvalidIndex,
    DivisionByZero,
    StackUnderflow,
    UnresolvedCall,
    InvalidArgCount,
    NullDereference,

    // ── Async / concurrency (5000) ────────────────────────────────────────
    AsyncAwaitOutsideAsync,  // await used outside async fn
    SpawnNonAsync,           // spawn called on a non-async fn
    InvalidSuspend,          // Suspend opcode reached in unexpected state
}

impl ErrorKind {
    pub fn code(&self) -> u16 {
        match self {
            // Lexer
            ErrorKind::UnexpectedChar          => 1001,
            ErrorKind::UnterminatedString      => 1002,
            // Parser
            ErrorKind::UnexpectedToken         => 2001,
            ErrorKind::InvalidStatement        => 2002,
            ErrorKind::InvalidExpression       => 2003,
            ErrorKind::InvalidType             => 2004,
            ErrorKind::ExpectedIdent           => 2005,
            ErrorKind::UnclosedDelimiter       => 2006,
            // Compiler
            ErrorKind::UndefinedFunction       => 3001,
            ErrorKind::BreakOutsideLoop        => 3002,
            ErrorKind::ContinueOutsideLoop     => 3003,
            ErrorKind::ReturnOutsideFunction   => 3004,
            ErrorKind::ConstReassignment       => 3005,
            // VM / runtime
            ErrorKind::UndefinedVariable       => 4001,
            ErrorKind::TypeError               => 4002,
            ErrorKind::IndexOutOfBounds        => 4003,
            ErrorKind::InvalidIndex            => 4004,
            ErrorKind::DivisionByZero          => 4005,
            ErrorKind::StackUnderflow          => 4006,
            ErrorKind::UnresolvedCall          => 4007,
            ErrorKind::InvalidArgCount         => 4008,
            ErrorKind::NullDereference         => 4009,
            // Async / concurrency
            ErrorKind::AsyncAwaitOutsideAsync  => 5001,
            ErrorKind::SpawnNonAsync           => 5002,
            ErrorKind::InvalidSuspend          => 5003,
        }
    }

    pub fn category(&self) -> &'static str {
        match self.code() {
            1000..=1999 => "lexer",
            2000..=2999 => "parser",
            3000..=3999 => "compiler",
            4000..=4999 => "runtime",
            5000..=5999 => "async",
            _           => "unknown",
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ErrorKind::UnexpectedChar         => "unexpected character",
            ErrorKind::UnterminatedString     => "unterminated string",
            ErrorKind::UnexpectedToken        => "unexpected token",
            ErrorKind::InvalidStatement       => "invalid statement",
            ErrorKind::InvalidExpression      => "invalid expression",
            ErrorKind::InvalidType            => "invalid type",
            ErrorKind::ExpectedIdent          => "expected identifier",
            ErrorKind::UnclosedDelimiter      => "unclosed delimiter",
            ErrorKind::UndefinedFunction      => "undefined function",
            ErrorKind::BreakOutsideLoop       => "break outside loop",
            ErrorKind::ContinueOutsideLoop    => "continue outside loop",
            ErrorKind::ReturnOutsideFunction  => "return outside function",
            ErrorKind::ConstReassignment      => "const reassignment",
            ErrorKind::UndefinedVariable      => "undefined variable",
            ErrorKind::TypeError              => "type error",
            ErrorKind::IndexOutOfBounds       => "index out of bounds",
            ErrorKind::InvalidIndex           => "invalid index",
            ErrorKind::DivisionByZero         => "division by zero",
            ErrorKind::StackUnderflow         => "stack underflow",
            ErrorKind::UnresolvedCall         => "unresolved function call",
            ErrorKind::InvalidArgCount        => "invalid argument count",
            ErrorKind::NullDereference        => "null dereference",
            ErrorKind::AsyncAwaitOutsideAsync => "await outside async fn",
            ErrorKind::SpawnNonAsync          => "spawn on non-async function",
            ErrorKind::InvalidSuspend         => "invalid suspend",
        };
        write!(f, "{}", s)
    }
}

// ── Error struct ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LiphiaError {
    pub kind:    ErrorKind,
    pub message: String,
    pub line:    Option<usize>,
    pub column:  Option<usize>,
    pub context: Option<String>,
}

impl LiphiaError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            line:    None,
            column:  None,
            context: None,
        }
    }

    /// Attach line and column information.
    pub fn at(mut self, line: usize, column: usize) -> Self {
        self.line   = Some(line);
        self.column = Some(column);
        self
    }

    /// Attach only line information.
    pub fn at_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Attach optional extra context shown in brackets after the message.
    pub fn with_context(mut self, ctx: impl Into<String>) -> Self {
        self.context = Some(ctx.into());
        self
    }

    // ── Semantic constructors ─────────────────────────────────────────────

    /// Construct a lexer error with position.
    pub fn lexer(kind: ErrorKind, message: impl Into<String>, line: usize, col: usize) -> Self {
        Self::new(kind, message).at(line, col)
    }

    /// Construct a parser error with position.
    pub fn parser(kind: ErrorKind, message: impl Into<String>, line: usize, col: usize) -> Self {
        Self::new(kind, message).at(line, col)
    }

    /// Construct a runtime (VM) error without position.
    pub fn runtime(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self::new(kind, message)
    }

    /// Construct an async error without position.
    pub fn async_err(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self::new(kind, message)
    }

    // ── Legacy alias kept for compatibility ───────────────────────────────
    pub fn vm(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self::new(kind, message)
    }
}

impl fmt::Display for LiphiaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format:
        //   error[line:col][code NNNN category/kind]: message [context]
        //   error[line][code NNNN category/kind]: message
        //   error[code NNNN category/kind]: message

        let location = match (self.line, self.column) {
            (Some(l), Some(c)) => format!("[{}:{}]", l, c),
            (Some(l), None)    => format!("[{}]", l),
            _                  => String::new(),
        };

        let code_tag = format!(
            "[code {} {}/{}]",
            self.kind.code(),
            self.kind.category(),
            self.kind
        );

        let base = format!("error{}{}: {}", location, code_tag, self.message);

        if let Some(ctx) = &self.context {
            write!(f, "{} [{}]", base, ctx)
        } else {
            write!(f, "{}", base)
        }
    }
}

// ── Result alias ──────────────────────────────────────────────────────────────

pub type LiphiaResult<T> = Result<T, LiphiaError>;