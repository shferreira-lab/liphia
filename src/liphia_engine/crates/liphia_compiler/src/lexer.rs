// liphia_compiler/src/lexer.rs


use crate::error::{ErrorKind, LiphiaError, LiphiaResult};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ident(String),
    IntLiteral(i64),
    FloatLiteral(f64),
    StrLiteral(String),
    FStrLiteral(String),
    // Punctuation
    Colon,
    Eq,
    Arrow,
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Question,
    Dot,
    // Comparison
    EqEq,
    NotEq,
    Gt,
    Lt,
    Gte,
    Lte,
    // Logical
    AmpAmp,
    PipePipe,
    Bang,
    // async await
    Async,
    Await,
    // Indentation
    Indent,
    Dedent,
    Newline,
    EOF,
}

pub struct Lexer {
    lines:           Vec<Vec<char>>,
    current_line:    usize,
    pos:             usize,
    indent_stack:    Vec<usize>,
    pending_dedents: usize,
    emit_newline:    bool,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        let mut lines: Vec<Vec<char>> = source
            .lines()
            .map(|l| l.chars().collect())
            .collect();
        lines.push(vec![]);
        Self {
            lines,
            current_line: 0,
            pos: 0,
            indent_stack: vec![0],
            pending_dedents: 0,
            emit_newline: false,
        }
    }

    pub fn line(&self)   -> usize { self.current_line + 1 }
    pub fn column(&self) -> usize { self.pos + 1 }

    fn chars(&self) -> &Vec<char> { &self.lines[self.current_line] }
    fn cur(&self)  -> Option<char> { self.chars().get(self.pos).copied() }
    fn peek(&self) -> Option<char> { self.chars().get(self.pos + 1).copied() }
    fn advance(&mut self) { self.pos += 1; }
    fn advance_line(&mut self) {
        self.current_line += 1;
        self.pos = 0;
        self.emit_newline = false;
    }

    fn count_indent(line: &[char]) -> usize {
        let mut n = 0;
        for c in line {
            match c {
                ' '  => n += 1,
                '\t' => n += 4,
                _    => break,
            }
        }
        n
    }

    fn is_blank_or_comment(line: &[char]) -> bool {
        let s: String = line.iter().collect();
        let t = s.trim();
        t.is_empty() || t.starts_with('#')
    }

    /// Returns the next token along with the (line, column) where that
    /// token actually starts. Callers must use THIS position for the
    /// token, not query the lexer's position separately afterward.
    pub fn next_token(&mut self) -> LiphiaResult<(Token, usize, usize)> {
        if self.pending_dedents > 0 {
            self.pending_dedents -= 1;
            let (l, c) = (self.line(), self.column());
            return Ok((Token::Dedent, l, c));
        }
        if self.emit_newline {
            self.emit_newline = false;
            let (l, c) = (self.line(), self.column());
            return Ok((Token::Newline, l, c));
        }
        if self.current_line >= self.lines.len() {
            if self.indent_stack.len() > 1 {
                self.indent_stack.pop();
                let (l, c) = (self.line(), self.column());
                return Ok((Token::Dedent, l, c));
            }
            let (l, c) = (self.line(), self.column());
            return Ok((Token::EOF, l, c));
        }

        if self.pos == 0 {
            let line = self.chars().clone();
            if Self::is_blank_or_comment(&line) {
                let (l, c) = (self.line(), self.column());
                self.advance_line();
                return Ok((Token::Newline, l, c));
            }
            let indent = Self::count_indent(&line);
            let top    = *self.indent_stack.last().unwrap();
            if indent > top {
                let (l, c) = (self.line(), 1);
                self.indent_stack.push(indent);
                self.pos = indent;
                return Ok((Token::Indent, l, c));
            }
            if indent < top {
                let (l, c) = (self.line(), 1);
                while self.indent_stack.last().copied().unwrap_or(0) > indent {
                    self.indent_stack.pop();
                    self.pending_dedents += 1;
                }
                self.pos = indent;
                if self.pending_dedents > 0 {
                    self.pending_dedents -= 1;
                    return Ok((Token::Dedent, l, c));
                }
            }
            self.pos = indent;
        }

        while let Some(c) = self.cur() {
            if matches!(c, ' ' | '\t' | '\r') {
                self.advance();
                continue;
            }

            let (tok_line, tok_col) = (self.line(), self.column());

            match c {
                '#' => {
                    self.emit_newline = true;
                    self.advance_line();
                    return Ok((Token::Newline, tok_line, tok_col));
                }
                ':' => { self.advance(); return Ok((Token::Colon, tok_line, tok_col)); }
                ',' => { self.advance(); return Ok((Token::Comma, tok_line, tok_col)); }
                '.' => { self.advance(); return Ok((Token::Dot, tok_line, tok_col)); }
                '(' => { self.advance(); return Ok((Token::LParen, tok_line, tok_col)); }
                ')' => { self.advance(); return Ok((Token::RParen, tok_line, tok_col)); }
                '[' => { self.advance(); return Ok((Token::LBracket, tok_line, tok_col)); }
                ']' => { self.advance(); return Ok((Token::RBracket, tok_line, tok_col)); }
                '{' => { self.advance(); return Ok((Token::LBrace, tok_line, tok_col)); }
                '}' => { self.advance(); return Ok((Token::RBrace, tok_line, tok_col)); }
                '+' => { self.advance(); return Ok((Token::Plus, tok_line, tok_col)); }
                '*' => { self.advance(); return Ok((Token::Star, tok_line, tok_col)); }
                '/' => { self.advance(); return Ok((Token::Slash, tok_line, tok_col)); }
                '?' => { self.advance(); return Ok((Token::Question, tok_line, tok_col)); }
                '-' => {
                    if self.peek() == Some('>') {
                        self.advance(); self.advance();
                        return Ok((Token::Arrow, tok_line, tok_col));
                    }
                    self.advance();
                    return Ok((Token::Minus, tok_line, tok_col));
                }
                '=' => {
                    if self.peek() == Some('=') {
                        self.advance(); self.advance();
                        return Ok((Token::EqEq, tok_line, tok_col));
                    }
                    self.advance();
                    return Ok((Token::Eq, tok_line, tok_col));
                }
                '!' => {
                    if self.peek() == Some('=') {
                        self.advance(); self.advance();
                        return Ok((Token::NotEq, tok_line, tok_col));
                    }
                    self.advance();
                    return Ok((Token::Bang, tok_line, tok_col));
                }
                '&' => {
                    if self.peek() == Some('&') {
                        self.advance(); self.advance();
                        return Ok((Token::AmpAmp, tok_line, tok_col));
                    }
                    self.advance();
                    return Err(LiphiaError::new(
                        ErrorKind::UnexpectedChar,
                        "single '&' is not valid — did you mean '&&'?",
                    ).at(tok_line, tok_col));
                }
                '|' => {
                    if self.peek() == Some('|') {
                        self.advance(); self.advance();
                        return Ok((Token::PipePipe, tok_line, tok_col));
                    }
                    self.advance();
                    return Err(LiphiaError::new(
                        ErrorKind::UnexpectedChar,
                        "single '|' is not valid — did you mean '||'?",
                    ).at(tok_line, tok_col));
                }
                '>' => {
                    if self.peek() == Some('=') {
                        self.advance(); self.advance();
                        return Ok((Token::Gte, tok_line, tok_col));
                    }
                    self.advance();
                    return Ok((Token::Gt, tok_line, tok_col));
                }
                '<' => {
                    if self.peek() == Some('=') {
                        self.advance(); self.advance();
                        return Ok((Token::Lte, tok_line, tok_col));
                    }
                    self.advance();
                    return Ok((Token::Lt, tok_line, tok_col));
                }
                // f"..."  — f-string literal. Only triggers when 'f' is
                // immediately followed by '"' (no space), so identifiers
                // like `for`, `foo`, or a variable named `f` used normally
                // are unaffected.
                'f' if self.peek() == Some('"') => {
                    self.advance(); // consume 'f'
                    match self.read_string()? {
                        Token::StrLiteral(s) => return Ok((Token::FStrLiteral(s), tok_line, tok_col)),
                        _ => unreachable!(),
                    }
                }
                '"' => {
                    let tok = self.read_string()?;
                    return Ok((tok, tok_line, tok_col));
                }
                _ => {
                    if c.is_ascii_digit() {
                        let tok = self.read_number()?;
                        return Ok((tok, tok_line, tok_col));
                    }
                    if c.is_alphabetic() || c == '_' {
                        let tok = self.read_ident();
                        return Ok((tok, tok_line, tok_col));
                    }
                    self.advance();
                    return Err(LiphiaError::new(
                        ErrorKind::UnexpectedChar,
                        format!("unexpected character '{}'", c),
                    ).at(tok_line, tok_col));
                }
            }
        }

        let (l, c) = (self.line(), self.column());
        self.advance_line();
        Ok((Token::Newline, l, c))
    }

    fn read_string(&mut self) -> LiphiaResult<Token> {
        let (sl, sc) = (self.line(), self.column());
        self.advance();
        let mut text = String::new();
        loop {
            match self.cur() {
                Some('"')  => { self.advance(); return Ok(Token::StrLiteral(text)); }
                Some('\\') => {
                    self.advance();
                    match self.cur() {
                        Some('n')  => { text.push('\n'); self.advance(); }
                        Some('t')  => { text.push('\t'); self.advance(); }
                        Some('r')  => { text.push('\r'); self.advance(); }
                        Some('\\') => { text.push('\\'); self.advance(); }
                        Some('"')  => { text.push('"');  self.advance(); }
                        Some('0')  => { text.push('\0'); self.advance(); }
                        Some(c) => { text.push('\\'); text.push(c); self.advance(); }
                        None => return Err(LiphiaError::new(
                            ErrorKind::UnterminatedString,
                            "string ends after backslash",
                        ).at(sl, sc)),
                    }
                }
                Some(c)   => { text.push(c); self.advance(); }
                None      => return Err(LiphiaError::new(
                    ErrorKind::UnterminatedString,
                    "string was opened but never closed",
                ).at(sl, sc)),
            }
        }
    }

    fn read_ident(&mut self) -> Token {
        let mut name = String::new();
        while let Some(c) = self.cur() {
            if c.is_alphanumeric() || c == '_' { name.push(c); self.advance(); }
            else { break; }
        }
        match name.as_str() {
            "async" => Token::Async,
            "await" => Token::Await,
            _       => Token::Ident(name),
        }
    }

    fn read_number(&mut self) -> LiphiaResult<Token> {
        let mut s = String::new();
        let mut has_dot = false;
        while let Some(c) = self.cur() {
            if c.is_ascii_digit()            { s.push(c); self.advance(); }
            else if c == '.' && !has_dot     { has_dot = true; s.push(c); self.advance(); }
            else                             { break; }
        }
        if has_dot { Ok(Token::FloatLiteral(s.parse().unwrap_or(0.0))) }
        else       { Ok(Token::IntLiteral(s.parse().unwrap_or(0))) }
    }
}