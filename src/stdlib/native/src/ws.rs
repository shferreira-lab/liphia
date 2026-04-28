// stdlib/native/src/ws.rs
//
// WebSocket server — RFC 6455 (text frames only).
// No external dependencies.
// Multi-client server with background accept thread.
//
// Functions registered:
//   ws_listen(port: int)              -> bool
//   ws_accept()                       -> int         (0 if none)
//   ws_clients()                      -> list[int]
//   ws_send(handle: int, msg: str)    -> bool
//   ws_recv(handle: int)              -> str         ("" if none)
//   ws_broadcast(msg: str)            -> bool
//   ws_close(handle: int)             -> bool

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use liphia_virtual_machine::value::Value;
use liphia_virtual_machine::vm::{VM, VmError, VmResult};

thread_local! {
    static WS_STATE: RefCell<Option<Arc<WsState>>> = RefCell::new(None);
}

struct WsState {
    clients: Mutex<HashMap<i64, TcpStream>>,
    next_id: Mutex<i64>,
    pending_accepts: Mutex<VecDeque<i64>>,
}

pub fn register(vm: &mut VM) {
    vm.register_native("ws_listen",    native_ws_listen);
    vm.register_native("ws_accept",    native_ws_accept);
    vm.register_native("ws_clients",   native_ws_clients);
    vm.register_native("ws_send",      native_ws_send);
    vm.register_native("ws_recv",      native_ws_recv);
    vm.register_native("ws_broadcast", native_ws_broadcast);
    vm.register_native("ws_close",     native_ws_close);
}

// ─────────────────────────────────────────────────────────────────────────────
// Handshake
// ─────────────────────────────────────────────────────────────────────────────

fn ws_handshake(stream: &mut TcpStream) -> Result<(), String> {
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut headers: HashMap<String, String> = HashMap::new();

    // request line
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| e.to_string())?;

    // headers
    loop {
        let mut h = String::new();
        reader.read_line(&mut h).map_err(|e| e.to_string())?;
        let h = h.trim();
        if h.is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            headers.insert(k.trim().to_lowercase(), v.trim().to_string());
        }
    }

    let key = headers
        .get("sec-websocket-key")
        .ok_or("missing Sec-WebSocket-Key")?;

    let accept = ws_accept_key(key);

    let response = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {}\r\n\r\n",
        accept
    );

    stream
        .write_all(response.as_bytes())
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn ws_accept_key(client_key: &str) -> String {
    const MAGIC: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let combined = format!("{}{}", client_key.trim(), MAGIC);
    let hash = sha1_bytes(combined.as_bytes());
    base64_encode(&hash)
}

// Minimal SHA-1 (RFC 3174)
fn sha1_bytes(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [
        0x67452301,
        0xEFCDAB89,
        0x98BADCFE,
        0x10325476,
        0xC3D2E1F0,
    ];

    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);

    while msg.len() % 64 != 56 {
        msg.push(0);
    }

    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks(64) {
        let mut w = [0u32; 80];

        for i in 0..16 {
            w[i] = u32::from_be_bytes(chunk[i * 4..i * 4 + 4].try_into().unwrap());
        }

        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }

        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);

        for i in 0..80 {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999u32),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1u32),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDCu32),
                _ => (b ^ c ^ d, 0xCA62C1D6u32),
            };

            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[i]);

            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }

    let mut out = [0u8; 20];
    for (i, &val) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&val.to_be_bytes());
    }

    out
}

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut out = String::new();

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = if chunk.len() > 1 { chunk[1] as usize } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as usize } else { 0 };

        out.push(TABLE[b0 >> 2] as char);
        out.push(TABLE[((b0 & 3) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            out.push(TABLE[((b1 & 0xf) << 2) | (b2 >> 6)] as char);
        } else {
            out.push('=');
        }

        if chunk.len() > 2 {
            out.push(TABLE[b2 & 0x3f] as char);
        } else {
            out.push('=');
        }
    }

    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Frame I/O
// ─────────────────────────────────────────────────────────────────────────────

fn ws_send_frame(stream: &mut TcpStream, msg: &str) -> VmResult<()> {
    let payload = msg.as_bytes();
    let mut frame = vec![0x81u8]; // FIN + TEXT opcode

    if payload.len() < 126 {
        frame.push(payload.len() as u8);
    } else if payload.len() < 65536 {
        frame.push(126);
        frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    } else {
        frame.push(127);
        frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    }

    frame.extend_from_slice(payload);

    stream
        .write_all(&frame)
        .map_err(|e| VmError::new(format!("ws_send: {}", e)))
}

fn ws_recv_frame_nonblocking(stream: &mut TcpStream) -> VmResult<Option<String>> {
    stream.set_read_timeout(Some(Duration::from_millis(1))).ok();

    let mut header = [0u8; 2];

    // se não tem dados disponíveis, retorna None
    if stream.read_exact(&mut header).is_err() {
        return Ok(None);
    }

    let masked = (header[1] & 0x80) != 0;
    let mut payload_len = (header[1] & 0x7f) as usize;

    if payload_len == 126 {
        let mut buf = [0u8; 2];
        stream
            .read_exact(&mut buf)
            .map_err(|e| VmError::new(format!("ws_recv: {}", e)))?;
        payload_len = u16::from_be_bytes(buf) as usize;
    } else if payload_len == 127 {
        let mut buf = [0u8; 8];
        stream
            .read_exact(&mut buf)
            .map_err(|e| VmError::new(format!("ws_recv: {}", e)))?;
        payload_len = u64::from_be_bytes(buf) as usize;
    }

    let mut mask = [0u8; 4];
    if masked {
        stream
            .read_exact(&mut mask)
            .map_err(|e| VmError::new(format!("ws_recv mask: {}", e)))?;
    }

    let mut payload = vec![0u8; payload_len];
    stream
        .read_exact(&mut payload)
        .map_err(|e| VmError::new(format!("ws_recv payload: {}", e)))?;

    if masked {
        for (i, b) in payload.iter_mut().enumerate() {
            *b ^= mask[i % 4];
        }
    }

    Ok(Some(String::from_utf8_lossy(&payload).to_string()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn get_state() -> VmResult<Arc<WsState>> {
    WS_STATE.with(|s| {
        s.borrow()
            .as_ref()
            .cloned()
            .ok_or_else(|| VmError::new("ws: server not started (call ws_listen(port))"))
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Native functions
// ─────────────────────────────────────────────────────────────────────────────

fn native_ws_listen(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 1 {
        return Err(VmError::new("ws_listen(port: int) — expected 1 argument"));
    }

    let port = match &args[0] {
        Value::Int(p) => *p,
        _ => return Err(VmError::new("ws_listen: port must be int")),
    };

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
        .map_err(|e| VmError::new(format!("ws_listen: {}", e)))?;

    listener
        .set_nonblocking(true)
        .map_err(|e| VmError::new(format!("ws_listen nonblocking: {}", e)))?;

    let state = Arc::new(WsState {
        clients: Mutex::new(HashMap::new()),
        next_id: Mutex::new(1),
        pending_accepts: Mutex::new(VecDeque::new()),
    });

    WS_STATE.with(|s| {
        *s.borrow_mut() = Some(state.clone());
    });

    eprintln!("[liphia/ws] listening on port {}", port);

    // background accept loop
    thread::spawn(move || loop {
        match listener.accept() {
            Ok((mut stream, addr)) => {
                if ws_handshake(&mut stream).is_ok() {
                    let id = {
                        let mut n = state.next_id.lock().unwrap();
                        let id = *n;
                        *n += 1;
                        id
                    };

                    stream.set_nodelay(true).ok();

                    state.clients.lock().unwrap().insert(id, stream);
                    state.pending_accepts.lock().unwrap().push_back(id);

                    eprintln!("[liphia/ws] client connected: {} handle={}", addr, id);
                }
            }
            Err(_) => {
                thread::sleep(Duration::from_millis(10));
            }
        }
    });

    Ok(Value::Bool(true))
}

fn native_ws_accept(_args: Vec<Value>) -> VmResult<Value> {
    let state = get_state()?;
    let mut q = state.pending_accepts.lock().unwrap();

    if let Some(id) = q.pop_front() {
        Ok(Value::Int(id))
    } else {
        Ok(Value::Int(0))
    }
}

fn native_ws_clients(_args: Vec<Value>) -> VmResult<Value> {
    let state = get_state()?;
    let map = state.clients.lock().unwrap();

    let mut list = Vec::new();
    for id in map.keys() {
        list.push(Value::Int(*id));
    }

    Ok(Value::List(Rc::new(RefCell::new(list))))
}

fn native_ws_send(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 2 {
        return Err(VmError::new("ws_send(handle: int, msg: str) — expected 2 arguments"));
    }

    let handle = match &args[0] {
        Value::Int(h) => *h,
        _ => return Err(VmError::new("ws_send: handle must be int")),
    };

    let msg = match &args[1] {
        Value::Str(s) => s.as_str().to_string(),
        _ => return Err(VmError::new("ws_send: msg must be str")),
    };

    let state = get_state()?;
    let mut map = state.clients.lock().unwrap();

    let stream = map
        .get_mut(&handle)
        .ok_or_else(|| VmError::new(format!("ws_send: no client with handle {}", handle)))?;

    ws_send_frame(stream, &msg)?;
    Ok(Value::Bool(true))
}

fn native_ws_recv(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 1 {
        return Err(VmError::new("ws_recv(handle: int) — expected 1 argument"));
    }

    let handle = match &args[0] {
        Value::Int(h) => *h,
        _ => return Err(VmError::new("ws_recv: handle must be int")),
    };

    let state = get_state()?;
    let mut map = state.clients.lock().unwrap();

    let stream = match map.get_mut(&handle) {
        Some(s) => s,
        None => return Ok(Value::Str(Rc::new("".to_string()))),
    };

    let msg = ws_recv_frame_nonblocking(stream)?;
    Ok(Value::Str(Rc::new(msg.unwrap_or_default())))
}

fn native_ws_broadcast(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 1 {
        return Err(VmError::new("ws_broadcast(msg: str) — expected 1 argument"));
    }

    let msg = match &args[0] {
        Value::Str(s) => s.as_str().to_string(),
        _ => return Err(VmError::new("ws_broadcast: msg must be str")),
    };

    let state = get_state()?;
    let mut map = state.clients.lock().unwrap();

    let mut dead = Vec::new();

    for (id, stream) in map.iter_mut() {
        if ws_send_frame(stream, &msg).is_err() {
            dead.push(*id);
        }
    }

    for id in dead {
        map.remove(&id);
    }

    Ok(Value::Bool(true))
}

fn native_ws_close(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 1 {
        return Err(VmError::new("ws_close(handle: int) — expected 1 argument"));
    }

    let handle = match &args[0] {
        Value::Int(h) => *h,
        _ => return Err(VmError::new("ws_close: handle must be int")),
    };

    let state = get_state()?;
    let removed = state.clients.lock().unwrap().remove(&handle).is_some();
    Ok(Value::Bool(removed))
}