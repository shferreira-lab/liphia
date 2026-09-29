// liphia_core_native/src/http.rs
//
// HTTP/1.1 server (non-blocking accept) + HTTP client — no external deps.
//
// ── Server workflow (use inside async fn + await loop) ────────────────────────
//
//   http_listen(port: int)               → bool   bind port, spawn accept thread
//   http_accept()                         → bool   true if a new request is ready
//                                                  false if queue empty  (await-able)
//   http_method()                         → str    "GET" | "POST" | "PUT" | ...
//   http_path()                           → str    "/users/42"
//   http_query()                          → str    "page=1&limit=10"  (after '?')
//   http_body()                           → str    request body
//   http_header(name: str)                → str    single header value (lowercase key)
//   http_cookie(name: str)                → str    one cookie from the Cookie header
//   http_set_header(name: str, value: str)→ bool   add a header to the response
//   http_respond(status, body)            → bool   text/plain response + close conn
//   http_respond_json(status, body)       → bool   application/json  response
//
// ── Client ───────────────────────────────────────────────────────────────────
//
//   http_get(url)                         → str    response body
//   http_post(url, body)                  → str
//   http_put(url, body)                   → str
//   http_patch(url, body)                 → str
//   http_delete(url)                      → str
//   http_status()                         → int    last response HTTP status code
//
// Server notes:
//   - Each connection is parsed on its own thread, so a slow client never
//     blocks the others; parsed requests are queued for http_accept.
//   - Request bodies larger than MAX_BODY_BYTES are rejected.
//   - CORS headers are added to every response (Access-Control-Allow-Origin: *)
//     unless the program sets the same header with http_set_header. Cookies
//     across subdomains need that: browsers refuse credentialed requests
//     when the allowed origin is `*`.
//   - http_set_header replaces an earlier value of the same header, except
//     Set-Cookie, which may appear several times. Content-Length and
//     Connection are always computed by the server.
//
// Client notes:
//   - Plain http:// only; https:// URLs are an error.
//   - Requests are sent as HTTP/1.0 so servers never answer with chunked
//     transfer encoding, which this minimal client does not decode.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;

use liphia_virtual_machine::value::Value;
use liphia_virtual_machine::vm::{VmError, VmResult, VM};

use crate::util::{expect_args, port_arg};

// ── State ─────────────────────────────────────────────────────────────────────

const MAX_BODY_BYTES: usize = 10 * 1024 * 1024;

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
    // Set by http_set_header, written by the next respond call.
    response_headers: Vec<(String, String)>,
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
    vm.register_native("http_cookie",       native_http_cookie);
    vm.register_native("http_set_header",   native_http_set_header);
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
    expect_args("http_listen", &args, 1)?;
    let port = port_arg(&args[0], "http_listen")?;
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
        .map_err(|e| VmError::new(format!("http_listen(): bind failed on port {}: {}", port, e)))?;

    let state = Arc::new(HttpServerState {
        pending: Mutex::new(VecDeque::new()),
    });

    SERVER_STATE.with(|s| *s.borrow_mut() = Some(state.clone()));

    eprintln!("[liphia/http] listening on port {}", port);

    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(mut s) => {
                    let state = state.clone();
                    thread::spawn(move || match parse_request(&mut s) {
                        Ok(req) => state.pending.lock().unwrap().push_back(req),
                        Err(e) => eprintln!("[liphia/http] parse error: {}", e),
                    });
                }
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
        None => Ok(Value::Bool(false)),
        Some(r) => {
            CURRENT.with(|c| {
                *c.borrow_mut() = Some(CurrentRequest {
                    method:  r.method,
                    path:    r.path,
                    query:   r.query,
                    headers: r.headers,
                    body:    r.body,
                    stream:  r.stream,
                    response_headers: vec![],
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

// The value of cookie `name` in the request's Cookie header ("a=1; b=2"),
// or "" when absent. Cookie names are case-sensitive.
fn native_http_cookie(args: Vec<Value>) -> VmResult<Value> {
    expect_args("http_cookie", &args, 1)?;
    let name = str_arg(&args[0], "http_cookie")?;
    let header = CURRENT.with(|c| {
        c.borrow()
            .as_ref()
            .and_then(|r| r.headers.get("cookie").cloned())
            .unwrap_or_default()
    });
    Ok(Value::Str(Rc::new(find_cookie(&header, &name))))
}

fn find_cookie(header: &str, name: &str) -> String {
    header
        .split(';')
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(k, _)| k.trim() == name)
        .map(|(_, v)| v.trim().trim_matches('"').to_string())
        .unwrap_or_default()
}

// Adds a header to the response of the current request. Rejects names
// that are not HTTP tokens and values containing CR or LF, so request data
// copied into a header can never inject extra headers or a second response.
fn native_http_set_header(args: Vec<Value>) -> VmResult<Value> {
    expect_args("http_set_header", &args, 2)?;
    let name = str_arg(&args[0], "http_set_header")?;
    let value = str_arg(&args[1], "http_set_header")?;
    validate_header(&name, &value)?;

    CURRENT.with(|c| {
        let mut opt = c.borrow_mut();
        let req = opt.as_mut().ok_or_else(|| {
            VmError::new("http_set_header: no active request — call http_accept first")
        })?;
        if !name.eq_ignore_ascii_case("set-cookie") {
            req.response_headers.retain(|(k, _)| !k.eq_ignore_ascii_case(&name));
        }
        req.response_headers.push((name, value));
        Ok(Value::Bool(true))
    })
}

fn validate_header(name: &str, value: &str) -> VmResult<()> {
    let is_token = |c: char| c.is_ascii_alphanumeric() || "!#$%&'*+-.^_`|~".contains(c);
    if name.is_empty() || !name.chars().all(is_token) {
        return Err(VmError::new(format!("http_set_header: invalid header name '{}'", name)));
    }
    if name.eq_ignore_ascii_case("content-length") || name.eq_ignore_ascii_case("connection") {
        return Err(VmError::new(format!(
            "http_set_header: '{}' is set by the server and cannot be changed",
            name
        )));
    }
    if value.contains('\r') || value.contains('\n') {
        return Err(VmError::new(format!(
            "http_set_header: the value of '{}' contains a line break",
            name
        )));
    }
    Ok(())
}

// ── Server: respond ───────────────────────────────────────────────────────────

// Defaults every response carries unless the program set the same header.
const DEFAULT_HEADERS: &[(&str, &str)] = &[
    ("Access-Control-Allow-Origin", "*"),
    ("Access-Control-Allow-Methods", "GET, POST, PUT, PATCH, DELETE, OPTIONS"),
    ("Access-Control-Allow-Headers", "Content-Type, Authorization, X-Requested-With"),
    ("Access-Control-Max-Age", "86400"),
];

fn build_response(
    status: i64,
    body: &str,
    content_type: &str,
    custom: &[(String, String)],
) -> String {
    let is_custom = |name: &str| custom.iter().any(|(k, _)| k.eq_ignore_ascii_case(name));
    let mut head = format!("HTTP/1.1 {} {}\r\n", status, status_reason(status));
    if !is_custom("Content-Type") {
        head.push_str(&format!("Content-Type: {}\r\n", content_type));
    }
    for (name, value) in DEFAULT_HEADERS {
        if !is_custom(name) {
            head.push_str(&format!("{}: {}\r\n", name, value));
        }
    }
    for (name, value) in custom {
        head.push_str(&format!("{}: {}\r\n", name, value));
    }
    head.push_str(&format!("Content-Length: {}\r\nConnection: close\r\n\r\n", body.len()));
    head.push_str(body);
    head
}

fn send_response(status: i64, body: &str, content_type: &str) -> VmResult<Value> {

    let result = CURRENT.with(|c| {
        let mut opt = c.borrow_mut();
        if let Some(ref mut req) = *opt {
            let response = build_response(status, body, content_type, &req.response_headers);
            req.stream
                .write_all(response.as_bytes())
                .map(|_| Value::Bool(true))
                .map_err(|e| VmError::new(format!("http_respond: write failed: {}", e)))
        } else {
            Err(VmError::new(
                "http_respond: no active request — call http_accept first",
            ))
        }
    });

    if result.is_ok() {
        CURRENT.with(|c| *c.borrow_mut() = None);
    }
    result
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

    let mut first_line = String::new();
    reader.read_line(&mut first_line).map_err(|e| e.to_string())?;
    let parts: Vec<&str> = first_line.trim().splitn(3, ' ').collect();
    if parts.len() < 2 {
        return Err(format!("malformed request line: {:?}", first_line));
    }
    let method    = parts[0].to_string();
    let full_path = parts[1].to_string();

    let (path, query) = if let Some(idx) = full_path.find('?') {
        (full_path[..idx].to_string(), full_path[idx+1..].to_string())
    } else {
        (full_path, String::new())
    };

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
                if content_length > MAX_BODY_BYTES {
                    return Err(format!("request body too large ({} bytes)", content_length));
                }
            }
            headers.insert(key, val);
        }
    }

    let mut body_bytes = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body_bytes).map_err(|e| e.to_string())?;
    }
    let body = String::from_utf8_lossy(&body_bytes).to_string();

    let write_stream = stream.try_clone().map_err(|e| e.to_string())?;
    Ok(PendingRequest { method, path, query, headers, body, stream: write_stream })
}

// ── HTTP client ───────────────────────────────────────────────────────────────

fn http_request(method: &str, url: &str, body: Option<&str>) -> VmResult<(i64, String)> {
    if url.starts_with("https://") {
        return Err(VmError::new(format!("http: https is not supported ('{}')", url)));
    }
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
        "{} {} HTTP/1.0\r\nHost: {}\r\nContent-Length: {}\r\n{}Connection: close\r\n\r\n{}",
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
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        304 => "Not Modified",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
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

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Value {
        Value::Str(Rc::new(s.to_string()))
    }

    #[test]
    fn cookie_lookup() {
        let header = "theme=dark; session=abc123; empty=; quoted=\"x y\"";
        assert_eq!(find_cookie(header, "session"), "abc123");
        assert_eq!(find_cookie(header, "theme"), "dark");
        assert_eq!(find_cookie(header, "empty"), "");
        assert_eq!(find_cookie(header, "quoted"), "x y");
        assert_eq!(find_cookie(header, "Session"), "");
        assert_eq!(find_cookie("", "session"), "");
    }

    #[test]
    fn custom_headers_override_defaults_and_cookies_repeat() {
        let custom = vec![
            ("Access-Control-Allow-Origin".to_string(), "https://loophase.com".to_string()),
            ("Access-Control-Allow-Credentials".to_string(), "true".to_string()),
            ("Set-Cookie".to_string(), "a=1; Path=/".to_string()),
            ("Set-Cookie".to_string(), "b=2; Path=/".to_string()),
        ];
        let r = build_response(302, "", "text/plain; charset=utf-8", &custom);
        assert!(r.starts_with("HTTP/1.1 302 Found\r\n"));
        assert!(r.contains("Access-Control-Allow-Origin: https://loophase.com\r\n"));
        assert!(!r.contains("Access-Control-Allow-Origin: *"));
        assert_eq!(r.matches("Set-Cookie:").count(), 2);
        assert!(r.contains("Access-Control-Allow-Methods:"));
        assert!(r.ends_with("Content-Length: 0\r\nConnection: close\r\n\r\n"));
    }

    #[test]
    fn header_injection_is_rejected() {
        assert!(validate_header("X-Ok", "value").is_ok());
        assert!(validate_header("X-Bad", "a\r\nSet-Cookie: evil=1").is_err());
        assert!(validate_header("Bad Name", "v").is_err());
        assert!(validate_header("Content-Length", "5").is_err());
    }

    // End to end over a real socket: the program reads a cookie, sets two
    // cookies and a specific allowed origin, and the client sees exactly
    // that.
    #[test]
    fn set_header_reaches_the_client() {
        let port = 38417;
        native_http_listen(vec![Value::Int(port)]).unwrap();

        let client = thread::spawn(move || {
            let mut stream = loop {
                if let Ok(s) = TcpStream::connect(("127.0.0.1", port as u16)) {
                    break s;
                }
                thread::sleep(std::time::Duration::from_millis(20));
            };
            stream
                .write_all(b"GET /me HTTP/1.1\r\nHost: x\r\nCookie: session=abc123\r\n\r\n")
                .unwrap();
            let mut response = String::new();
            stream.read_to_string(&mut response).unwrap();
            response
        });

        while native_http_accept(vec![]).unwrap() != Value::Bool(true) {
            thread::sleep(std::time::Duration::from_millis(10));
        }
        let session = native_http_cookie(vec![text("session")]).unwrap();
        assert_eq!(session, text("abc123"));
        native_http_set_header(vec![text("Set-Cookie"), text("session=new; HttpOnly")]).unwrap();
        native_http_set_header(vec![text("Set-Cookie"), text("theme=dark")]).unwrap();
        native_http_set_header(vec![text("Access-Control-Allow-Origin"), text("https://loophase.com")]).unwrap();
        native_http_respond_json(vec![Value::Int(200), text("{}")]).unwrap();

        let response = client.join().unwrap();
        assert!(response.contains("Set-Cookie: session=new; HttpOnly\r\n"));
        assert!(response.contains("Set-Cookie: theme=dark\r\n"));
        assert!(response.contains("Access-Control-Allow-Origin: https://loophase.com\r\n"));
        assert!(!response.contains("Access-Control-Allow-Origin: *"));

        // The request is closed: headers cannot be set any more.
        assert!(native_http_set_header(vec![text("X-Late"), text("1")]).is_err());
    }
}
