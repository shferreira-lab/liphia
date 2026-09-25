// liphia_bytecode/src/lib.rs
//
// LBC — the Liphia bytecode file format. This crate is the single source
// of truth for how a compiled program is written to and read from bytes.
// Any VM implementation (the Rust VM today, others later) only needs to
// understand this format, not the compiler. The normative description
// lives in docs/spec/LBC.md; keep both in sync.
//
// Layout (all integers little-endian):
//
//   magic          4 bytes   "LBC\0"
//   format_version u32
//   source_hash    u64       FNV-1a of the resolved AST (cache validation)
//   op_count       u32
//   opcodes        op_count x (tag u8 + operands)

use std::fmt;

use liphia_virtual_machine::opcode::Opcode;

pub const MAGIC: &[u8; 4] = b"LBC\0";
pub const FORMAT_VERSION: u32 = 4;

// Size of the fixed header: magic + version + hash + op_count.
const HEADER_LEN: usize = 4 + 4 + 8 + 4;

// ── Opcode tags ───────────────────────────────────────────────────────────────
// One byte per instruction kind. Values are part of the format: changing
// or reusing a tag requires bumping FORMAT_VERSION.

pub mod tag {
    pub const PUSH_INT: u8 = 0x01;
    pub const PUSH_FLOAT: u8 = 0x02;
    pub const PUSH_STRING: u8 = 0x03;
    pub const PUSH_BOOL: u8 = 0x04;
    pub const PUSH_NULL: u8 = 0x05;
    pub const PUSH_ENUM: u8 = 0x06;

    pub const LOAD_VAR: u8 = 0x10;
    pub const STORE_VAR: u8 = 0x11;
    pub const LOAD_GLOBAL: u8 = 0x12;
    pub const STORE_GLOBAL: u8 = 0x13;

    pub const ADD: u8 = 0x20;
    pub const SUB: u8 = 0x21;
    pub const MUL: u8 = 0x22;
    pub const DIV: u8 = 0x23;
    pub const EQ: u8 = 0x24;
    pub const NEQ: u8 = 0x25;
    pub const GT: u8 = 0x26;
    pub const LT: u8 = 0x27;
    pub const GTE: u8 = 0x28;
    pub const LTE: u8 = 0x29;
    pub const AND: u8 = 0x2A;
    pub const OR: u8 = 0x2B;
    pub const NOT: u8 = 0x2C;

    pub const INPUT: u8 = 0x30;
    pub const PRINT: u8 = 0x31;

    pub const JUMP: u8 = 0x40;
    pub const JUMP_IF_FALSE: u8 = 0x41;

    pub const CALL_NAMED: u8 = 0x50;
    pub const CALL: u8 = 0x51;
    pub const RETURN: u8 = 0x52;

    pub const BUILD_LIST: u8 = 0x60;
    pub const GET_INDEX: u8 = 0x61;
    pub const SET_INDEX: u8 = 0x62;
    pub const POP: u8 = 0x63;
    pub const BUILD_MAP: u8 = 0x64;

    pub const SUSPEND: u8 = 0x70;
    pub const SPAWN: u8 = 0x71;

    pub const PUSH_HANDLER: u8 = 0x80;
    pub const POP_HANDLER: u8 = 0x81;

    pub const HALT: u8 = 0xFF;
}

// ── Public types ──────────────────────────────────────────────────────────────

/// A decoded LBC file.
#[derive(Debug, Clone)]
pub struct Program {
    pub source_hash: u64,
    pub opcodes: Vec<Opcode>,
}

/// Why a byte buffer could not be decoded. `at` is a byte offset.
#[derive(Debug, Clone, PartialEq)]
pub enum DecodeError {
    BadMagic,
    UnsupportedVersion(u32),
    Truncated { at: usize },
    UnknownOpcode { tag: u8, at: usize },
    InvalidUtf8 { at: usize },
    TrailingBytes { at: usize },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::BadMagic => write!(f, "not an LBC file (bad magic)"),
            DecodeError::UnsupportedVersion(v) => write!(
                f,
                "unsupported LBC format version {} (this build reads {})",
                v, FORMAT_VERSION
            ),
            DecodeError::Truncated { at } => write!(f, "file truncated at byte {}", at),
            DecodeError::UnknownOpcode { tag, at } => {
                write!(f, "unknown opcode tag 0x{:02X} at byte {}", tag, at)
            }
            DecodeError::InvalidUtf8 { at } => write!(f, "invalid UTF-8 string at byte {}", at),
            DecodeError::TrailingBytes { at } => {
                write!(f, "unexpected bytes after last opcode at byte {}", at)
            }
        }
    }
}

impl std::error::Error for DecodeError {}

// ── Encoding ──────────────────────────────────────────────────────────────────

/// Serializes a compiled program into LBC bytes.
pub fn encode(source_hash: u64, opcodes: &[Opcode]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(HEADER_LEN + opcodes.len() * 4);
    buf.extend_from_slice(MAGIC);
    buf.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    buf.extend_from_slice(&source_hash.to_le_bytes());
    buf.extend_from_slice(&(opcodes.len() as u32).to_le_bytes());
    for op in opcodes {
        write_opcode(&mut buf, op);
    }
    buf
}

fn write_opcode(buf: &mut Vec<u8>, op: &Opcode) {
    use tag::*;
    match op {
        Opcode::PushInt(v) => { buf.push(PUSH_INT); buf.extend(v.to_le_bytes()); }
        Opcode::PushFloat(v) => { buf.push(PUSH_FLOAT); buf.extend(v.to_le_bytes()); }
        Opcode::PushString(s) => { buf.push(PUSH_STRING); write_str(buf, s); }
        Opcode::PushBool(v) => { buf.push(PUSH_BOOL); buf.push(*v as u8); }
        Opcode::PushNull => buf.push(PUSH_NULL),
        Opcode::PushEnum(en, vn) => { buf.push(PUSH_ENUM); write_str(buf, en); write_str(buf, vn); }

        Opcode::LoadVar(i) => { buf.push(LOAD_VAR); buf.extend(i.to_le_bytes()); }
        Opcode::StoreVar(i) => { buf.push(STORE_VAR); buf.extend(i.to_le_bytes()); }
        Opcode::LoadGlobal(s) => { buf.push(LOAD_GLOBAL); write_str(buf, s); }
        Opcode::StoreGlobal(s) => { buf.push(STORE_GLOBAL); write_str(buf, s); }

        Opcode::Add => buf.push(ADD),
        Opcode::Sub => buf.push(SUB),
        Opcode::Mul => buf.push(MUL),
        Opcode::Div => buf.push(DIV),
        Opcode::Eq => buf.push(EQ),
        Opcode::Neq => buf.push(NEQ),
        Opcode::Gt => buf.push(GT),
        Opcode::Lt => buf.push(LT),
        Opcode::Gte => buf.push(GTE),
        Opcode::Lte => buf.push(LTE),
        Opcode::And => buf.push(AND),
        Opcode::Or => buf.push(OR),
        Opcode::Not => buf.push(NOT),

        Opcode::Input => buf.push(INPUT),
        Opcode::Print(n) => { buf.push(PRINT); write_u32(buf, *n); }

        Opcode::Jump(d) => { buf.push(JUMP); write_u32(buf, *d); }
        Opcode::JumpIfFalse(d) => { buf.push(JUMP_IF_FALSE); write_u32(buf, *d); }

        Opcode::CallNamed(s, n) => { buf.push(CALL_NAMED); write_str(buf, s); write_u32(buf, *n); }
        Opcode::Call(a, n) => { buf.push(CALL); write_u32(buf, *a); write_u32(buf, *n); }
        Opcode::Return => buf.push(RETURN),

        Opcode::BuildList(n) => { buf.push(BUILD_LIST); write_u32(buf, *n); }
        Opcode::BuildMap(n) => { buf.push(BUILD_MAP); write_u32(buf, *n); }
        Opcode::GetIndex => buf.push(GET_INDEX),
        Opcode::SetIndex => buf.push(SET_INDEX),
        Opcode::Pop => buf.push(POP),

        Opcode::Suspend => buf.push(SUSPEND),
        Opcode::Spawn(a, n) => { buf.push(SPAWN); write_u32(buf, *a); write_u32(buf, *n); }

        Opcode::PushHandler(pc) => { buf.push(PUSH_HANDLER); write_u32(buf, *pc); }
        Opcode::PopHandler => buf.push(POP_HANDLER),

        Opcode::Halt => buf.push(HALT),
    }
}

fn write_u32(buf: &mut Vec<u8>, v: usize) {
    buf.extend((v as u32).to_le_bytes());
}

fn write_str(buf: &mut Vec<u8>, s: &str) {
    write_u32(buf, s.len());
    buf.extend_from_slice(s.as_bytes());
}

// ── Decoding ──────────────────────────────────────────────────────────────────

/// Parses LBC bytes. Never panics on malformed input: every read is
/// bounds-checked and reported as a DecodeError.
pub fn decode(data: &[u8]) -> Result<Program, DecodeError> {
    let mut r = Reader { data, pos: 0 };

    if r.bytes(4)? != MAGIC {
        return Err(DecodeError::BadMagic);
    }
    let version = r.u32()?;
    if version != FORMAT_VERSION {
        return Err(DecodeError::UnsupportedVersion(version));
    }
    let source_hash = r.u64()?;
    let count = r.u32()? as usize;

    // Cap the pre-allocation so a corrupt count cannot request huge memory.
    let mut opcodes = Vec::with_capacity(count.min(data.len()));
    for _ in 0..count {
        opcodes.push(read_opcode(&mut r)?);
    }
    if r.pos != data.len() {
        return Err(DecodeError::TrailingBytes { at: r.pos });
    }
    Ok(Program { source_hash, opcodes })
}

/// Reads only the header's source hash, without decoding opcodes.
/// Lets the cache reject a stale file cheaply.
pub fn peek_source_hash(data: &[u8]) -> Result<u64, DecodeError> {
    let mut r = Reader { data, pos: 0 };
    if r.bytes(4)? != MAGIC {
        return Err(DecodeError::BadMagic);
    }
    let version = r.u32()?;
    if version != FORMAT_VERSION {
        return Err(DecodeError::UnsupportedVersion(version));
    }
    r.u64()
}

fn read_opcode(r: &mut Reader) -> Result<Opcode, DecodeError> {
    use tag::*;
    let at = r.pos;
    let op = match r.u8()? {
        PUSH_INT => Opcode::PushInt(r.i64()?),
        PUSH_FLOAT => Opcode::PushFloat(r.f64()?),
        PUSH_STRING => Opcode::PushString(r.string()?),
        PUSH_BOOL => Opcode::PushBool(r.u8()? != 0),
        PUSH_NULL => Opcode::PushNull,
        PUSH_ENUM => {
            let en = r.string()?;
            let vn = r.string()?;
            Opcode::PushEnum(en, vn)
        }

        LOAD_VAR => Opcode::LoadVar(r.u16()?),
        STORE_VAR => Opcode::StoreVar(r.u16()?),
        LOAD_GLOBAL => Opcode::LoadGlobal(r.string()?),
        STORE_GLOBAL => Opcode::StoreGlobal(r.string()?),

        ADD => Opcode::Add,
        SUB => Opcode::Sub,
        MUL => Opcode::Mul,
        DIV => Opcode::Div,
        EQ => Opcode::Eq,
        NEQ => Opcode::Neq,
        GT => Opcode::Gt,
        LT => Opcode::Lt,
        GTE => Opcode::Gte,
        LTE => Opcode::Lte,
        AND => Opcode::And,
        OR => Opcode::Or,
        NOT => Opcode::Not,

        INPUT => Opcode::Input,
        PRINT => Opcode::Print(r.usize()?),

        JUMP => Opcode::Jump(r.usize()?),
        JUMP_IF_FALSE => Opcode::JumpIfFalse(r.usize()?),

        CALL_NAMED => {
            let name = r.string()?;
            Opcode::CallNamed(name, r.usize()?)
        }
        CALL => {
            let addr = r.usize()?;
            Opcode::Call(addr, r.usize()?)
        }
        RETURN => Opcode::Return,

        BUILD_LIST => Opcode::BuildList(r.usize()?),
        BUILD_MAP => Opcode::BuildMap(r.usize()?),
        GET_INDEX => Opcode::GetIndex,
        SET_INDEX => Opcode::SetIndex,
        POP => Opcode::Pop,

        SUSPEND => Opcode::Suspend,
        SPAWN => {
            let addr = r.usize()?;
            Opcode::Spawn(addr, r.usize()?)
        }

        PUSH_HANDLER => Opcode::PushHandler(r.usize()?),
        POP_HANDLER => Opcode::PopHandler,

        HALT => Opcode::Halt,

        other => return Err(DecodeError::UnknownOpcode { tag: other, at }),
    };
    Ok(op)
}

// Bounds-checked cursor over the input buffer.
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn bytes(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        let end = self.pos.checked_add(n).filter(|&e| e <= self.data.len());
        match end {
            Some(end) => {
                let slice = &self.data[self.pos..end];
                self.pos = end;
                Ok(slice)
            }
            None => Err(DecodeError::Truncated { at: self.pos }),
        }
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.bytes(N)?);
        Ok(out)
    }

    fn u8(&mut self) -> Result<u8, DecodeError> { Ok(self.array::<1>()?[0]) }
    fn u16(&mut self) -> Result<u16, DecodeError> { Ok(u16::from_le_bytes(self.array()?)) }
    fn u32(&mut self) -> Result<u32, DecodeError> { Ok(u32::from_le_bytes(self.array()?)) }
    fn u64(&mut self) -> Result<u64, DecodeError> { Ok(u64::from_le_bytes(self.array()?)) }
    fn i64(&mut self) -> Result<i64, DecodeError> { Ok(i64::from_le_bytes(self.array()?)) }
    fn f64(&mut self) -> Result<f64, DecodeError> { Ok(f64::from_le_bytes(self.array()?)) }
    fn usize(&mut self) -> Result<usize, DecodeError> { Ok(self.u32()? as usize) }

    fn string(&mut self) -> Result<String, DecodeError> {
        let len = self.usize()?;
        let at = self.pos;
        let raw = self.bytes(len)?;
        std::str::from_utf8(raw)
            .map(str::to_string)
            .map_err(|_| DecodeError::InvalidUtf8 { at })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // One instance of every opcode, so a new variant without an encoder
    // or decoder arm fails here instead of at runtime.
    fn every_opcode() -> Vec<Opcode> {
        vec![
            Opcode::PushInt(-42),
            Opcode::PushFloat(3.5),
            Opcode::PushString("olá".into()),
            Opcode::PushBool(true),
            Opcode::PushNull,
            Opcode::PushEnum("Status".into(), "Ok".into()),
            Opcode::LoadVar(7),
            Opcode::StoreVar(7),
            Opcode::LoadGlobal("x".into()),
            Opcode::StoreGlobal("x".into()),
            Opcode::Add, Opcode::Sub, Opcode::Mul, Opcode::Div,
            Opcode::Eq, Opcode::Neq, Opcode::Gt, Opcode::Lt, Opcode::Gte, Opcode::Lte,
            Opcode::And, Opcode::Or, Opcode::Not,
            Opcode::Input,
            Opcode::Print(2),
            Opcode::Jump(10),
            Opcode::JumpIfFalse(12),
            Opcode::CallNamed("len".into(), 1),
            Opcode::Call(30, 2),
            Opcode::Return,
            Opcode::BuildList(3),
            Opcode::BuildMap(2),
            Opcode::GetIndex,
            Opcode::SetIndex,
            Opcode::Pop,
            Opcode::Suspend,
            Opcode::Spawn(40, 1),
            Opcode::PushHandler(50),
            Opcode::PopHandler,
            Opcode::Halt,
        ]
    }

    #[test]
    fn round_trip_every_opcode() {
        let ops = every_opcode();
        let bytes = encode(0xDEAD_BEEF, &ops);
        let program = decode(&bytes).expect("decode");
        assert_eq!(program.source_hash, 0xDEAD_BEEF);
        assert_eq!(format!("{:?}", program.opcodes), format!("{:?}", ops));
    }

    #[test]
    fn peek_hash_matches() {
        let bytes = encode(99, &[Opcode::Halt]);
        assert_eq!(peek_source_hash(&bytes), Ok(99));
    }

    #[test]
    fn rejects_bad_magic() {
        let mut bytes = encode(1, &[Opcode::Halt]);
        bytes[0] = b'X';
        assert_eq!(decode(&bytes).unwrap_err(), DecodeError::BadMagic);
    }

    #[test]
    fn rejects_other_version() {
        let mut bytes = encode(1, &[Opcode::Halt]);
        bytes[4] = 99;
        assert!(matches!(decode(&bytes), Err(DecodeError::UnsupportedVersion(_))));
    }

    #[test]
    fn truncated_input_is_an_error_not_a_panic() {
        let bytes = encode(1, &every_opcode());
        for cut in 0..bytes.len() {
            assert!(decode(&bytes[..cut]).is_err(), "cut at {} should fail", cut);
        }
    }

    #[test]
    fn rejects_unknown_tag() {
        let mut bytes = encode(1, &[Opcode::Halt]);
        let last = bytes.len() - 1;
        bytes[last] = 0xEE;
        assert!(matches!(decode(&bytes), Err(DecodeError::UnknownOpcode { tag: 0xEE, .. })));
    }
}
