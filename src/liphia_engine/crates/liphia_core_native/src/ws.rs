// liphia_core_native/src/ws.rs
//
// WebSocket server, RFC 6455, text frames only, no external dependencies.
//
// Functions registered:
//   ws_listen(port: int)                 -> bool
//   ws_accept() -> int                   new client handle, 0 when none is pending
//   ws_clients() -> list                 handles of connected clients
//   ws_send(handle: int, msg: str)       -> bool
//   ws_recv(handle: int) -> str          next text message, "" when none is pending
//   ws_broadcast(msg: str)               -> bool
//   ws_close(handle: int)                -> bool
//
//   ws_send_json(handle: int, data: any) -> bool   json_encode + ws_send
//   ws_broadcast_json(data: any)         -> bool   json_encode + ws_broadcast
//
// Threads: a background thread accepts connections and performs the HTTP
// upgrade handshake; accepted clients are queued for ws_accept(). Reads and
// writes happen on the VM thread, inside the natives.
//
// Client sockets are always blocking. Windows makes accepted sockets inherit
// the listener's non-blocking mode (Linux does not), so every accepted
// stream is switched back explicitly; without that the handshake read fails
// with WouldBlock before the browser's request arrives. ws_recv stays
// non-blocking by peeking for a frame header first and only reading once
// one is there.
//
// Control frames are handled internally: ping is answered with pong, close
// removes the client, and a client whose socket reached EOF is removed too.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use liphia_virtual_machine::value::Value;
use liphia_virtual_machine::vm::{VmError, VmResult, VM};

use crate::json;
use crate::util::{expect_args, int_arg, port_arg, str_arg, str_value};

// Upper bound for one message and for how long a started frame or a
// handshake may take to arrive; protects the server from stalled or
// hostile clients.
const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;
const IO_TIMEOUT: Duration = Duration::from_secs(5);

thread_local! {
    static WS_STATE: RefCell<Option<Arc<WsState>>> = RefCell::new(None);
}

struct WsState {
    clients: Mutex<HashMap<i64, TcpStream>>,
    next_id: Mutex<i64>,
    pending_accepts: Mutex<VecDeque<i64>>,
}

pub fn register(vm: &mut VM) {
    vm.register_native("ws_listen", native_ws_listen);
    vm.register_native("ws_accept", native_ws_accept);
    vm.register_native("ws_clients", native_ws_clients);
    vm.register_native("ws_send", native_ws_send);
    vm.register_native("ws_recv", native_ws_recv);
    vm.register_native("ws_broadcast", native_ws_broadcast);
    vm.register_native("ws_close", native_ws_close);
    vm.register_native("ws_send_json", native_ws_send_json);
    vm.register_native("ws_broadcast_json", native_ws_broadcast_json);
}

// ── Handshake ─────────────────────────────────────────────────────────────────

fn ws_handshake(stream: &mut TcpStream) -> Result<(), String> {
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut headers: HashMap<String, String> = HashMap::new();

    let mut request_line = String::new();
    let n = reader
        .read_line(&mut request_line)
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("connection closed before any request was sent".to_string());
    }
    let request_line = request_line.trim().to_string();

    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).map_err(|e| e.to_string())?;
        let line = line.trim();
        if n == 0 || line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.insert(k.trim().to_lowercase(), v.trim().to_string());
        }
    }

    // The request line and header names go into the error so a failed
    // handshake can be diagnosed from the server log alone.
    let key = match headers.get("sec-websocket-key") {
        Some(k) => k,
        None => {
            let mut names: Vec<&String> = headers.keys().collect();
            names.sort();
            let names: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            return Err(format!(
                "not a WebSocket request: '{}' (headers: {})",
                request_line,
                names.join(", ")
            ));
        }
    };

    let response = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {}\r\n\r\n",
        ws_accept_key(key)
    );
    stream
        .write_all(response.as_bytes())
        .map_err(|e| e.to_string())
}

fn ws_accept_key(client_key: &str) -> String {
    const MAGIC: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let combined = format!("{}{}", client_key.trim(), MAGIC);
    base64_encode(&sha1_bytes(combined.as_bytes()))
}

// Minimal SHA-1 (RFC 3174)
fn sha1_bytes(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];

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
        let b1 = if chunk.len() > 1 {
            chunk[1] as usize
        } else {
            0
        };
        let b2 = if chunk.len() > 2 {
            chunk[2] as usize
        } else {
            0
        };

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

// ── Frame I/O ─────────────────────────────────────────────────────────────────

// Server frames are never masked (RFC 6455, section 5.1).
fn write_frame(stream: &mut TcpStream, opcode: u8, payload: &[u8]) -> std::io::Result<()> {
    let mut frame = vec![0x80 | opcode];
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
    stream.write_all(&frame)
}

fn ws_send_frame(stream: &mut TcpStream, msg: &str) -> VmResult<()> {
    write_frame(stream, 0x1, msg.as_bytes()).map_err(|e| VmError::new(format!("ws_send(): {}", e)))
}

enum Incoming {
    Nothing,
    Text(String),
    Closed,
}

// Checks without blocking whether a frame header is waiting, then reads the
// whole frame in blocking mode (bounded by IO_TIMEOUT). Peeking first means
// a frame is never half-consumed by a timeout.
fn poll_frame(stream: &mut TcpStream) -> Incoming {
    let mut probe = [0u8; 2];
    if stream.set_nonblocking(true).is_err() {
        return Incoming::Closed;
    }
    let peeked = stream.peek(&mut probe);
    let _ = stream.set_nonblocking(false);
    match peeked {
        Ok(0) => return Incoming::Closed,
        Ok(n) if n < 2 => return Incoming::Nothing,
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::WouldBlock => return Incoming::Nothing,
        Err(_) => return Incoming::Closed,
    }
    let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
    match read_frame(stream) {
        Ok(Some((0x8, _))) => Incoming::Closed,
        Ok(Some((0x9, payload))) => {
            // Ping: answer with a pong carrying the same payload.
            if write_frame(stream, 0xA, &payload).is_err() {
                return Incoming::Closed;
            }
            Incoming::Nothing
        }
        Ok(Some((0x1, payload))) | Ok(Some((0x0, payload))) => {
            Incoming::Text(String::from_utf8_lossy(&payload).to_string())
        }
        Ok(_) => Incoming::Nothing,
        Err(_) => Incoming::Closed,
    }
}

// Reads one complete frame and returns (opcode, unmasked payload).
fn read_frame(stream: &mut TcpStream) -> std::io::Result<Option<(u8, Vec<u8>)>> {
    let mut header = [0u8; 2];
    stream.read_exact(&mut header)?;
    let opcode = header[0] & 0x0F;
    let masked = header[1] & 0x80 != 0;
    let mut len = (header[1] & 0x7F) as usize;

    if len == 126 {
        let mut buf = [0u8; 2];
        stream.read_exact(&mut buf)?;
        len = u16::from_be_bytes(buf) as usize;
    } else if len == 127 {
        let mut buf = [0u8; 8];
        stream.read_exact(&mut buf)?;
        len = u64::from_be_bytes(buf) as usize;
    }
    if len > MAX_FRAME_BYTES {
        return Err(std::io::Error::new(
            ErrorKind::InvalidData,
            "frame too large",
        ));
    }

    let mut mask = [0u8; 4];
    if masked {
        stream.read_exact(&mut mask)?;
    }
    let mut payload = vec![0u8; len];
    stream.read_exact(&mut payload)?;
    if masked {
        for (i, b) in payload.iter_mut().enumerate() {
            *b ^= mask[i % 4];
        }
    }
    Ok(Some((opcode, payload)))
}

// ── State helpers ─────────────────────────────────────────────────────────────

fn get_state() -> VmResult<Arc<WsState>> {
    WS_STATE.with(|s| {
        s.borrow()
            .as_ref()
            .cloned()
            .ok_or_else(|| VmError::new("ws: server not started (call ws_listen(port))"))
    })
}

fn send_to(name: &str, handle: i64, msg: &str) -> VmResult<Value> {
    let state = get_state()?;
    let mut map = state.clients.lock().unwrap();
    let stream = map
        .get_mut(&handle)
        .ok_or_else(|| VmError::new(format!("{}(): no client with handle {}", name, handle)))?;
    ws_send_frame(stream, msg)?;
    Ok(Value::Bool(true))
}

// Clients whose socket fails during the broadcast are dropped from the
// registry instead of failing the whole call.
fn broadcast(msg: &str) -> VmResult<Value> {
    let state = get_state()?;
    let mut map = state.clients.lock().unwrap();
    let dead: Vec<i64> = map
        .iter_mut()
        .filter_map(|(id, stream)| ws_send_frame(stream, msg).is_err().then_some(*id))
        .collect();
    for id in dead {
        map.remove(&id);
    }
    Ok(Value::Bool(true))
}

// Runs on a per-connection thread: completes the upgrade handshake and, on
// success, registers the client and queues it for ws_accept().
fn handshake_and_register(mut stream: TcpStream, state: &WsState) {
    let addr = stream
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "?".to_string());
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
    if let Err(e) = ws_handshake(&mut stream) {
        eprintln!("[liphia/ws] handshake failed from {}: {}", addr, e);
        return;
    }
    let _ = stream.set_nodelay(true);

    let id = {
        let mut n = state.next_id.lock().unwrap();
        let id = *n;
        *n += 1;
        id
    };
    state.clients.lock().unwrap().insert(id, stream);
    state.pending_accepts.lock().unwrap().push_back(id);
    eprintln!("[liphia/ws] client connected: {} handle={}", addr, id);
}

// ── Natives ───────────────────────────────────────────────────────────────────

fn native_ws_listen(args: Vec<Value>) -> VmResult<Value> {
    expect_args("ws_listen", &args, 1)?;
    let port = port_arg(&args[0], "ws_listen")?;
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
        .map_err(|e| VmError::new(format!("ws_listen(): port {}: {}", port, e)))?;

    let state = Arc::new(WsState {
        clients: Mutex::new(HashMap::new()),
        next_id: Mutex::new(1),
        pending_accepts: Mutex::new(VecDeque::new()),
    });
    WS_STATE.with(|s| *s.borrow_mut() = Some(state.clone()));

    eprintln!("[liphia/ws] listening on port {}", port);

    // The listener stays blocking: this thread only accepts. Each accepted
    // stream gets its own thread for the handshake, so a socket that opens
    // and never sends anything (browsers pre-open spare connections) cannot
    // hold up the real ones. Streams are forced to blocking mode (see the
    // header note about Windows) and get a timeout for the handshake.
    thread::spawn(move || {
        for incoming in listener.incoming() {
            let stream = match incoming {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[liphia/ws] accept error: {}", e);
                    continue;
                }
            };
            let state = state.clone();
            thread::spawn(move || handshake_and_register(stream, &state));
        }
    });

    Ok(Value::Bool(true))
}

fn native_ws_accept(args: Vec<Value>) -> VmResult<Value> {
    expect_args("ws_accept", &args, 0)?;
    let state = get_state()?;
    let id = state
        .pending_accepts
        .lock()
        .unwrap()
        .pop_front()
        .unwrap_or(0);
    Ok(Value::Int(id))
}

fn native_ws_clients(args: Vec<Value>) -> VmResult<Value> {
    expect_args("ws_clients", &args, 0)?;
    let state = get_state()?;
    let mut ids: Vec<i64> = state.clients.lock().unwrap().keys().copied().collect();
    ids.sort_unstable();
    Ok(Value::List(Rc::new(RefCell::new(
        ids.into_iter().map(Value::Int).collect(),
    ))))
}

fn native_ws_send(args: Vec<Value>) -> VmResult<Value> {
    expect_args("ws_send", &args, 2)?;
    let handle = int_arg(&args[0], "ws_send")?;
    let msg = str_arg(&args[1], "ws_send")?;
    send_to("ws_send", handle, &msg)
}

fn native_ws_send_json(args: Vec<Value>) -> VmResult<Value> {
    expect_args("ws_send_json", &args, 2)?;
    let handle = int_arg(&args[0], "ws_send_json")?;
    send_to("ws_send_json", handle, &json::encode(&args[1]))
}

// Unknown handles and closed clients read as "" so a polling loop never
// has to special-case disconnects; the client simply leaves ws_clients().
fn native_ws_recv(args: Vec<Value>) -> VmResult<Value> {
    expect_args("ws_recv", &args, 1)?;
    let handle = int_arg(&args[0], "ws_recv")?;
    let state = get_state()?;
    let mut map = state.clients.lock().unwrap();
    let incoming = match map.get_mut(&handle) {
        Some(stream) => poll_frame(stream),
        None => return Ok(str_value("")),
    };
    match incoming {
        Incoming::Text(msg) => Ok(str_value(msg)),
        Incoming::Nothing => Ok(str_value("")),
        Incoming::Closed => {
            map.remove(&handle);
            eprintln!("[liphia/ws] client disconnected: handle={}", handle);
            Ok(str_value(""))
        }
    }
}

fn native_ws_broadcast(args: Vec<Value>) -> VmResult<Value> {
    expect_args("ws_broadcast", &args, 1)?;
    let msg = str_arg(&args[0], "ws_broadcast")?;
    broadcast(&msg)
}

fn native_ws_broadcast_json(args: Vec<Value>) -> VmResult<Value> {
    expect_args("ws_broadcast_json", &args, 1)?;
    broadcast(&json::encode(&args[0]))
}

// Sends a close frame (best effort) before dropping the socket.
fn native_ws_close(args: Vec<Value>) -> VmResult<Value> {
    expect_args("ws_close", &args, 1)?;
    let handle = int_arg(&args[0], "ws_close")?;
    let state = get_state()?;
    let removed = state.clients.lock().unwrap().remove(&handle);
    if let Some(mut stream) = removed {
        let _ = write_frame(&mut stream, 0x8, &[]);
        return Ok(Value::Bool(true));
    }
    Ok(Value::Bool(false))
}
