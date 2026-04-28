// liphia_compiler/src/error.rs
//
// Unified error type for the Liphia compiler and VM.
// Replaces all panic! calls with structured, user-friendly messages.

use std::fmt;

#[derive(Debug, Clone)]
pub struct LiphiaError {
    pub kind: ErrorKind,
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub context: Option<String>, // Novo campo opcional
}

#[derive(Debug, Clone)]
pub enum ErrorKind {
    // Lexer errors
    UnexpectedChar,
    UnterminatedString,
    // Parser errors
    UnexpectedToken,
    InvalidStatement,
    InvalidExpression,
    InvalidType,
    ExpectedIdent,
    // Compiler errors
    UndefinedFunction,
    BreakOutsideLoop,
    ContinueOutsideLoop,
    // VM errors
    UndefinedVariable,
    TypeError,
    IndexOutOfBounds,
    InvalidIndex,
    DivisionByZero,
    StackUnderflow,
    UnresolvedCall,
}

impl ErrorKind {
    /// Código numérico opcional para rastrear ou logar erros
    pub fn code(&self) -> u16 {
        match self {
            // Lexer
            ErrorKind::UnexpectedChar      => 1001,
            ErrorKind::UnterminatedString  => 1002,
            // Parser
            ErrorKind::UnexpectedToken     => 2001,
            ErrorKind::InvalidStatement    => 2002,
            ErrorKind::InvalidExpression   => 2003,
            ErrorKind::InvalidType         => 2004,
            ErrorKind::ExpectedIdent       => 2005,
            // Compiler / VM
            ErrorKind::UndefinedFunction   => 3001,
            ErrorKind::BreakOutsideLoop    => 3002,
            ErrorKind::ContinueOutsideLoop => 3003,
            ErrorKind::UndefinedVariable   => 3004,
            ErrorKind::TypeError           => 3005,
            ErrorKind::IndexOutOfBounds    => 3006,
            ErrorKind::InvalidIndex        => 3007,
            ErrorKind::DivisionByZero      => 3008,
            ErrorKind::StackUnderflow      => 3009,
            ErrorKind::UnresolvedCall      => 3010,
        }
    }
}

impl LiphiaError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            line: None,
            column: None,
            context: None,
        }
    }

    pub fn at(mut self, line: usize, column: usize) -> Self {
        self.line = Some(line);
        self.column = Some(column);
        self
    }

    pub fn at_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Adiciona contexto opcional extra sobre o erro
    pub fn with_context(mut self, ctx: impl Into<String>) -> Self {
        self.context = Some(ctx.into());
        self
    }

    /// Métodos auxiliares semânticos
    pub fn lexer(kind: ErrorKind, message: impl Into<String>, line: usize, column: usize) -> Self {
        Self::new(kind, message).at(line, column)
    }

    pub fn parser(kind: ErrorKind, message: impl Into<String>, line: usize, column: usize) -> Self {
        Self::new(kind, message).at(line, column)
    }

    pub fn vm(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self::new(kind, message)
    }
}

impl fmt::Display for LiphiaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind_str = match self.kind {
            ErrorKind::UnexpectedChar      => "unexpected character",
            ErrorKind::UnterminatedString  => "unterminated string",
            ErrorKind::UnexpectedToken     => "unexpected token",
            ErrorKind::InvalidStatement    => "invalid statement",
            ErrorKind::InvalidExpression   => "invalid expression",
            ErrorKind::InvalidType         => "invalid type",
            ErrorKind::ExpectedIdent       => "expected identifier",
            ErrorKind::UndefinedFunction   => "undefined function",
            ErrorKind::BreakOutsideLoop    => "break outside loop",
            ErrorKind::ContinueOutsideLoop => "continue outside loop",
            ErrorKind::UndefinedVariable   => "undefined variable",
            ErrorKind::TypeError           => "type error",
            ErrorKind::IndexOutOfBounds    => "index out of bounds",
            ErrorKind::InvalidIndex        => "invalid index",
            ErrorKind::DivisionByZero      => "division by zero",
            ErrorKind::StackUnderflow      => "stack underflow",
            ErrorKind::UnresolvedCall      => "unresolved function call",
        };

        // Formata a saída de erro
        let base_msg = match (self.line, self.column) {
            (Some(l), Some(c)) => format!("error[{}:{}][code {} {}]: {}", l, c, self.kind.code(), kind_str, self.message),
            (Some(l), None)    => format!("error[{}][code {} {}]: {}", l, self.kind.code(), kind_str, self.message),
            _                  => format!("error[code {} {}]: {}", self.kind.code(), kind_str, self.message),
        };

        if let Some(ctx) = &self.context {
            write!(f, "{} [{}]", base_msg, ctx)
        } else {
            write!(f, "{}", base_msg)
        }
    }
}

pub type LiphiaResult<T> = Result<T, LiphiaError>;