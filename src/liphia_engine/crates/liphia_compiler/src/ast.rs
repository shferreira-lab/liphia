// liphia_compiler/src/ast.rs

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Str,
    Bool,
    Void,
    List,
    Null,
    Optional(Box<Type>),
    Named(String),
}

impl Type {
    pub fn is_compatible(&self, other: &Type) -> bool {
        if self == other { return true; }
        if let Type::Named(n) = self  { if n == "any" { return true; } }
        if let Type::Named(n) = other { if n == "any" { return true; } }
        if let Type::Optional(_) = self {
            if other == &Type::Null { return true; }
        }
        if let Type::Optional(inner) = self {
            return inner.is_compatible(other);
        }
        false
    }
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Null,
    Variable(String),
    // Arithmetic
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    // Comparison
    Eq(Box<Expr>, Box<Expr>),
    NotEq(Box<Expr>, Box<Expr>),
    Gt(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),
    Gte(Box<Expr>, Box<Expr>),
    Lte(Box<Expr>, Box<Expr>),
    // Logical
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    // Collections
    List(Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    // Enum variant access: Status.Ok
    EnumVariant { enum_name: String, variant: String },
    // Function call
    FunctionCall { name: String, args: Vec<Expr> },
    /// await <expr>  — only valid inside an async fn
    Await(Box<Expr>),
    /// spawn <name>(args)  — fire-and-forget async task
    Spawn { name: String, args: Vec<Expr> },
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub ty:   Type,
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name:     String,
    pub variants: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    VarDecl  { name: String, ty: Type, value: Expr },
    Var      { name: String, value: Expr },
    Const    { name: String, value: Expr },
    Assign   { name: String, value: Expr },
    AssignIndex { name: String, index: Expr, value: Expr },
    Print(Vec<Expr>),
    If {
        condition:  Expr,
        block:      Vec<Stmt>,
        branches:   Vec<(Expr, Vec<Stmt>)>,
        else_block: Option<Vec<Stmt>>,
    },
    While  { condition: Expr, body: Vec<Stmt> },
    For    { var: String, from: Expr, to: Expr, step: Option<Expr>, body: Vec<Stmt> },
    Break,
    Continue,
    /// fn / async fn
    /// `is_async = true`  → compiled to a suspendable coroutine entry point.
    /// The body may contain `Expr::Await`.
    Fn {
        name:        String,
        params:      Vec<Parameter>,
        return_type: Type,
        body:        Vec<Stmt>,
        is_async:    bool,   
    },
    Enum(EnumDef),
    ExprStmt(Expr),
    Return(Expr),
}
