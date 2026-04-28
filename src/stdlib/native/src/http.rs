// stdlib/native/src/http.rs
//
// HTTP/1.1 server (non-blocking accept) + HTTP client — no external deps.
//
// ── Server workflow (use inside async fn + await loop) ────────────────────────
//
//   http_listen(port)               → bool   bind port, spawn accept thread
//   http_accept()                   → bool   true if a new request is ready
//                                            false if queue empty  (await-able)
//   http_method()                   → str    "GET" | "POST" | "PUT" | ...
//   http_path()                     → str    "/users/42"
//   http_query()                    → str    "page=1&limit=10"  (after '?')
//   http_body()                     → str    request body
//   http_header(name: str)          → str    single header value (lowercase key)
//   http_respond(status, body)      → bool   text/plain response + close conn
//   http_respond_json(status, body) → bool   application/json  response
//
// ── Client ───────────────────────────────────────────────────────────────────
//
//   http_get(url)                   → str    response body
//   http_post(url, body)            → str
//   http_put(url, body)             → str
//   http_patch(url, body)           → str
//   http_delete(url)                → str
//   http_status()                   → int    last response HTTP status code

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;

use liphia_virtual_machine::value::Value;
use liphia_virtual_machine::vm::{VmError, VmResult, VM};

// ── State ─────────────────────────────────────────────────────────────────────

struct PendingRequest {
    method:  String,
    path:    String,
    query:   String,
    headers: HashMap<String, String>,
    body:    String,
    stream:  TcpStream,
}

struct HttpServerState {
    pending: Mutex<VecDeque<PendingRequest>>,
}

thread_local! {
    static SERVER_STATE: RefCell<Option<Arc<HttpServerState>>> = RefCell::new(None);

    static CURRENT: RefCell<Option<CurrentRequest>> = RefCell::new(None);

    static LAST_STATUS: RefCell<i64> = RefCell::new(0);
}

struct CurrentRequest {
    method:  String,
    path:    String,
    query:   String,
    headers: HashMap<String, String>,
    body:    String,
    stream:  TcpStream,
}

// ── Registration ──────────────────────────────────────────────────────────────

pub fn register(vm: &mut VM) {
    vm.register_native("http_listen",       native_http_listen);
    vm.register_native("http_accept",       native_http_accept);
    vm.register_native("http_method",       native_http_method);
    vm.register_native("http_path",         native_http_path);
    vm.register_native("http_query",        native_http_query);
    vm.register_native("http_body",         native_http_body);
    vm.register_native("http_header",       native_http_header);
    vm.register_native("http_respond",      native_http_respond);
    vm.register_native("http_respond_json", native_http_respond_json);
    vm.register_native("http_get",          native_http_get);
    vm.register_native("http_post",         native_http_post);
    vm.register_native("http_put",          native_http_put);
    vm.register_native("http_patch",        native_http_patch);
    vm.register_native("http_delete",       native_http_delete);
    vm.register_native("http_status",       native_http_status);
}

// ── Server: listen ────────────────────────────────────────────────────────────

fn native_http_listen(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 1 {
        return Err(VmError::new("http_listen(port: int) — expected 1 argument"));
    }
    let port = match &args[0] {
        Value::Int(p) => *p,
        _ => return Err(VmError::new("http_listen: port must be int")),
    };

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
        .map_err(|e| VmError::new(format!("http_listen: bind failed on port {}: {}", port, e)))?;

    let state = Arc::new(HttpServerState {
        pending: Mutex::new(VecDeque::new()),
    });

    SERVER_STATE.with(|s| *s.borrow_mut() = Some(state.clone()));

    eprintln!("[liphia/http] listening on port {}", port);

    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(mut s) => match parse_request(&mut s) {
                    Ok(req) => {
                        state.pending.lock().unwrap().push_back(req);
                    }
                    Err(e) => eprintln!("[liphia/http] parse error: {}", e),
                },
                Err(e) => eprintln!("[liphia/http] accept error: {}", e),
            }
        }
    });

    Ok(Value::Bool(true))
}

// ── Server: accept (non-blocking — returns false when queue empty) ─────────────

fn native_http_accept(_args: Vec<Value>) -> VmResult<Value> {
    let state = SERVER_STATE
        .with(|s| s.borrow().clone())
        .ok_or_else(|| VmError::new("http_accept: server not started — call http_listen first"))?;

    let req = {
        let mut q = state.pending.lock().unwrap();
        q.pop_front()
    };

    match req {
        None => Ok(Value::Bool(false)), // not ready — Suspend will re-poll
        Some(r) => {
            CURRENT.with(|c| {
                *c.borrow_mut() = Some(CurrentRequest {
                    method:  r.method,
                    path:    r.path,
                    query:   r.query,
                    headers: r.headers,
                    body:    r.body,
                    stream:  r.stream,
                });
            });
            Ok(Value::Bool(true))
        }
    }
}

// ── Server: request accessors ─────────────────────────────────────────────────

fn native_http_method(_args: Vec<Value>) -> VmResult<Value> {
    Ok(Value::Str(Rc::new(
        CURRENT.with(|c| c.borrow().as_ref().map(|r| r.method.clone()).unwrap_or_default()),
    )))
}

fn native_http_path(_args: Vec<Value>) -> VmResult<Value> {
    Ok(Value::Str(Rc::new(
        CURRENT.with(|c| c.borrow().as_ref().map(|r| r.path.clone()).unwrap_or_default()),
    )))
}

fn native_http_query(_args: Vec<Value>) -> VmResult<Value> {
    Ok(Value::Str(Rc::new(
        CURRENT.with(|c| c.borrow().as_ref().map(|r| r.query.clone()).unwrap_or_default()),
    )))
}

fn native_http_body(_args: Vec<Value>) -> VmResult<Value> {
    Ok(Value::Str(Rc::new(
        CURRENT.with(|c| c.borrow().as_ref().map(|r| r.body.clone()).unwrap_or_default()),
    )))
}

fn native_http_header(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 1 {
        return Err(VmError::new("http_header(name: str) — expected 1 argument"));
    }
    let name = match &args[0] {
        Value::Str(s) => s.to_lowercase(),
        _ => return Err(VmError::new("http_header: name must be str")),
    };
    let val = CURRENT.with(|c| {
        c.borrow()
            .as_ref()
            .and_then(|r| r.headers.get(&name).cloned())
            .unwrap_or_default()
    });
    Ok(Value::Str(Rc::new(val)))
}

// ── Server: respond ───────────────────────────────────────────────────────────

fn send_response(status: i64, body: &str, content_type: &str) -> VmResult<Value> {
    let reason   = status_reason(status);
    let response = format!(
        "HTTP/1.1 {} {}\r\n\
         Content-Type: {}\r\n\
         Content-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Connection: close\r\n\r\n{}",
        status, reason, content_type, body.len(), body
    );

    CURRENT.with(|c| {
        let mut opt = c.borrow_mut();
        if let Some(ref mut req) = *opt {
            req.stream
                .write_all(response.as_bytes())
                .map(|_| Value::Bool(true))
                .map_err(|e| VmError::new(format!("http_respond: write failed: {}", e)))
        } else {
            Err(VmError::new(
                "http_respond: no active request — call http_accept first",
            ))
        }
    })
}

fn native_http_respond(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 2 {
        return Err(VmError::new(
            "http_respond(status: int, body: str) — expected 2 arguments",
        ));
    }
    let status = match &args[0] {
        Value::Int(s) => *s,
        _ => return Err(VmError::new("http_respond: status must be int")),
    };
    let body = match &args[1] {
        Value::Str(s) => s.as_str().to_string(),
        _ => return Err(VmError::new("http_respond: body must be str")),
    };
    send_response(status, &body, "text/plain; charset=utf-8")
}

fn native_http_respond_json(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 2 {
        return Err(VmError::new(
            "http_respond_json(status: int, body: str) — expected 2 arguments",
        ));
    }
    let status = match &args[0] {
        Value::Int(s) => *s,
        _ => return Err(VmError::new("http_respond_json: status must be int")),
    };
    let body = match &args[1] {
        Value::Str(s) => s.as_str().to_string(),
        _ => return Err(VmError::new("http_respond_json: body must be str")),
    };
    send_response(status, &body, "application/json; charset=utf-8")
}

// ── Request parser ────────────────────────────────────────────────────────────

fn parse_request(stream: &mut TcpStream) -> Result<PendingRequest, String> {
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);

    // Request line: METHOD /path?query HTTP/1.1
    let mut first_line = String::new();
    reader.read_line(&mut first_line).map_err(|e| e.to_string())?;
    let parts: Vec<&str> = first_line.trim().splitn(3, ' ').collect();
    if parts.len() < 2 {
        return Err(format!("malformed request line: {:?}", first_line));
    }
    let method     = parts[0].to_string();
    let full_path  = parts[1].to_string();

    // Split path from query string
    let (path, query) = if let Some(idx) = full_path.find('?') {
        (full_path[..idx].to_string(), full_path[idx+1..].to_string())
    } else {
        (full_path, String::new())
    };

    // Headers
    let mut headers        = HashMap::new();
    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|e| e.to_string())?;
        let line = line.trim();
        if line.is_empty() { break; }
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim().to_lowercase();
            let val = v.trim().to_string();
            if key == "content-length" {
                content_length = val.parse().unwrap_or(0);
            }
            headers.insert(key, val);
        }
    }

    // Body
    let mut body_bytes = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body_bytes).map_err(|e| e.to_string())?;
    }
    let body = String::from_utf8_lossy(&body_bytes).to_string();

    // We need the original stream (not the BufReader clone) for writing.
    // The TcpStream passed in is the one we write to.
    let write_stream = stream.try_clone().map_err(|e| e.to_string())?;

    Ok(PendingRequest { method, path, query, headers, body, stream: write_stream })
}

// ── HTTP client ───────────────────────────────────────────────────────────────

fn http_request(method: &str, url: &str, body: Option<&str>) -> VmResult<(i64, String)> {
    // Strip scheme
    let url = url.strip_prefix("http://").unwrap_or(url);

    let (host_port, path) = if let Some(idx) = url.find('/') {
        (&url[..idx], &url[idx..])
    } else {
        (url, "/")
    };

    let (host, port) = if let Some(idx) = host_port.rfind(':') {
        let p: u16 = host_port[idx+1..].parse()
            .map_err(|_| VmError::new(format!("http: invalid port in URL '{}'", url)))?;
        (&host_port[..idx], p)
    } else {
        (host_port, 80u16)
    };

    let mut stream = TcpStream::connect(format!("{}:{}", host, port))
        .map_err(|e| VmError::new(format!("http: connect failed to {}: {}", host, e)))?;

    let body_str     = body.unwrap_or("");
    let content_type = if body.is_some() { "Content-Type: application/json\r\n" } else { "" };
    let request      = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nContent-Length: {}\r\n{}Connection: close\r\n\r\n{}",
        method, path, host, body_str.len(), content_type, body_str
    );

    stream.write_all(request.as_bytes())
        .map_err(|e| VmError::new(format!("http: send failed: {}", e)))?;

    let mut reader      = BufReader::new(&stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line)
        .map_err(|e| VmError::new(format!("http: read status failed: {}", e)))?;

    let status: i64 = status_line.split_whitespace().nth(1)
        .and_then(|s| s.parse().ok()).unwrap_or(0);

    // Skip response headers
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).ok();
        if line.trim().is_empty() { break; }
    }

    let mut response_body = String::new();
    reader.read_to_string(&mut response_body)
        .map_err(|e| VmError::new(format!("http: read body failed: {}", e)))?;

    Ok((status, response_body))
}

fn native_http_get(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 1 {
        return Err(VmError::new("http_get(url: str) — expected 1 argument"));
    }
    let url = str_arg(&args[0], "http_get")?;
    let (status, body) = http_request("GET", &url, None)?;
    LAST_STATUS.with(|s| *s.borrow_mut() = status);
    Ok(Value::Str(Rc::new(body)))
}

fn native_http_post(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 2 {
        return Err(VmError::new("http_post(url: str, body: str) — expected 2 arguments"));
    }
    let url  = str_arg(&args[0], "http_post")?;
    let body = str_arg(&args[1], "http_post")?;
    let (status, resp) = http_request("POST", &url, Some(&body))?;
    LAST_STATUS.with(|s| *s.borrow_mut() = status);
    Ok(Value::Str(Rc::new(resp)))
}

fn native_http_put(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 2 {
        return Err(VmError::new("http_put(url: str, body: str) — expected 2 arguments"));
    }
    let url  = str_arg(&args[0], "http_put")?;
    let body = str_arg(&args[1], "http_put")?;
    let (status, resp) = http_request("PUT", &url, Some(&body))?;
    LAST_STATUS.with(|s| *s.borrow_mut() = status);
    Ok(Value::Str(Rc::new(resp)))
}

fn native_http_patch(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 2 {
        return Err(VmError::new("http_patch(url: str, body: str) — expected 2 arguments"));
    }
    let url  = str_arg(&args[0], "http_patch")?;
    let body = str_arg(&args[1], "http_patch")?;
    let (status, resp) = http_request("PATCH", &url, Some(&body))?;
    LAST_STATUS.with(|s| *s.borrow_mut() = status);
    Ok(Value::Str(Rc::new(resp)))
}

fn native_http_delete(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 1 {
        return Err(VmError::new("http_delete(url: str) — expected 1 argument"));
    }
    let url = str_arg(&args[0], "http_delete")?;
    let (status, body) = http_request("DELETE", &url, None)?;
    LAST_STATUS.with(|s| *s.borrow_mut() = status);
    Ok(Value::Str(Rc::new(body)))
}

fn native_http_status(_args: Vec<Value>) -> VmResult<Value> {
    Ok(Value::Int(LAST_STATUS.with(|s| *s.borrow())))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn str_arg(val: &Value, ctx: &str) -> VmResult<String> {
    match val {
        Value::Str(s) => Ok(s.as_str().to_string()),
        _ => Err(VmError::new(format!("{}: argument must be str", ctx))),
    }
}

fn status_reason(code: i64) -> &'static str {
    match code {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        503 => "Service Unavailable",
        _   => "Unknown",
    }
}
