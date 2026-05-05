// liphia_compiler/src/parser.rs
use crate::lexer::{Lexer, Token};
use crate::ast::{EnumDef, Expr, Parameter, Stmt, Type};
use crate::error::{ErrorKind, LiphiaError, LiphiaResult};

pub struct Parser {
    lexer:   Lexer,
    current: Token,
    line:    usize,
    column:  usize,
}

impl Parser {
    pub fn new(mut lexer: Lexer) -> LiphiaResult<Self> {
        let line    = lexer.line();
        let column  = lexer.column();
        let current = lexer.next_token()?;
        Ok(Self { lexer, current, line, column })
    }

    fn advance(&mut self) -> LiphiaResult<()> {
        self.line   = self.lexer.line();
        self.column = self.lexer.column();
        self.current = self.lexer.next_token()?;
        Ok(())
    }

    fn expect(&mut self, token: Token) -> LiphiaResult<()> {
        if self.current == token {
            self.advance()
        } else {
            Err(LiphiaError::new(
                ErrorKind::UnexpectedToken,
                format!("expected {:?}, found {:?}", token, self.current),
            ).at(self.line, self.column))
        }
    }

    fn skip_newlines(&mut self) -> LiphiaResult<()> {
        while self.current == Token::Newline { self.advance()?; }
        Ok(())
    }

    // tolerant version of expect(Newline) + expect(Indent).
    // Skips any blank lines/comments between ":" and the first indented line.
    fn expect_indent_after_colon(&mut self) -> LiphiaResult<()> {
        self.skip_newlines()?;
        if self.current == Token::Indent {
            self.advance()
        } else {
            Err(LiphiaError::new(
                ErrorKind::UnexpectedToken,
                format!("expected indented block, found {:?}", self.current),
            ).at(self.line, self.column))
        }
    }

    fn is(&self, keyword: &str) -> bool {
        matches!(&self.current, Token::Ident(n) if n == keyword)
    }

    fn expect_kw(&mut self, keyword: &str) -> LiphiaResult<()> {
        if self.is(keyword) {
            self.advance()
        } else {
            Err(LiphiaError::new(
                ErrorKind::UnexpectedToken,
                format!("expected '{}', found {:?}", keyword, self.current),
            ).at(self.line, self.column))
        }
    }

    // ── Public entry ──────────────────────────────────────────────────────

    pub fn parse(&mut self) -> LiphiaResult<Vec<Stmt>> {
        let mut stmts = vec![];
        self.skip_newlines()?;
        while self.current != Token::EOF {
            stmts.push(self.parse_stmt()?);
            self.skip_newlines()?;
        }
        Ok(stmts)
    }

    // ── Statements ────────────────────────────────────────────────────────

    fn parse_stmt(&mut self) -> LiphiaResult<Stmt> {
        match &self.current {
            Token::Async                           => self.parse_async_fn(),
            Token::Ident(n) if n == "print"    => self.parse_print(),
            Token::Ident(n) if n == "if"       => self.parse_if(),
            Token::Ident(n) if n == "while"    => self.parse_while(),
            Token::Ident(n) if n == "for"      => self.parse_for(),
            Token::Ident(n) if n == "break"    => { self.advance()?; Ok(Stmt::Break) }
            Token::Ident(n) if n == "continue" => { self.advance()?; Ok(Stmt::Continue) }
            Token::Ident(n) if n == "fn"       => self.parse_fn(false),
            Token::Ident(n) if n == "return"   => self.parse_return(),
            Token::Ident(n) if n == "var"      => self.parse_var(),
            Token::Ident(n) if n == "const"    => self.parse_const(),
            Token::Ident(n) if n == "enum"     => self.parse_enum(),
            Token::Ident(n) if n == "spawn"    => self.parse_spawn_stmt(),
            Token::Ident(_)                    => self.parse_ident_stmt(),
            Token::Await => {
                self.advance()?;
                let expr = self.parse_expr()?;
                Ok(Stmt::ExprStmt(Expr::Await(Box::new(expr))))
            }
            _ => Err(LiphiaError::new(
                ErrorKind::InvalidStatement,
                format!("unexpected {:?} at start of statement", self.current),
            ).at(self.line, self.column)),
        }
    }

    // ── spawn fn_name(args) as statement ─────────────────────────────────

    fn parse_spawn_stmt(&mut self) -> LiphiaResult<Stmt> {
        self.advance()?;
        let name = self.take_ident("expected function name after 'spawn'")?;
        self.expect(Token::LParen)?;
        let args = self.parse_arg_list()?;
        Ok(Stmt::ExprStmt(Expr::Spawn { name, args }))
    }

    // ── async fn ─────────────────────────────────────────────────────────

    fn parse_async_fn(&mut self) -> LiphiaResult<Stmt> {
        self.advance()?;
        if !self.is("fn") {
            return Err(LiphiaError::new(
                ErrorKind::UnexpectedToken,
                format!("expected 'fn' after 'async', found {:?}", self.current),
            ).at(self.line, self.column));
        }
        self.parse_fn(true)
    }

    // ── fn (shared by sync and async) ────────────────────────────────────

    fn parse_fn(&mut self, is_async: bool) -> LiphiaResult<Stmt> {
        self.advance()?;
        let name = self.take_ident("expected function name")?;
        self.expect(Token::LParen)?;

        let mut params = vec![];
        while self.current != Token::RParen {
            if self.current == Token::EOF {
                return Err(LiphiaError::new(
                    ErrorKind::UnexpectedToken,
                    "unclosed parameter list — missing ')'",
                ).at(self.line, self.column));
            }
            let pname = self.take_ident("expected parameter name")?;
            self.expect(Token::Colon)?;
            let ty = self.parse_type()?;
            params.push(Parameter { name: pname, ty });
            if self.current == Token::Comma { self.advance()?; }
        }
        self.expect(Token::RParen)?;
        self.expect(Token::Arrow)?;
        let return_type = self.parse_type()?;
        self.expect(Token::Colon)?;

        self.expect_indent_after_colon()?;
        let body = self.parse_block()?;
        Ok(Stmt::Fn { name, params, return_type, body, is_async })
    }

    fn parse_var(&mut self) -> LiphiaResult<Stmt> {
        self.advance()?;
        let name = self.take_ident("expected variable name after 'var'")?;
        if self.current == Token::Colon {
            self.advance()?;
            let ty = self.parse_type()?;
            self.expect(Token::Eq)?;
            let value = self.parse_expr()?;
            return Ok(Stmt::VarDecl { name, ty, value });
        }
        self.expect(Token::Eq)?;
        let value = self.parse_expr()?;
        Ok(Stmt::Var { name, value })
    }

    fn parse_const(&mut self) -> LiphiaResult<Stmt> {
        self.advance()?;
        let name = self.take_ident("expected constant name after 'const'")?;
        if self.current == Token::Colon {
            self.advance()?;
            let _ty = self.parse_type()?;
        }
        self.expect(Token::Eq)?;
        let value = self.parse_expr()?;
        Ok(Stmt::Const { name, value })
    }

    fn parse_enum(&mut self) -> LiphiaResult<Stmt> {
        self.advance()?;
        let name = self.take_ident("expected enum name")?;
        self.expect(Token::Colon)?;
        self.expect_indent_after_colon()?;
        let mut variants = vec![];
        self.skip_newlines()?;
        while self.current != Token::Dedent && self.current != Token::EOF {
            let v = self.take_ident("expected enum variant name")?;
            variants.push(v);
            self.skip_newlines()?;
        }
        self.expect(Token::Dedent)?;
        Ok(Stmt::Enum(EnumDef { name, variants }))
    }

    fn parse_ident_stmt(&mut self) -> LiphiaResult<Stmt> {
        let name = self.take_ident("expected identifier")?;

        // name[index] = value
        if self.current == Token::LBracket {
            self.advance()?;
            let index = self.parse_expr()?;
            self.expect(Token::RBracket)?;
            self.expect(Token::Eq)?;
            let value = self.parse_expr()?;
            return Ok(Stmt::AssignIndex { name, index, value });
        }

        // name = value  (reassignment)
        if self.current == Token::Eq {
            self.advance()?;
            let value = self.parse_expr()?;
            return Ok(Stmt::Assign { name, value });
        }

        // name(args)  — function call as statement, result discarded
        if self.current == Token::LParen {
            self.advance()?;
            let args = self.parse_arg_list()?;
            return Ok(Stmt::ExprStmt(Expr::FunctionCall { name, args }));
        }

        // name: type = value  (typed declaration)
        self.expect(Token::Colon)?;
        let ty = self.parse_type()?;
        self.expect(Token::Eq)?;
        let value = self.parse_expr()?;
        Ok(Stmt::VarDecl { name, ty, value })
    }

    fn parse_print(&mut self) -> LiphiaResult<Stmt> {
        self.advance()?;
        self.expect(Token::LParen)?;
        let args = self.parse_arg_list()?;
        Ok(Stmt::Print(args))
    }

    fn parse_while(&mut self) -> LiphiaResult<Stmt> {
        self.advance()?;
        let condition = self.parse_expr()?;
        self.expect(Token::Colon)?;
        self.expect_indent_after_colon()?;
        let body = self.parse_block()?;
        Ok(Stmt::While { condition, body })
    }

    fn parse_for(&mut self) -> LiphiaResult<Stmt> {
        self.advance()?;
        let var = self.take_ident("expected loop variable after 'for'")?;
        self.expect_kw("from")?;
        let from = self.parse_expr()?;
        self.expect_kw("to")?;
        let to = self.parse_expr()?;
        let step = if self.is("step") {
            self.advance()?;
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.expect(Token::Colon)?;
        self.expect_indent_after_colon()?;
        let body = self.parse_block()?;
        Ok(Stmt::For { var, from, to, step, body })
    }

    fn parse_return(&mut self) -> LiphiaResult<Stmt> {
        self.advance()?;
        if matches!(self.current, Token::Newline | Token::EOF | Token::Dedent) {
            return Ok(Stmt::Return(Expr::Null));
        }
        let expr = self.parse_expr()?;
        Ok(Stmt::Return(expr))
    }

    fn parse_if(&mut self) -> LiphiaResult<Stmt> {
        self.advance()?;
        let condition = self.parse_expr()?;
        self.expect(Token::Colon)?;
        self.expect_indent_after_colon()?;
        let block = self.parse_block()?;

        let mut branches   = vec![];
        let mut else_block = None;

        loop {
            self.skip_newlines()?;
            if self.is("elif") {
                self.advance()?;
                let cond = self.parse_expr()?;
                self.expect(Token::Colon)?;
                self.expect_indent_after_colon()?;
                branches.push((cond, self.parse_block()?));
            } else if self.is("else") {
                self.advance()?;
                self.expect(Token::Colon)?;
                self.expect_indent_after_colon()?;
                else_block = Some(self.parse_block()?);
                break;
            } else {
                break;
            }
        }

        Ok(Stmt::If { condition, block, branches, else_block })
    }

    fn parse_block(&mut self) -> LiphiaResult<Vec<Stmt>> {
        let mut stmts = vec![];
        self.skip_newlines()?;
        while self.current != Token::Dedent && self.current != Token::EOF {
            stmts.push(self.parse_stmt()?);
            self.skip_newlines()?;
        }
        if self.current == Token::Dedent {
            self.advance()?;
        }
        Ok(stmts)
    }

    // ── Types ─────────────────────────────────────────────────────────────

    fn parse_type(&mut self) -> LiphiaResult<Type> {
        let base = match &self.current {
            Token::Ident(n) if n == "int"   => { self.advance()?; Type::Int }
            Token::Ident(n) if n == "float" => { self.advance()?; Type::Float }
            Token::Ident(n) if n == "str"   => { self.advance()?; Type::Str }
            Token::Ident(n) if n == "bool"  => { self.advance()?; Type::Bool }
            Token::Ident(n) if n == "void"  => { self.advance()?; Type::Void }
            Token::Ident(n) if n == "list"  => { self.advance()?; Type::List }
            Token::Ident(n) if n == "null"  => { self.advance()?; Type::Null }
            Token::Ident(n) => {
                let name = n.clone();
                self.advance()?;
                Type::Named(name)
            }
            Token::Async => return Err(LiphiaError::new(
                ErrorKind::InvalidType,
                "'async' is a keyword and cannot be used as a type",
            ).at(self.line, self.column)),
            Token::Await => return Err(LiphiaError::new(
                ErrorKind::InvalidType,
                "'await' is a keyword and cannot be used as a type",
            ).at(self.line, self.column)),
            _ => return Err(LiphiaError::new(
                ErrorKind::InvalidType,
                format!("unknown type {:?}", self.current),
            ).at(self.line, self.column)),
        };
        if self.current == Token::Question {
            self.advance()?;
            return Ok(Type::Optional(Box::new(base)));
        }
        Ok(base)
    }

    // ── Expressions ───────────────────────────────────────────────────────

    fn parse_expr(&mut self) -> LiphiaResult<Expr> { self.parse_or() }

    fn parse_or(&mut self) -> LiphiaResult<Expr> {
        let mut expr = self.parse_and()?;
        while self.is("or") || self.current == Token::PipePipe {
            self.advance()?;
            expr = Expr::Or(Box::new(expr), Box::new(self.parse_and()?));
        }
        Ok(expr)
    }

    fn parse_and(&mut self) -> LiphiaResult<Expr> {
        let mut expr = self.parse_comparison()?;
        while self.is("and") || self.current == Token::AmpAmp {
            self.advance()?;
            expr = Expr::And(Box::new(expr), Box::new(self.parse_comparison()?));
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> LiphiaResult<Expr> {
        let mut expr = self.parse_add_sub()?;
        loop {
            match self.current.clone() {
                Token::EqEq  => { self.advance()?; expr = Expr::Eq(Box::new(expr),    Box::new(self.parse_add_sub()?)); }
                Token::NotEq => { self.advance()?; expr = Expr::NotEq(Box::new(expr), Box::new(self.parse_add_sub()?)); }
                Token::Gt    => { self.advance()?; expr = Expr::Gt(Box::new(expr),    Box::new(self.parse_add_sub()?)); }
                Token::Lt    => { self.advance()?; expr = Expr::Lt(Box::new(expr),    Box::new(self.parse_add_sub()?)); }
                Token::Gte   => { self.advance()?; expr = Expr::Gte(Box::new(expr),   Box::new(self.parse_add_sub()?)); }
                Token::Lte   => { self.advance()?; expr = Expr::Lte(Box::new(expr),   Box::new(self.parse_add_sub()?)); }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_add_sub(&mut self) -> LiphiaResult<Expr> {
        let mut expr = self.parse_mul_div()?;
        while self.current == Token::Plus || self.current == Token::Minus {
            let op = self.current.clone();
            self.advance()?;
            let r = self.parse_mul_div()?;
            expr = match op {
                Token::Plus  => Expr::Add(Box::new(expr), Box::new(r)),
                Token::Minus => Expr::Sub(Box::new(expr), Box::new(r)),
                _ => unreachable!(),
            };
        }
        Ok(expr)
    }

    fn parse_mul_div(&mut self) -> LiphiaResult<Expr> {
        let mut expr = self.parse_unary()?;
        while self.current == Token::Star || self.current == Token::Slash {
            let op = self.current.clone();
            self.advance()?;
            let r = self.parse_unary()?;
            expr = match op {
                Token::Star  => Expr::Mul(Box::new(expr), Box::new(r)),
                Token::Slash => Expr::Div(Box::new(expr), Box::new(r)),
                _ => unreachable!(),
            };
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> LiphiaResult<Expr> {
        if self.is("not") || self.current == Token::Bang {
            self.advance()?;
            return Ok(Expr::Not(Box::new(self.parse_unary()?)));
        }
        if self.current == Token::Minus {
            self.advance()?;
            let inner = self.parse_unary()?;
            return Ok(match inner {
                Expr::Int(v)   => Expr::Int(-v),
                Expr::Float(v) => Expr::Float(-v),
                other          => Expr::Sub(Box::new(Expr::Int(0)), Box::new(other)),
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> LiphiaResult<Expr> {
        let mut expr = self.parse_primary()?;
        while self.current == Token::LBracket {
            self.advance()?;
            let index = self.parse_expr()?;
            self.expect(Token::RBracket)?;
            expr = Expr::Index(Box::new(expr), Box::new(index));
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> LiphiaResult<Expr> {
        match &self.current.clone() {
            Token::IntLiteral(n)   => { let v = *n; self.advance()?; Ok(Expr::Int(v)) }
            Token::FloatLiteral(n) => { let v = *n; self.advance()?; Ok(Expr::Float(v)) }
            Token::StrLiteral(t)   => { let v = t.clone(); self.advance()?; Ok(Expr::Str(v)) }
            Token::Ident(n) if n == "true"  => { self.advance()?; Ok(Expr::Bool(true)) }
            Token::Ident(n) if n == "false" => { self.advance()?; Ok(Expr::Bool(false)) }
            Token::Ident(n) if n == "null"  => { self.advance()?; Ok(Expr::Null) }

            // await <expr>
            Token::Await => {
                self.advance()?;
                let inner = self.parse_expr()?;
                Ok(Expr::Await(Box::new(inner)))
            }

            // spawn name(args) as expression
            Token::Ident(n) if n == "spawn" => {
                self.advance()?;
                let name = self.take_ident("expected function name after 'spawn'")?;
                self.expect(Token::LParen)?;
                let args = self.parse_arg_list()?;
                Ok(Expr::Spawn { name, args })
            }

            Token::Ident(_) => {
                let name = self.take_ident("")?;
                if self.current == Token::Dot {
                    self.advance()?;
                    let variant = self.take_ident("expected variant name after '.'")?;
                    return Ok(Expr::EnumVariant { enum_name: name, variant });
                }
                if self.current == Token::LParen {
                    self.advance()?;
                    let args = self.parse_arg_list()?;
                    return Ok(Expr::FunctionCall { name, args });
                }
                Ok(Expr::Variable(name))
            }

            Token::LParen => {
                self.advance()?;
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }

            Token::LBracket => {
                self.advance()?;
                let mut items = vec![];
                while self.current != Token::RBracket {
                    if self.current == Token::EOF {
                        return Err(LiphiaError::new(
                            ErrorKind::UnexpectedToken,
                            "unclosed list literal — missing ']'",
                        ).at(self.line, self.column));
                    }
                    items.push(self.parse_expr()?);
                    if self.current == Token::Comma { self.advance()?; }
                }
                self.expect(Token::RBracket)?;
                Ok(Expr::List(items))
            }

            Token::Async => Err(LiphiaError::new(
                ErrorKind::InvalidExpression,
                "'async' cannot be used as an expression — did you mean 'async fn ...'?",
            ).at(self.line, self.column)),

            _ => Err(LiphiaError::new(
                ErrorKind::InvalidExpression,
                format!("unexpected {:?} in expression", self.current),
            ).at(self.line, self.column)),
        }
    }

    // ── EOF-safe argument list ─────────────────────────────────────
    fn parse_arg_list(&mut self) -> LiphiaResult<Vec<Expr>> {
        let mut args = vec![];
        while self.current != Token::RParen {
            if self.current == Token::EOF {
                return Err(LiphiaError::new(
                    ErrorKind::UnexpectedToken,
                    "unclosed argument list — missing ')'",
                ).at(self.line, self.column));
            }
            args.push(self.parse_expr()?);
            if self.current == Token::Comma { self.advance()?; }
        }
        self.expect(Token::RParen)?;
        Ok(args)
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    fn take_ident(&mut self, msg: &str) -> LiphiaResult<String> {
        match &self.current {
            Token::Ident(name) => {
                let name = name.clone();
                self.advance()?;
                Ok(name)
            }
            Token::Async => Err(LiphiaError::new(
                ErrorKind::ExpectedIdent,
                "'async' is a keyword and cannot be used as an identifier",
            ).at(self.line, self.column)),
            Token::Await => Err(LiphiaError::new(
                ErrorKind::ExpectedIdent,
                "'await' is a keyword and cannot be used as an identifier",
            ).at(self.line, self.column)),
            _ => {
                let m = if msg.is_empty() {
                    format!("expected identifier, found {:?}", self.current)
                } else {
                    format!("{}, found {:?}", msg, self.current)
                };
                Err(LiphiaError::new(ErrorKind::ExpectedIdent, m).at(self.line, self.column))
            }
        }
    }
}