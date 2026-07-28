// liphia_cli/src/cache.rs
use std::fs;
use std::path::{Path, PathBuf};
use liphia_virtual_machine::opcode::Opcode;

const MAGIC:          &[u8; 4] = b"LBC\0";
const FORMAT_VERSION: u32      = 4;

pub fn cache_path(source_path: &Path) -> PathBuf {
    let dir  = source_path.parent().unwrap_or(Path::new("."));
    let stem = source_path.file_stem().unwrap_or_default().to_string_lossy();
    dir.join("liphia_cache").join(format!("{}.lbc", stem))
}
 
pub fn source_hash(source: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in source.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub fn load_cache(source_path: &Path, current_hash: u64) -> Option<Vec<Opcode>> {
    let path = cache_path(source_path);
    let data = match fs::read(&path) {
        Ok(d)  => d,
        Err(_) => {
            eprintln!("[cache] no cache file found: {}", path.display());
            return None;
        }
    };
    let result = deserialize(&data, current_hash);
    if result.is_some() {
        eprintln!("[cache] cache hit: {}", path.display());
    } else {
        eprintln!("[cache] cache miss (hash/version mismatch): {}", path.display());
    }
    result
}

pub fn save_cache(source_path: &Path, hash: u64, opcodes: &[Opcode]) {
    let path = cache_path(source_path);
    if let Some(dir) = path.parent() {
        if let Err(e) = fs::create_dir_all(dir) {
            eprintln!("[cache] could not create liphia_cache/: {}", e);
            return;
        }
    }
    if let Err(e) = fs::write(&path, serialize(hash, opcodes)) {
        eprintln!("[cache] could not write {}: {}", path.display(), e);
    }
    eprintln!("[cache] saved: {}", path.display());
}

// ── Serialization ─────────────────────────────────────────────────────────────

fn serialize(hash: u64, opcodes: &[Opcode]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(MAGIC);
    buf.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    buf.extend_from_slice(&hash.to_le_bytes());
    buf.extend_from_slice(&(opcodes.len() as u32).to_le_bytes());
    for op in opcodes { write_opcode(&mut buf, op); }
    buf
}

fn deserialize(data: &[u8], expected_hash: u64) -> Option<Vec<Opcode>> {
    if data.len() < 16 { return None; }
    if &data[0..4] != MAGIC { return None; }
    let mut cur = 4usize;
    let version = u32::from_le_bytes(data[cur..cur+4].try_into().ok()?);
    if version != FORMAT_VERSION { return None; }
    cur += 4;
    let stored_hash = u64::from_le_bytes(data[cur..cur+8].try_into().ok()?);
    if stored_hash != expected_hash { return None; }
    cur += 8;
    if cur + 4 > data.len() { return None; }
    let count = u32::from_le_bytes(data[cur..cur+4].try_into().ok()?) as usize;
    cur += 4;
    let mut opcodes = Vec::with_capacity(count);
    for _ in 0..count {
        let (op, consumed) = read_opcode(&data[cur..])?;
        opcodes.push(op);
        cur += consumed;
    }
    Some(opcodes)
}

fn write_opcode(buf: &mut Vec<u8>, op: &Opcode) {
    match op {
        Opcode::PushInt(v)       => { buf.push(0x01); buf.extend(&v.to_le_bytes()); }
        Opcode::PushFloat(v)     => { buf.push(0x02); buf.extend(&v.to_le_bytes()); }
        Opcode::PushString(v)    => { buf.push(0x03); write_str(buf, v); }
        Opcode::PushBool(v)      => { buf.push(0x04); buf.push(*v as u8); }
        Opcode::PushNull         => buf.push(0x05),
        Opcode::PushEnum(en, vn) => { buf.push(0x06); write_str(buf, en); write_str(buf, vn); }
        Opcode::LoadVar(i)       => { buf.push(0x10); buf.extend(&i.to_le_bytes()); }
        Opcode::StoreVar(i)      => { buf.push(0x11); buf.extend(&i.to_le_bytes()); }
        Opcode::LoadGlobal(s)    => { buf.push(0x12); write_str(buf, s); }
        Opcode::StoreGlobal(s)   => { buf.push(0x13); write_str(buf, s); }
        Opcode::Add              => buf.push(0x20),
        Opcode::Sub              => buf.push(0x21),
        Opcode::Mul              => buf.push(0x22),
        Opcode::Div              => buf.push(0x23),
        Opcode::Eq               => buf.push(0x24),
        Opcode::Neq              => buf.push(0x25),
        Opcode::Gt               => buf.push(0x26),
        Opcode::Lt               => buf.push(0x27),
        Opcode::Gte              => buf.push(0x28),
        Opcode::Lte              => buf.push(0x29),
        Opcode::And              => buf.push(0x2A),
        Opcode::Or               => buf.push(0x2B),
        Opcode::Not              => buf.push(0x2C),
        Opcode::Input            => buf.push(0x30),
        Opcode::Print(n)         => { buf.push(0x31); buf.extend(&(*n as u32).to_le_bytes()); }
        Opcode::Jump(d)          => { buf.push(0x40); buf.extend(&(*d as u32).to_le_bytes()); }
        Opcode::JumpIfFalse(d)   => { buf.push(0x41); buf.extend(&(*d as u32).to_le_bytes()); }
        Opcode::CallNamed(s, n)  => { buf.push(0x50); write_str(buf, s); buf.extend(&(*n as u32).to_le_bytes()); }
        Opcode::Call(a, n)       => { buf.push(0x51); buf.extend(&(*a as u32).to_le_bytes()); buf.extend(&(*n as u32).to_le_bytes()); }
        Opcode::Return           => buf.push(0x52),
        Opcode::BuildList(n)     => { buf.push(0x60); buf.extend(&(*n as u32).to_le_bytes()); }
        Opcode::BuildMap(n) => { buf.push(0x64); buf.extend(&(*n as u32).to_le_bytes());},
        Opcode::PushHandler(pc)  => { buf.push(0x80); buf.extend(&(*pc as u32).to_le_bytes()); }
        Opcode::PopHandler       => buf.push(0x81),
        Opcode::GetIndex         => buf.push(0x61),
        Opcode::SetIndex         => buf.push(0x62),
        Opcode::Pop              => buf.push(0x63),
        Opcode::Suspend          => buf.push(0x70),
        Opcode::Spawn(a, n)      => { buf.push(0x71); buf.extend(&(*a as u32).to_le_bytes()); buf.extend(&(*n as u32).to_le_bytes()); }
        Opcode::Halt             => buf.push(0xFF),
    }
}

fn read_opcode(data: &[u8]) -> Option<(Opcode, usize)> {
    if data.is_empty() { return None; }
    let (op, size): (Opcode, usize) = match data[0] {
        0x01 => { let v = i64::from_le_bytes(data[1..9].try_into().ok()?); (Opcode::PushInt(v), 8) }
        0x02 => { let v = f64::from_le_bytes(data[1..9].try_into().ok()?); (Opcode::PushFloat(v), 8) }
        0x03 => { let (s,n) = read_str(&data[1..])?; (Opcode::PushString(s), n) }
        0x04 => { (Opcode::PushBool(data[1] != 0), 1) }
        0x05 => { (Opcode::PushNull, 0) }
        0x06 => {
            let (en, n1) = read_str(&data[1..])?;
            let (vn, n2) = read_str(&data[1+n1..])?;
            (Opcode::PushEnum(en, vn), n1 + n2)
        }
        0x10 => { let v = u16::from_le_bytes(data[1..3].try_into().ok()?); (Opcode::LoadVar(v), 2) }
        0x11 => { let v = u16::from_le_bytes(data[1..3].try_into().ok()?); (Opcode::StoreVar(v), 2) }
        0x12 => { let (s,n) = read_str(&data[1..])?; (Opcode::LoadGlobal(s), n) }
        0x13 => { let (s,n) = read_str(&data[1..])?; (Opcode::StoreGlobal(s), n) }
        0x20 => (Opcode::Add,  0), 0x21 => (Opcode::Sub, 0),
        0x22 => (Opcode::Mul,  0), 0x23 => (Opcode::Div, 0),
        0x24 => (Opcode::Eq,   0), 0x25 => (Opcode::Neq, 0),
        0x26 => (Opcode::Gt,   0), 0x27 => (Opcode::Lt,  0),
        0x28 => (Opcode::Gte,  0), 0x29 => (Opcode::Lte, 0),
        0x2A => (Opcode::And,  0), 0x2B => (Opcode::Or,  0),
        0x2C => (Opcode::Not,  0),
        0x30 => (Opcode::Input, 0),
        0x31 => { let n = u32::from_le_bytes(data[1..5].try_into().ok()?) as usize; (Opcode::Print(n), 4) }
        0x40 => { let d = u32::from_le_bytes(data[1..5].try_into().ok()?) as usize; (Opcode::Jump(d), 4) }
        0x41 => { let d = u32::from_le_bytes(data[1..5].try_into().ok()?) as usize; (Opcode::JumpIfFalse(d), 4) }
        0x50 => {
            let (s, sn) = read_str(&data[1..])?;
            let n = u32::from_le_bytes(data[1+sn..1+sn+4].try_into().ok()?) as usize;
            (Opcode::CallNamed(s, n), sn + 4)
        }
        0x51 => {
            let a = u32::from_le_bytes(data[1..5].try_into().ok()?) as usize;
            let n = u32::from_le_bytes(data[5..9].try_into().ok()?) as usize;
            (Opcode::Call(a, n), 8)
        }
        0x52 => (Opcode::Return, 0),
        0x60 => { let n = u32::from_le_bytes(data[1..5].try_into().ok()?) as usize; (Opcode::BuildList(n), 4) }
        0x61 => (Opcode::GetIndex, 0),
        0x62 => (Opcode::SetIndex, 0),
        0x63 => (Opcode::Pop, 0),
        0x64 => { let n  = u32::from_le_bytes(data[1..5].try_into().ok()?) as usize; (Opcode::BuildMap(n), 4) }
        0x70 => (Opcode::Suspend, 0),
        0x71 => {
            let a = u32::from_le_bytes(data[1..5].try_into().ok()?) as usize;
            let n = u32::from_le_bytes(data[5..9].try_into().ok()?) as usize;
            (Opcode::Spawn(a, n), 8)
        }
        0x80 => { let pc = u32::from_le_bytes(data[1..5].try_into().ok()?) as usize; (Opcode::PushHandler(pc), 4) }
        0x81 => (Opcode::PopHandler, 0),
        0xFF => (Opcode::Halt, 0),
        _    => return None,
    };
    Some((op, 1 + size))
}

fn write_str(buf: &mut Vec<u8>, s: &str) {
    let b = s.as_bytes();
    buf.extend(&(b.len() as u32).to_le_bytes());
    buf.extend_from_slice(b);
}

fn read_str(data: &[u8]) -> Option<(String, usize)> {
    if data.len() < 4 { return None; }
    let len = u32::from_le_bytes(data[0..4].try_into().ok()?) as usize;
    if data.len() < 4 + len { return None; }
    let s = std::str::from_utf8(&data[4..4+len]).ok()?.to_string();
    Some((s, 4 + len))
}
