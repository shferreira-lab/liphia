// stdlib/native/src/json.rs
//
// JSON encode / decode — zero external dependencies.
//
// Functions registered:
//   json_encode(value: any)    → str    serialize a Liphia value to JSON
//   json_decode(text: str)     → list   parse JSON object → flat list [k,v,k,v,...]
//   json_get(text: str, key: str) → str  get a string value from a JSON object
//   json_has(text: str, key: str) → bool  check if key exists

use std::cell::RefCell;
use std::rc::Rc;

use liphia_virtual_machine::value::Value;
use liphia_virtual_machine::vm::{VmError, VmResult, VM};

pub fn register(vm: &mut VM) {
    vm.register_native("json_encode", native_json_encode);
    vm.register_native("json_decode", native_json_decode);
    vm.register_native("json_get",    native_json_get);
    vm.register_native("json_has",    native_json_has);
}

// ── Encode ────────────────────────────────────────────────────────────────────

fn native_json_encode(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 1 {
        return Err(VmError::new("json_encode(value) — expected 1 argument"));
    }
    let s = encode_value(&args[0]);
    Ok(Value::Str(Rc::new(s)))
}

fn encode_value(v: &Value) -> String {
    match v {
        Value::Null        => "null".to_string(),
        Value::Bool(b)     => if *b { "true".to_string() } else { "false".to_string() },
        Value::Int(n)      => n.to_string(),
        Value::Float(f)    => {
            // Avoid "NaN" / "Infinity" which are not valid JSON
            if f.is_finite() { format!("{}", f) } else { "null".to_string() }
        }
        Value::Str(s)      => encode_string(s),
        Value::List(rc)    => {
            let items = rc.borrow();
            let parts: Vec<String> = items.iter().map(encode_value).collect();
            format!("[{}]", parts.join(","))
        }
        Value::EnumVariant { variant, .. } => encode_string(variant),
    }
}

fn encode_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"'  => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c    => out.push(c),
        }
    }
    out.push('"');
    out
}

// ── Decode ────────────────────────────────────────────────────────────────────
// Returns a flat list [key, value, key, value ...] for JSON objects.
// Returns a list of values for JSON arrays.
// Returns a single-element list for scalars.

fn native_json_decode(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 1 {
        return Err(VmError::new("json_decode(text: str) — expected 1 argument"));
    }
    let text = match &args[0] {
        Value::Str(s) => s.as_str().to_string(),
        _ => return Err(VmError::new("json_decode: argument must be str")),
    };

    let mut pos = 0usize;
    let chars: Vec<char> = text.chars().collect();
    let val = parse_value(&chars, &mut pos)
        .map_err(|e| VmError::new(format!("json_decode: {}", e)))?;

    Ok(val)
}

// ── json_get(text, key) → str ─────────────────────────────────────────────────

fn native_json_get(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 2 {
        return Err(VmError::new("json_get(text: str, key: str) — expected 2 arguments"));
    }
    let text = match &args[0] {
        Value::Str(s) => s.as_str().to_string(),
        _ => return Err(VmError::new("json_get: text must be str")),
    };
    let key = match &args[1] {
        Value::Str(s) => s.as_str().to_string(),
        _ => return Err(VmError::new("json_get: key must be str")),
    };

    Ok(Value::Str(Rc::new(simple_get(&text, &key))))
}

fn native_json_has(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 2 {
        return Err(VmError::new("json_has(text: str, key: str) — expected 2 arguments"));
    }
    let text = match &args[0] {
        Value::Str(s) => s.as_str().to_string(),
        _ => return Err(VmError::new("json_has: text must be str")),
    };
    let key = match &args[1] {
        Value::Str(s) => s.as_str().to_string(),
        _ => return Err(VmError::new("json_has: key must be str")),
    };

    // Use the naive search: if json_get returns "" it might still be present
    // with value "".  For has() we need to actually check key presence.
    let needle = format!("\"{}\"", key);
    Ok(Value::Bool(text.contains(&needle)))
}

// ── Minimal JSON parser ───────────────────────────────────────────────────────

fn skip_ws(chars: &[char], pos: &mut usize) {
    while *pos < chars.len() && matches!(chars[*pos], ' ' | '\t' | '\n' | '\r') {
        *pos += 1;
    }
}

fn parse_value(chars: &[char], pos: &mut usize) -> Result<Value, String> {
    skip_ws(chars, pos);
    if *pos >= chars.len() {
        return Err("unexpected end of JSON".to_string());
    }
    match chars[*pos] {
        '"' => parse_string_value(chars, pos),
        '{' => parse_object(chars, pos),
        '[' => parse_array(chars, pos),
        't' => {
            if chars[*pos..].starts_with(&['t','r','u','e']) {
                *pos += 4;
                Ok(Value::Bool(true))
            } else {
                Err(format!("invalid token at pos {}", pos))
            }
        }
        'f' => {
            if chars[*pos..].starts_with(&['f','a','l','s','e']) {
                *pos += 5;
                Ok(Value::Bool(false))
            } else {
                Err(format!("invalid token at pos {}", pos))
            }
        }
        'n' => {
            if chars[*pos..].starts_with(&['n','u','l','l']) {
                *pos += 4;
                Ok(Value::Null)
            } else {
                Err(format!("invalid token at pos {}", pos))
            }
        }
        '-' | '0'..='9' => parse_number(chars, pos),
        c => Err(format!("unexpected character '{}' at pos {}", c, pos)),
    }
}

fn parse_string_value(chars: &[char], pos: &mut usize) -> Result<Value, String> {
    Ok(Value::Str(Rc::new(parse_string(chars, pos)?)))
}

fn parse_string(chars: &[char], pos: &mut usize) -> Result<String, String> {
    if chars[*pos] != '"' {
        return Err(format!("expected '\"' at pos {}", pos));
    }
    *pos += 1;
    let mut s = String::new();
    loop {
        if *pos >= chars.len() {
            return Err("unterminated string".to_string());
        }
        match chars[*pos] {
            '"' => { *pos += 1; return Ok(s); }
            '\\' => {
                *pos += 1;
                if *pos >= chars.len() {
                    return Err("unexpected end after backslash".to_string());
                }
                match chars[*pos] {
                    '"'  => s.push('"'),
                    '\\' => s.push('\\'),
                    '/'  => s.push('/'),
                    'n'  => s.push('\n'),
                    'r'  => s.push('\r'),
                    't'  => s.push('\t'),
                    'b'  => s.push('\x08'),
                    'f'  => s.push('\x0c'),
                    'u'  => {
                        *pos += 1;
                        if *pos + 4 > chars.len() {
                            return Err("incomplete \\u escape".to_string());
                        }
                        let hex: String = chars[*pos..*pos+4].iter().collect();
                        let code = u32::from_str_radix(&hex, 16)
                            .map_err(|_| format!("invalid \\u{}", hex))?;
                        s.push(char::from_u32(code).unwrap_or('?'));
                        *pos += 3; // +1 below
                    }
                    c => { s.push('\\'); s.push(c); }
                }
                *pos += 1;
            }
            c => { s.push(c); *pos += 1; }
        }
    }
}

fn parse_number(chars: &[char], pos: &mut usize) -> Result<Value, String> {
    let start = *pos;
    if *pos < chars.len() && chars[*pos] == '-' { *pos += 1; }
    while *pos < chars.len() && chars[*pos].is_ascii_digit() { *pos += 1; }
    let mut is_float = false;
    if *pos < chars.len() && chars[*pos] == '.' {
        is_float = true;
        *pos += 1;
        while *pos < chars.len() && chars[*pos].is_ascii_digit() { *pos += 1; }
    }
    if *pos < chars.len() && matches!(chars[*pos], 'e' | 'E') {
        is_float = true;
        *pos += 1;
        if *pos < chars.len() && matches!(chars[*pos], '+' | '-') { *pos += 1; }
        while *pos < chars.len() && chars[*pos].is_ascii_digit() { *pos += 1; }
    }
    let s: String = chars[start..*pos].iter().collect();
    if is_float {
        Ok(Value::Float(s.parse().map_err(|_| format!("invalid float '{}'", s))?))
    } else {
        Ok(Value::Int(s.parse().map_err(|_| format!("invalid int '{}'", s))?))
    }
}

/// JSON object → flat list [key, value, key, value ...]
fn parse_object(chars: &[char], pos: &mut usize) -> Result<Value, String> {
    *pos += 1; // consume '{'
    let mut items: Vec<Value> = vec![];
    skip_ws(chars, pos);
    if *pos < chars.len() && chars[*pos] == '}' {
        *pos += 1;
        return Ok(Value::List(Rc::new(RefCell::new(items))));
    }
    loop {
        skip_ws(chars, pos);
        let key = parse_string(chars, pos)?;
        items.push(Value::Str(Rc::new(key)));
        skip_ws(chars, pos);
        if *pos >= chars.len() || chars[*pos] != ':' {
            return Err(format!("expected ':' at pos {}", pos));
        }
        *pos += 1;
        let val = parse_value(chars, pos)?;
        items.push(val);
        skip_ws(chars, pos);
        match chars.get(*pos) {
            Some(',') => { *pos += 1; }
            Some('}') => { *pos += 1; break; }
            _ => return Err(format!("expected ',' or '}}' at pos {}", pos)),
        }
    }
    Ok(Value::List(Rc::new(RefCell::new(items))))
}

fn parse_array(chars: &[char], pos: &mut usize) -> Result<Value, String> {
    *pos += 1; // consume '['
    let mut items: Vec<Value> = vec![];
    skip_ws(chars, pos);
    if *pos < chars.len() && chars[*pos] == ']' {
        *pos += 1;
        return Ok(Value::List(Rc::new(RefCell::new(items))));
    }
    loop {
        let val = parse_value(chars, pos)?;
        items.push(val);
        skip_ws(chars, pos);
        match chars.get(*pos) {
            Some(',') => { *pos += 1; }
            Some(']') => { *pos += 1; break; }
            _ => return Err(format!("expected ',' or ']' at pos {}", pos)),
        }
    }
    Ok(Value::List(Rc::new(RefCell::new(items))))
}

// ── Simple string-based key lookup (no full parse needed) ─────────────────────
// For quick access to known string fields in typical REST payloads.

fn simple_get(text: &str, key: &str) -> String {
    let needle = format!("\"{}\"", key);
    let pos = match text.find(&needle) {
        Some(p) => p + needle.len(),
        None    => return String::new(),
    };
    let rest = text[pos..].trim_start();
    let rest = if rest.starts_with(':') { rest[1..].trim_start() } else { return String::new() };

    if rest.starts_with('"') {
        // String value
        let inner = &rest[1..];
        let mut s = String::new();
        let mut escaped = false;
        for c in inner.chars() {
            if escaped {
                match c {
                    'n'  => s.push('\n'),
                    't'  => s.push('\t'),
                    '"'  => s.push('"'),
                    '\\' => s.push('\\'),
                    c    => { s.push('\\'); s.push(c); }
                }
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                break;
            } else {
                s.push(c);
            }
        }
        s
    } else {
        // Number, bool, null — take until separator
        rest.chars()
            .take_while(|&c| !matches!(c, ',' | '}' | ']' | ' ' | '\n' | '\r' | '\t'))
            .collect()
    }
}
