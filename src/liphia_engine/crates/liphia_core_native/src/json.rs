// liphia_core_native/src/json.rs
//
// JSON encode / decode, no external dependencies.
//
// Functions registered:
//   json_encode(value: any) -> str   serialize a Liphia value
//   json_decode(text: str)  -> any   object -> map, array -> list, scalars as-is
//
// Mapping:
//   null <-> null, true/false <-> bool, string <-> str
//   integer literal (no '.', no exponent) <-> int
//   any other number <-> float (always encoded with a '.' or exponent, so a
//   float round-trips as float and never comes back as int)
//   array <-> list, object <-> map (str keys; other key types are encoded
//   through their text form)
//   enum variant -> its variant name as a string
//   NaN / infinity -> null (JSON has no representation for them)
//
// Decoding is strict: trailing content after the value is an error, and a
// duplicated object key keeps the last value.
//
// `encode` and `decode` are also used by the composed natives of fs, net
// and ws (read_json, tcp_send_json, ws_send_json...).

use std::cell::RefCell;
use std::rc::Rc;

use liphia_virtual_machine::value::Value;
use liphia_virtual_machine::vm::{VmError, VmResult, VM};

use crate::util::{expect_args, str_arg, str_value};

pub fn register(vm: &mut VM) {
    vm.register_native("json_encode", native_json_encode);
    vm.register_native("json_decode", native_json_decode);
}

fn native_json_encode(args: Vec<Value>) -> VmResult<Value> {
    expect_args("json_encode", &args, 1)?;
    Ok(str_value(encode(&args[0])))
}

fn native_json_decode(args: Vec<Value>) -> VmResult<Value> {
    expect_args("json_decode", &args, 1)?;
    let text = str_arg(&args[0], "json_decode")?;
    decode(&text).map_err(|e| VmError::new(format!("json_decode(): {}", e)))
}

// ── Encode ────────────────────────────────────────────────────────────────────

pub(crate) fn encode(value: &Value) -> String {
    let mut out = String::new();
    encode_into(value, &mut out);
    out
}

fn encode_into(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Int(n) => out.push_str(&n.to_string()),
        Value::Float(f) => encode_float(*f, out),
        Value::Str(s) => encode_string(s, out),
        Value::List(rc) => {
            out.push('[');
            for (i, item) in rc.borrow().iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                encode_into(item, out);
            }
            out.push(']');
        }
        Value::Map(rc) => {
            out.push('{');
            for (i, (k, v)) in rc.borrow().iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                match k {
                    Value::Str(s) => encode_string(s, out),
                    other => encode_string(&other.to_string(), out),
                }
                out.push(':');
                encode_into(v, out);
            }
            out.push('}');
        }
        Value::EnumVariant { variant, .. } => encode_string(variant, out),
    }
}

// Debug formatting of f64 always keeps a '.' or an exponent ("1.0",
// "1e20"), which is what makes floats round-trip as floats.
fn encode_float(f: f64, out: &mut String) {
    if f.is_finite() {
        out.push_str(&format!("{:?}", f));
    } else {
        out.push_str("null");
    }
}

fn encode_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

// ── Decode ────────────────────────────────────────────────────────────────────

pub(crate) fn decode(text: &str) -> Result<Value, String> {
    let mut parser = Parser { chars: text.chars().collect(), pos: 0 };
    let value = parser.value()?;
    parser.skip_ws();
    if parser.pos < parser.chars.len() {
        return Err(format!("unexpected content after value at pos {}", parser.pos));
    }
    Ok(value)
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
            self.pos += 1;
        }
    }

    fn literal(&mut self, word: &str, value: Value) -> Result<Value, String> {
        let end = self.pos + word.len();
        if end <= self.chars.len() && self.chars[self.pos..end].iter().copied().eq(word.chars()) {
            self.pos = end;
            Ok(value)
        } else {
            Err(format!("invalid token at pos {}", self.pos))
        }
    }

    fn value(&mut self) -> Result<Value, String> {
        self.skip_ws();
        match self.peek() {
            None => Err("unexpected end of JSON".to_string()),
            Some('"') => Ok(Value::Str(Rc::new(self.string()?))),
            Some('{') => self.object(),
            Some('[') => self.array(),
            Some('t') => self.literal("true", Value::Bool(true)),
            Some('f') => self.literal("false", Value::Bool(false)),
            Some('n') => self.literal("null", Value::Null),
            Some('-' | '0'..='9') => self.number(),
            Some(c) => Err(format!("unexpected character '{}' at pos {}", c, self.pos)),
        }
    }

    fn hex4(&mut self) -> Result<u32, String> {
        if self.pos + 4 > self.chars.len() {
            return Err("incomplete \\u escape".to_string());
        }
        let hex: String = self.chars[self.pos..self.pos + 4].iter().collect();
        self.pos += 4;
        u32::from_str_radix(&hex, 16).map_err(|_| format!("invalid \\u{}", hex))
    }

    // Reads a \u escape (the "\u" already consumed). A high surrogate must
    // be followed by "\u" + low surrogate; together they form one char.
    fn unicode_escape(&mut self) -> Result<char, String> {
        let first = self.hex4()?;
        let code = if (0xD800..0xDC00).contains(&first) {
            if self.peek() != Some('\\') || self.chars.get(self.pos + 1) != Some(&'u') {
                return Err("unpaired surrogate in \\u escape".to_string());
            }
            self.pos += 2;
            let second = self.hex4()?;
            if !(0xDC00..0xE000).contains(&second) {
                return Err("invalid low surrogate in \\u escape".to_string());
            }
            0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00)
        } else {
            first
        };
        char::from_u32(code).ok_or_else(|| format!("invalid code point {:x}", code))
    }

    fn string(&mut self) -> Result<String, String> {
        if self.peek() != Some('"') {
            return Err(format!("expected '\"' at pos {}", self.pos));
        }
        self.pos += 1;
        let mut s = String::new();
        loop {
            let c = self.peek().ok_or("unterminated string")?;
            self.pos += 1;
            match c {
                '"' => return Ok(s),
                '\\' => {
                    let esc = self.peek().ok_or("unexpected end after backslash")?;
                    self.pos += 1;
                    match esc {
                        '"' => s.push('"'),
                        '\\' => s.push('\\'),
                        '/' => s.push('/'),
                        'n' => s.push('\n'),
                        'r' => s.push('\r'),
                        't' => s.push('\t'),
                        'b' => s.push('\x08'),
                        'f' => s.push('\x0c'),
                        'u' => s.push(self.unicode_escape()?),
                        other => return Err(format!("invalid escape '\\{}'", other)),
                    }
                }
                c => s.push(c),
            }
        }
    }

    fn digits(&mut self) -> usize {
        let start = self.pos;
        while matches!(self.peek(), Some('0'..='9')) {
            self.pos += 1;
        }
        self.pos - start
    }

    fn number(&mut self) -> Result<Value, String> {
        let start = self.pos;
        if self.peek() == Some('-') {
            self.pos += 1;
        }
        if self.digits() == 0 {
            return Err(format!("invalid number at pos {}", start));
        }
        let mut is_float = false;
        if self.peek() == Some('.') {
            is_float = true;
            self.pos += 1;
            if self.digits() == 0 {
                return Err(format!("invalid number at pos {}", start));
            }
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            is_float = true;
            self.pos += 1;
            if matches!(self.peek(), Some('+' | '-')) {
                self.pos += 1;
            }
            if self.digits() == 0 {
                return Err(format!("invalid number at pos {}", start));
            }
        }
        let s: String = self.chars[start..self.pos].iter().collect();
        if is_float {
            s.parse().map(Value::Float).map_err(|_| format!("invalid float '{}'", s))
        } else {
            s.parse()
                .map(Value::Int)
                .map_err(|_| format!("integer '{}' does not fit in int", s))
        }
    }

    fn object(&mut self) -> Result<Value, String> {
        self.pos += 1;
        let mut pairs: Vec<(Value, Value)> = vec![];
        self.skip_ws();
        if self.peek() == Some('}') {
            self.pos += 1;
            return Ok(Value::Map(Rc::new(RefCell::new(pairs))));
        }
        loop {
            self.skip_ws();
            let key = Value::Str(Rc::new(self.string()?));
            self.skip_ws();
            if self.peek() != Some(':') {
                return Err(format!("expected ':' at pos {}", self.pos));
            }
            self.pos += 1;
            let value = self.value()?;
            match pairs.iter_mut().find(|(k, _)| *k == key) {
                Some(slot) => slot.1 = value,
                None => pairs.push((key, value)),
            }
            self.skip_ws();
            match self.peek() {
                Some(',') => self.pos += 1,
                Some('}') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(format!("expected ',' or '}}' at pos {}", self.pos)),
            }
        }
        Ok(Value::Map(Rc::new(RefCell::new(pairs))))
    }

    fn array(&mut self) -> Result<Value, String> {
        self.pos += 1;
        let mut items: Vec<Value> = vec![];
        self.skip_ws();
        if self.peek() == Some(']') {
            self.pos += 1;
            return Ok(Value::List(Rc::new(RefCell::new(items))));
        }
        loop {
            items.push(self.value()?);
            self.skip_ws();
            match self.peek() {
                Some(',') => self.pos += 1,
                Some(']') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(format!("expected ',' or ']' at pos {}", self.pos)),
            }
        }
        Ok(Value::List(Rc::new(RefCell::new(items))))
    }
}
