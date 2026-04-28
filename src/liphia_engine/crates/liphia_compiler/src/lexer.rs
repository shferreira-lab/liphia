// liphia_compiler/src/lexer.rs
use crate::error::{ErrorKind, LiphiaError, LiphiaResult};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ident(String),
    IntLiteral(i64),
    FloatLiteral(f64),
    StrLiteral(String),
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
    // ── NEW ──────────────────────────────────────────────────────────────
    Async,   // async
    Await,   // await
    // ─────────────────────────────────────────────────────────────────────
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

    pub fn line(&self) -> usize   { self.current_line + 1 }
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

    pub fn next_token(&mut self) -> LiphiaResult<Token> {
        if self.pending_dedents > 0 {
            self.pending_dedents -= 1;
            return Ok(Token::Dedent);
        }
        if self.emit_newline {
            self.emit_newline = false;
            return Ok(Token::Newline);
        }
        if self.current_line >= self.lines.len() {
            if self.indent_stack.len() > 1 {
                self.indent_stack.pop();
                return Ok(Token::Dedent);
            }
            return Ok(Token::EOF);
        }

        if self.pos == 0 {
            let line = self.chars().clone();
            if Self::is_blank_or_comment(&line) {
                self.advance_line();
                return Ok(Token::Newline);
            }
            let indent = Self::count_indent(&line);
            let top    = *self.indent_stack.last().unwrap();
            if indent > top {
                self.indent_stack.push(indent);
                self.pos = indent;
                return Ok(Token::Indent);
            }
            if indent < top {
                while self.indent_stack.last().copied().unwrap_or(0) > indent {
                    self.indent_stack.pop();
                    self.pending_dedents += 1;
                }
                self.pos = indent;
                if self.pending_dedents > 0 {
                    self.pending_dedents -= 1;
                    return Ok(Token::Dedent);
                }
            }
            self.pos = indent;
        }

        while let Some(c) = self.cur() {
            match c {
                ' ' | '\t' | '\r' => { self.advance(); }
                '#' => {
                    self.emit_newline = true;
                    self.advance_line();
                    return Ok(Token::Newline);
                }
                ':' => { self.advance(); return Ok(Token::Colon); }
                ',' => { self.advance(); return Ok(Token::Comma); }
                '.' => { self.advance(); return Ok(Token::Dot); }
                '(' => { self.advance(); return Ok(Token::LParen); }
                ')' => { self.advance(); return Ok(Token::RParen); }
                '[' => { self.advance(); return Ok(Token::LBracket); }
                ']' => { self.advance(); return Ok(Token::RBracket); }
                '+' => { self.advance(); return Ok(Token::Plus); }
                '*' => { self.advance(); return Ok(Token::Star); }
                '/' => { self.advance(); return Ok(Token::Slash); }
                '?' => { self.advance(); return Ok(Token::Question); }
                '-' => {
                    if self.peek() == Some('>') {
                        self.advance(); self.advance();
                        return Ok(Token::Arrow);
                    }
                    self.advance();
                    return Ok(Token::Minus);
                }
                '=' => {
                    if self.peek() == Some('=') {
                        self.advance(); self.advance();
                        return Ok(Token::EqEq);
                    }
                    self.advance();
                    return Ok(Token::Eq);
                }
                '!' => {
                    if self.peek() == Some('=') {
                        self.advance(); self.advance();
                        return Ok(Token::NotEq);
                    }
                    self.advance();
                    return Ok(Token::Bang);
                }
                '&' => {
                    if self.peek() == Some('&') {
                        self.advance(); self.advance();
                        return Ok(Token::AmpAmp);
                    }
                    let (l, c2) = (self.line(), self.column());
                    self.advance();
                    return Err(LiphiaError::new(
                        ErrorKind::UnexpectedChar,
                        "single '&' is not valid — did you mean '&&'?",
                    ).at(l, c2));
                }
                '|' => {
                    if self.peek() == Some('|') {
                        self.advance(); self.advance();
                        return Ok(Token::PipePipe);
                    }
                    let (l, c2) = (self.line(), self.column());
                    self.advance();
                    return Err(LiphiaError::new(
                        ErrorKind::UnexpectedChar,
                        "single '|' is not valid — did you mean '||'?",
                    ).at(l, c2));
                }
                '>' => {
                    if self.peek() == Some('=') {
                        self.advance(); self.advance();
                        return Ok(Token::Gte);
                    }
                    self.advance();
                    return Ok(Token::Gt);
                }
                '<' => {
                    if self.peek() == Some('=') {
                        self.advance(); self.advance();
                        return Ok(Token::Lte);
                    }
                    self.advance();
                    return Ok(Token::Lt);
                }
                '"' => return self.read_string(),
                _ => {
                    if c.is_ascii_digit() { return self.read_number(); }
                    if c.is_alphabetic() || c == '_' { return Ok(self.read_ident()); }
                    let (l, col) = (self.line(), self.column());
                    self.advance();
                    return Err(LiphiaError::new(
                        ErrorKind::UnexpectedChar,
                        format!("unexpected character '{}'", c),
                    ).at(l, col));
                }
            }
        }

        self.advance_line();
        Ok(Token::Newline)
    }

    fn read_string(&mut self) -> LiphiaResult<Token> {
        let (sl, sc) = (self.line(), self.column());
        self.advance(); // opening "
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
        // ── Keyword dispatch — async/await added here ─────────────────────
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
