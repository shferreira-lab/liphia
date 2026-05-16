// stdlib/native/src/db.rs
//
// Database driver — SQLite via rusqlite + PostgreSQL via TCP (protocolo v3).
//
// ── Cargo.toml (stdlib/native/Cargo.toml) ────────────────────────────────────
//
//   [dependencies]
//   rusqlite = { version = "0.31", features = ["bundled"] }
//
// "bundled" compila o SQLite C junto — sem dependência de instalação do sistema.
//
// ── Functions registered ──────────────────────────────────────────────────────
//
//   db_open(path: str)                     → int    connection handle (SQLite file)
//   db_open_memory()                       → int    in-memory SQLite
//   db_close(handle: int)                  → bool
//   db_exec(handle: int, sql: str)         → int    rows affected
//   db_query(handle: int, sql: str)        → list   flat [col,val,col,val,...]
//   db_query_rows(handle: int, sql: str)   → list   list of flat-row lists
//   db_last_id(handle: int)                → int    last inserted rowid
//   db_begin(handle: int)                  → bool
//   db_commit(handle: int)                 → bool
//   db_rollback(handle: int)               → bool
//   db_error(handle: int)                  → str    last error message
//   db_tables(handle: int)                 → list   table names
//   db_columns(handle: int, table: str)    → list   column names
//
//   pg_connect(host,port,user,pass,db)     → int    PG connection handle
//   pg_exec(handle: int, sql: str)         → int    rows affected
//   pg_query(handle: int, sql: str)        → list   flat [col,val,...]
//   pg_query_rows(handle: int, sql: str)   → list   list of flat-row lists
//   pg_last_id(handle: int)                → int    last inserted id (RETURNING)
//   pg_begin(handle: int)                  → bool
//   pg_commit(handle: int)                 → bool
//   pg_rollback(handle: int)               → bool
//   pg_close(handle: int)                  → bool
//   pg_error(handle: int)                  → str
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use rusqlite::{Connection as SqliteConnection, types::ValueRef};
use liphia_virtual_machine::value::Value;
use liphia_virtual_machine::vm::{VmError, VmResult, VM};
// ── Registration ──────────────────────────────────────────────────────────────
pub fn register(vm: &mut VM) {
    // SQLite
    vm.register_native("db_open",        native_db_open);
    vm.register_native("db_open_memory", native_db_open_memory);
    vm.register_native("db_close",       native_db_close);
    vm.register_native("db_exec",        native_db_exec);
    vm.register_native("db_query",       native_db_query);
    vm.register_native("db_query_rows",  native_db_query_rows);
    vm.register_native("db_last_id",     native_db_last_id);
    vm.register_native("db_begin",       native_db_begin);
    vm.register_native("db_commit",      native_db_commit);
    vm.register_native("db_rollback",    native_db_rollback);
    vm.register_native("db_error",       native_db_error);
    vm.register_native("db_tables",      native_db_tables);
    vm.register_native("db_columns",     native_db_columns);
    // PostgreSQL
    vm.register_native("pg_connect",     native_pg_connect);
    vm.register_native("pg_exec",        native_pg_exec);
    vm.register_native("pg_query",       native_pg_query);
    vm.register_native("pg_query_rows",  native_pg_query_rows);
    vm.register_native("pg_last_id",     native_pg_last_id);
    vm.register_native("pg_begin",       native_pg_begin);
    vm.register_native("pg_commit",      native_pg_commit);
    vm.register_native("pg_rollback",    native_pg_rollback);
    vm.register_native("pg_close",       native_pg_close);
    vm.register_native("pg_error",       native_pg_error);
}
// ─────────────────────────────────────────────────────────────────────────────
// Connection registry
// ─────────────────────────────────────────────────────────────────────────────
thread_local! {
    static SQLITE_CONNS: RefCell<HashMap<i64, SqliteConn>> = RefCell::new(HashMap::new());
    static PG_CONNS:     RefCell<HashMap<i64, PgConn>>     = RefCell::new(HashMap::new());
    static NEXT_HANDLE:  RefCell<i64>                      = RefCell::new(1);
}
fn alloc_handle() -> i64 {
    NEXT_HANDLE.with(|h| {
        let mut h = h.borrow_mut();
        let id = *h;
        *h += 1;
        id
    })
}
// ─────────────────────────────────────────────────────────────────────────────
// SQLite connection wrapper
// ─────────────────────────────────────────────────────────────────────────────
struct SqliteConn {
    conn:       SqliteConnection,
    last_id:    i64,
    last_error: String,
    in_txn:     bool,
}
impl SqliteConn {
    fn open(path: &str) -> Result<Self, String> {
        let conn = SqliteConnection::open(path)
            .map_err(|e| e.to_string())?;
        // Enable WAL for better concurrency
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| e.to_string())?;
        Ok(Self { conn, last_id: 0, last_error: String::new(), in_txn: false })
    }
    fn open_memory() -> Result<Self, String> {
        let conn = SqliteConnection::open_in_memory()
            .map_err(|e| e.to_string())?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")
            .map_err(|e| e.to_string())?;
        Ok(Self { conn, last_id: 0, last_error: String::new(), in_txn: false })
    }
}
// ── Value conversion ──────────────────────────────────────────────────────────
fn sqlite_value_to_liphia(val: ValueRef) -> Value {
    match val {
        ValueRef::Null        => Value::Null,
        ValueRef::Integer(n)  => Value::Int(n),
        ValueRef::Real(f)     => Value::Float(f),
        ValueRef::Text(s)     => Value::Str(Rc::new(
            std::str::from_utf8(s).unwrap_or("").to_string()
        )),
        ValueRef::Blob(b)     => Value::Str(Rc::new(
            format!("<blob {} bytes>", b.len())
        )),
    }
}
// ── Helper: run a SELECT and return flat list [col, val, col, val, ...] ───────
fn sqlite_query_flat(conn: &SqliteConn, sql: &str) -> Result<Value, String> {
    let mut stmt = conn.conn.prepare(sql)
        .map_err(|e| e.to_string())?;
    let col_names: Vec<String> = stmt.column_names()
        .iter()
        .map(|s| s.to_string())
        .collect();
    let col_count = col_names.len();
    let mut flat  = vec![];
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        for i in 0..col_count {
            flat.push(Value::Str(Rc::new(col_names[i].clone())));
            flat.push(sqlite_value_to_liphia(row.get_ref(i).unwrap_or(ValueRef::Null)));
        }
    }
    Ok(Value::List(Rc::new(RefCell::new(flat))))
}
// ── Helper: run a SELECT and return list of flat-row lists ────────────────────
fn sqlite_query_rows(conn: &SqliteConn, sql: &str) -> Result<Value, String> {
    let mut stmt = conn.conn.prepare(sql)
        .map_err(|e| e.to_string())?;
    let col_names: Vec<String> = stmt.column_names()
        .iter()
        .map(|s| s.to_string())
        .collect();
    let col_count = col_names.len();
    let mut result_rows = vec![];
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let mut flat_row = vec![];
        for i in 0..col_count {
            flat_row.push(Value::Str(Rc::new(col_names[i].clone())));
            flat_row.push(sqlite_value_to_liphia(row.get_ref(i).unwrap_or(ValueRef::Null)));
        }
        result_rows.push(Value::List(Rc::new(RefCell::new(flat_row))));
    }
    Ok(Value::List(Rc::new(RefCell::new(result_rows))))
}
// ─────────────────────────────────────────────────────────────────────────────
// SQLite native functions
// ─────────────────────────────────────────────────────────────────────────────
fn native_db_open(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 1 {
        return Err(VmError::new("db_open(path: str) — expected 1 argument"));
    }
    let path = str_arg(&args[0], "db_open")?;
    let conn = SqliteConn::open(&path)
        .map_err(|e| VmError::new(format!("db_open: {}", e)))?;
    let handle = alloc_handle();
    SQLITE_CONNS.with(|c| c.borrow_mut().insert(handle, conn));
    Ok(Value::Int(handle))
}
fn native_db_open_memory(_args: Vec<Value>) -> VmResult<Value> {
    let conn = SqliteConn::open_memory()
        .map_err(|e| VmError::new(format!("db_open_memory: {}", e)))?;
    let handle = alloc_handle();
    SQLITE_CONNS.with(|c| c.borrow_mut().insert(handle, conn));
    Ok(Value::Int(handle))
}
fn native_db_close(args: Vec<Value>) -> VmResult<Value> {
    let handle = int_arg(&args, 0, "db_close")?;
    let removed = SQLITE_CONNS.with(|c| c.borrow_mut().remove(&handle).is_some());
    Ok(Value::Bool(removed))
}
fn native_db_exec(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 2 {
        return Err(VmError::new("db_exec(handle: int, sql: str) — expected 2 arguments"));
    }
    let handle = int_arg(&args, 0, "db_exec")?;
    let sql    = str_arg(&args[1], "db_exec")?;
    SQLITE_CONNS.with(|c| {
        let mut map = c.borrow_mut();
        let conn = map.get_mut(&handle)
            .ok_or_else(|| VmError::new(format!("db_exec: invalid handle {}", handle)))?;
        match conn.conn.execute(&sql, []) {
            Ok(affected) => {
                conn.last_id    = conn.conn.last_insert_rowid();
                conn.last_error = String::new();
                Ok(Value::Int(affected as i64))
            }
            Err(e) => {
                conn.last_error = e.to_string();
                Err(VmError::new(format!("db_exec: {}", e)))
            }
        }
    })
}
fn native_db_query(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 2 {
        return Err(VmError::new("db_query(handle: int, sql: str) — expected 2 arguments"));
    }
    let handle = int_arg(&args, 0, "db_query")?;
    let sql    = str_arg(&args[1], "db_query")?;
    SQLITE_CONNS.with(|c| {
        let mut map = c.borrow_mut();
        let conn = map.get_mut(&handle)
            .ok_or_else(|| VmError::new(format!("db_query: invalid handle {}", handle)))?;
        sqlite_query_flat(conn, &sql).map_err(|e| {
            conn.last_error = e.clone();
            VmError::new(format!("db_query: {}", e))
        })
    })
}
fn native_db_query_rows(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 2 {
        return Err(VmError::new("db_query_rows(handle: int, sql: str) — expected 2 arguments"));
    }
    let handle = int_arg(&args, 0, "db_query_rows")?;
    let sql    = str_arg(&args[1], "db_query_rows")?;
    SQLITE_CONNS.with(|c| {
        let mut map = c.borrow_mut();
        let conn = map.get_mut(&handle)
            .ok_or_else(|| VmError::new(format!("db_query_rows: invalid handle {}", handle)))?;
        sqlite_query_rows(conn, &sql).map_err(|e| {
            conn.last_error = e.clone();
            VmError::new(format!("db_query_rows: {}", e))
        })
    })
}
fn native_db_last_id(args: Vec<Value>) -> VmResult<Value> {
    let handle = int_arg(&args, 0, "db_last_id")?;
    SQLITE_CONNS.with(|c| {
        let map = c.borrow();
        let conn = map.get(&handle)
            .ok_or_else(|| VmError::new(format!("db_last_id: invalid handle {}", handle)))?;
        Ok(Value::Int(conn.last_id))
    })
}
fn native_db_begin(args: Vec<Value>) -> VmResult<Value> {
    let handle = int_arg(&args, 0, "db_begin")?;
    SQLITE_CONNS.with(|c| {
        let mut map = c.borrow_mut();
        let conn = map.get_mut(&handle)
            .ok_or_else(|| VmError::new(format!("db_begin: invalid handle {}", handle)))?;
        conn.conn.execute_batch("BEGIN")
            .map_err(|e| VmError::new(format!("db_begin: {}", e)))?;
        conn.in_txn = true;
        Ok(Value::Bool(true))
    })
}
fn native_db_commit(args: Vec<Value>) -> VmResult<Value> {
    let handle = int_arg(&args, 0, "db_commit")?;
    SQLITE_CONNS.with(|c| {
        let mut map = c.borrow_mut();
        let conn = map.get_mut(&handle)
            .ok_or_else(|| VmError::new(format!("db_commit: invalid handle {}", handle)))?;
        conn.conn.execute_batch("COMMIT")
            .map_err(|e| VmError::new(format!("db_commit: {}", e)))?;
        conn.in_txn = false;
        Ok(Value::Bool(true))
    })
}
fn native_db_rollback(args: Vec<Value>) -> VmResult<Value> {
    let handle = int_arg(&args, 0, "db_rollback")?;
    SQLITE_CONNS.with(|c| {
        let mut map = c.borrow_mut();
        let conn = map.get_mut(&handle)
            .ok_or_else(|| VmError::new(format!("db_rollback: invalid handle {}", handle)))?;
        conn.conn.execute_batch("ROLLBACK")
            .map_err(|e| VmError::new(format!("db_rollback: {}", e)))?;
        conn.in_txn = false;
        Ok(Value::Bool(true))
    })
}
fn native_db_error(args: Vec<Value>) -> VmResult<Value> {
    let handle = int_arg(&args, 0, "db_error")?;
    SQLITE_CONNS.with(|c| {
        let map = c.borrow();
        let conn = map.get(&handle)
            .ok_or_else(|| VmError::new(format!("db_error: invalid handle {}", handle)))?;
        Ok(Value::Str(Rc::new(conn.last_error.clone())))
    })
}
fn native_db_tables(args: Vec<Value>) -> VmResult<Value> {
    let handle = int_arg(&args, 0, "db_tables")?;
    SQLITE_CONNS.with(|c| {
        let map = c.borrow();
        let conn = map.get(&handle)
            .ok_or_else(|| VmError::new(format!("db_tables: invalid handle {}", handle)))?;
        let result = sqlite_query_flat(conn,
            "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
        ).map_err(|e| VmError::new(format!("db_tables: {}", e)))?;
        Ok(result)
    })
}
fn native_db_columns(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 2 {
        return Err(VmError::new("db_columns(handle: int, table: str) — expected 2 arguments"));
    }
    let handle = int_arg(&args, 0, "db_columns")?;
    let table  = str_arg(&args[1], "db_columns")?;
    SQLITE_CONNS.with(|c| {
        let map = c.borrow();
        let conn = map.get(&handle)
            .ok_or_else(|| VmError::new(format!("db_columns: invalid handle {}", handle)))?;
        let sql = format!("PRAGMA table_info({})", table);
        let result = sqlite_query_flat(conn, &sql)
            .map_err(|e| VmError::new(format!("db_columns: {}", e)))?;
        Ok(result)
    })
}
// ─────────────────────────────────────────────────────────────────────────────
// PostgreSQL — wire protocol v3 (pure TCP, no external crate)
// ─────────────────────────────────────────────────────────────────────────────
//
// Implements enough of the Postgres Frontend/Backend Protocol (v3) to:
//   - Authenticate (cleartext password or trust)
//   - Execute simple queries (Simple Query protocol)
//   - Parse RowDescription + DataRow messages
//   - Handle CommandComplete and ErrorResponse
//
// Does NOT support: TLS, SCRAM, prepared statements, COPY, streaming replication.
// For production use, add the postgres crate later.
use std::io::{Read, Write};
use std::net::TcpStream;
struct PgConn {
    stream:     TcpStream,
    last_id:    i64,
    last_error: String,
}
// ── Protocol helpers ──────────────────────────────────────────────────────────
fn pg_write_startup(stream: &mut TcpStream, user: &str, db: &str) -> Result<(), String> {
    // StartupMessage: length(4) + protocol(4) + "user\0<user>\0database\0<db>\0\0"
    let mut payload = vec![];
    payload.extend_from_slice(&196608u32.to_be_bytes()); // protocol 3.0
    payload.extend_from_slice(b"user\0");
    payload.extend_from_slice(user.as_bytes());
    payload.push(0);
    payload.extend_from_slice(b"database\0");
    payload.extend_from_slice(db.as_bytes());
    payload.push(0);
    payload.push(0); // terminator
    let len = (payload.len() + 4) as u32;
    stream.write_all(&len.to_be_bytes()).map_err(|e| e.to_string())?;
    stream.write_all(&payload).map_err(|e| e.to_string())?;
    Ok(())
}
fn pg_read_msg(stream: &mut TcpStream) -> Result<(u8, Vec<u8>), String> {
    let mut tag = [0u8; 1];
    stream.read_exact(&mut tag).map_err(|e| e.to_string())?;
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).map_err(|e| e.to_string())?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len < 4 { return Err("invalid message length".to_string()); }
    let mut body = vec![0u8; len - 4];
    stream.read_exact(&mut body).map_err(|e| e.to_string())?;
    Ok((tag[0], body))
}
fn pg_write_msg(stream: &mut TcpStream, tag: u8, body: &[u8]) -> Result<(), String> {
    stream.write_all(&[tag]).map_err(|e| e.to_string())?;
    let len = (body.len() + 4) as u32;
    stream.write_all(&len.to_be_bytes()).map_err(|e| e.to_string())?;
    stream.write_all(body).map_err(|e| e.to_string())?;
    Ok(())
}
fn pg_send_password(stream: &mut TcpStream, password: &str) -> Result<(), String> {
    let mut body = password.as_bytes().to_vec();
    body.push(0);
    pg_write_msg(stream, b'p', &body)
}
fn pg_send_query(stream: &mut TcpStream, sql: &str) -> Result<(), String> {
    let mut body = sql.as_bytes().to_vec();
    body.push(0);
    pg_write_msg(stream, b'Q', &body)
}
fn pg_connect_inner(host: &str, port: u16, user: &str, password: &str, db: &str)
    -> Result<TcpStream, String>
{
    let addr = format!("{}:{}", host, port);
    let mut stream = TcpStream::connect(&addr)
        .map_err(|e| format!("pg_connect: cannot connect to {}: {}", addr, e))?;
    pg_write_startup(&mut stream, user, db)?;
    // Auth handshake
    loop {
        let (tag, body) = pg_read_msg(&mut stream)?;
        match tag {
            b'R' => {
                if body.len() < 4 { return Err("bad AuthenticationRequest".to_string()); }
                let auth_type = u32::from_be_bytes([body[0], body[1], body[2], body[3]]);
                match auth_type {
                    0 => {} // AuthenticationOk — trust, no password needed
                    3 => {
                        // CleartextPassword
                        pg_send_password(&mut stream, password)?;
                    }
                    _ => return Err(format!(
                        "pg_connect: unsupported auth method {} (only trust/cleartext supported)",
                        auth_type
                    )),
                }
            }
            b'E' => {
                let msg = pg_error_message(&body);
                return Err(format!("pg_connect: auth error: {}", msg));
            }
            b'Z' => break, // ReadyForQuery — connected
            b'S' => {}     // ParameterStatus — ignore
            b'K' => {}     // BackendKeyData — ignore
            _    => {}     // ignore unknown
        }
    }
    Ok(stream)
}
fn pg_error_message(body: &[u8]) -> String {
    // ErrorResponse fields: byte(field_type) + str(value) + \0, terminated by \0
    let mut msg = String::new();
    let mut i = 0;
    while i < body.len() {
        let field = body[i]; i += 1;
        if field == 0 { break; }
        let start = i;
        while i < body.len() && body[i] != 0 { i += 1; }
        let val = std::str::from_utf8(&body[start..i]).unwrap_or("?");
        i += 1; // skip \0
        if field == b'M' { // Message field
            msg = val.to_string();
            break;
        }
    }
    msg
}
// ── Execute a simple query; returns (affected_rows, col_names, data_rows) ─────
fn pg_simple_query(stream: &mut TcpStream, sql: &str)
    -> Result<(i64, Vec<String>, Vec<Vec<Option<String>>>), String>
{
    pg_send_query(stream, sql)?;
    let mut col_names: Vec<String> = vec![];
    let mut data_rows: Vec<Vec<Option<String>>> = vec![];
    let mut affected: i64 = 0;
    loop {
        let (tag, body) = pg_read_msg(stream)?;
        match tag {
            // RowDescription
            b'T' => {
                if body.len() < 2 { continue; }
                let field_count = u16::from_be_bytes([body[0], body[1]]) as usize;
                let mut pos = 2;
                for _ in 0..field_count {
                    // column name: null-terminated string
                    let start = pos;
                    while pos < body.len() && body[pos] != 0 { pos += 1; }
                    let name = std::str::from_utf8(&body[start..pos]).unwrap_or("?").to_string();
                    col_names.push(name);
                    pos += 1;  // skip null
                    pos += 18; // table OID(4) + col attr(2) + type OID(4) + type size(2)
                               // + type modifier(4) + format code(2)
                }
            }
            // DataRow
            b'D' => {
                if body.len() < 2 { continue; }
                let field_count = u16::from_be_bytes([body[0], body[1]]) as usize;
                let mut pos = 2;
                let mut row = vec![];
                for _ in 0..field_count {
                    if pos + 4 > body.len() { row.push(None); continue; }
                    let len = i32::from_be_bytes([
                        body[pos], body[pos+1], body[pos+2], body[pos+3]
                    ]);
                    pos += 4;
                    if len < 0 {
                        row.push(None); // NULL
                    } else {
                        let len = len as usize;
                        let val = std::str::from_utf8(&body[pos..pos+len])
                            .unwrap_or("")
                            .to_string();
                        row.push(Some(val));
                        pos += len;
                    }
                }
                data_rows.push(row);
            }
            // CommandComplete: "INSERT 0 1", "UPDATE 3", "SELECT 5", etc.
            b'C' => {
                let s = std::str::from_utf8(&body).unwrap_or("").trim_end_matches('\0');
                let parts: Vec<&str> = s.split_whitespace().collect();
                if let Some(last) = parts.last() {
                    affected = last.parse().unwrap_or(0);
                }
            }
            b'E' => {
                let msg = pg_error_message(&body);
                return Err(msg);
            }
            b'Z' => break, // ReadyForQuery
            b'I' => break, // EmptyQueryResponse
            _    => {}
        }
    }
    Ok((affected, col_names, data_rows))
}
// ── Rows to flat Liphia list ──────────────────────────────────────────────────
fn pg_rows_to_flat(col_names: &[String], data_rows: &[Vec<Option<String>>]) -> Value {
    let mut flat = vec![];
    for row in data_rows {
        for (i, val) in row.iter().enumerate() {
            flat.push(Value::Str(Rc::new(
                col_names.get(i).cloned().unwrap_or_default()
            )));
            match val {
                None    => flat.push(Value::Null),
                Some(s) => flat.push(Value::Str(Rc::new(s.clone()))),
            }
        }
    }
    Value::List(Rc::new(RefCell::new(flat)))
}
fn pg_rows_to_row_list(col_names: &[String], data_rows: &[Vec<Option<String>>]) -> Value {
    let mut rows = vec![];
    for row in data_rows {
        let mut flat_row = vec![];
        for (i, val) in row.iter().enumerate() {
            flat_row.push(Value::Str(Rc::new(
                col_names.get(i).cloned().unwrap_or_default()
            )));
            match val {
                None    => flat_row.push(Value::Null),
                Some(s) => flat_row.push(Value::Str(Rc::new(s.clone()))),
            }
        }
        rows.push(Value::List(Rc::new(RefCell::new(flat_row))));
    }
    Value::List(Rc::new(RefCell::new(rows)))
}
// ── PostgreSQL native functions ───────────────────────────────────────────────
fn native_pg_connect(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 5 {
        return Err(VmError::new(
            "pg_connect(host, port, user, pass, db) — expected 5 arguments"
        ));
    }
    let host = str_arg(&args[0], "pg_connect")?;
    let port = int_arg(&args, 1, "pg_connect")? as u16;
    let user = str_arg(&args[2], "pg_connect")?;
    let pass = str_arg(&args[3], "pg_connect")?;
    let db   = str_arg(&args[4], "pg_connect")?;
    let stream = pg_connect_inner(&host, port, &user, &pass, &db)
        .map_err(|e| VmError::new(e))?;
    let handle = alloc_handle();
    PG_CONNS.with(|c| c.borrow_mut().insert(handle, PgConn {
        stream,
        last_id:    0,
        last_error: String::new(),
    }));
    Ok(Value::Int(handle))
}
fn native_pg_exec(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 2 {
        return Err(VmError::new("pg_exec(handle: int, sql: str) — expected 2 arguments"));
    }
    let handle = int_arg(&args, 0, "pg_exec")?;
    let sql    = str_arg(&args[1], "pg_exec")?;
    PG_CONNS.with(|c| {
        let mut map = c.borrow_mut();
        let conn = map.get_mut(&handle)
            .ok_or_else(|| VmError::new(format!("pg_exec: invalid handle {}", handle)))?;
        match pg_simple_query(&mut conn.stream, &sql) {
            Ok((affected, _, _)) => {
                conn.last_error = String::new();
                Ok(Value::Int(affected))
            }
            Err(e) => {
                conn.last_error = e.clone();
                Err(VmError::new(format!("pg_exec: {}", e)))
            }
        }
    })
}
fn native_pg_query(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 2 {
        return Err(VmError::new("pg_query(handle: int, sql: str) — expected 2 arguments"));
    }
    let handle = int_arg(&args, 0, "pg_query")?;
    let sql    = str_arg(&args[1], "pg_query")?;
    PG_CONNS.with(|c| {
        let mut map = c.borrow_mut();
        let conn = map.get_mut(&handle)
            .ok_or_else(|| VmError::new(format!("pg_query: invalid handle {}", handle)))?;
        match pg_simple_query(&mut conn.stream, &sql) {
            Ok((_, cols, rows)) => {
                conn.last_error = String::new();
                Ok(pg_rows_to_flat(&cols, &rows))
            }
            Err(e) => {
                conn.last_error = e.clone();
                Err(VmError::new(format!("pg_query: {}", e)))
            }
        }
    })
}
fn native_pg_query_rows(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 2 {
        return Err(VmError::new("pg_query_rows(handle: int, sql: str) — expected 2 arguments"));
    }
    let handle = int_arg(&args, 0, "pg_query_rows")?;
    let sql    = str_arg(&args[1], "pg_query_rows")?;
    PG_CONNS.with(|c| {
        let mut map = c.borrow_mut();
        let conn = map.get_mut(&handle)
            .ok_or_else(|| VmError::new(format!("pg_query_rows: invalid handle {}", handle)))?;
        match pg_simple_query(&mut conn.stream, &sql) {
            Ok((_, cols, rows)) => {
                conn.last_error = String::new();
                Ok(pg_rows_to_row_list(&cols, &rows))
            }
            Err(e) => {
                conn.last_error = e.clone();
                Err(VmError::new(format!("pg_query_rows: {}", e)))
            }
        }
    })
}
fn native_pg_last_id(args: Vec<Value>) -> VmResult<Value> {
    let handle = int_arg(&args, 0, "pg_last_id")?;
    PG_CONNS.with(|c| {
        let map = c.borrow();
        let conn = map.get(&handle)
            .ok_or_else(|| VmError::new(format!("pg_last_id: invalid handle {}", handle)))?;
        Ok(Value::Int(conn.last_id))
    })
}
fn native_pg_begin(args: Vec<Value>) -> VmResult<Value> {
    let handle = int_arg(&args, 0, "pg_begin")?;
    PG_CONNS.with(|c| {
        let mut map = c.borrow_mut();
        let conn = map.get_mut(&handle)
            .ok_or_else(|| VmError::new(format!("pg_begin: invalid handle {}", handle)))?;
        pg_simple_query(&mut conn.stream, "BEGIN")
            .map_err(|e| VmError::new(format!("pg_begin: {}", e)))?;
        Ok(Value::Bool(true))
    })
}
fn native_pg_commit(args: Vec<Value>) -> VmResult<Value> {
    let handle = int_arg(&args, 0, "pg_commit")?;
    PG_CONNS.with(|c| {
        let mut map = c.borrow_mut();
        let conn = map.get_mut(&handle)
            .ok_or_else(|| VmError::new(format!("pg_commit: invalid handle {}", handle)))?;
        pg_simple_query(&mut conn.stream, "COMMIT")
            .map_err(|e| VmError::new(format!("pg_commit: {}", e)))?;
        Ok(Value::Bool(true))
    })
}
fn native_pg_rollback(args: Vec<Value>) -> VmResult<Value> {
    let handle = int_arg(&args, 0, "pg_rollback")?;
    PG_CONNS.with(|c| {
        let mut map = c.borrow_mut();
        let conn = map.get_mut(&handle)
            .ok_or_else(|| VmError::new(format!("pg_rollback: invalid handle {}", handle)))?;
        pg_simple_query(&mut conn.stream, "ROLLBACK")
            .map_err(|e| VmError::new(format!("pg_rollback: {}", e)))?;
        Ok(Value::Bool(true))
    })
}
fn native_pg_close(args: Vec<Value>) -> VmResult<Value> {
    let handle = int_arg(&args, 0, "pg_close")?;
    // Send Terminate message before dropping
    PG_CONNS.with(|c| {
        let mut map = c.borrow_mut();
        if let Some(conn) = map.get_mut(&handle) {
            let _ = pg_write_msg(&mut conn.stream, b'X', &[]);
        }
        let removed = map.remove(&handle).is_some();
        Ok(Value::Bool(removed))
    })
}
fn native_pg_error(args: Vec<Value>) -> VmResult<Value> {
    let handle = int_arg(&args, 0, "pg_error")?;
    PG_CONNS.with(|c| {
        let map = c.borrow();
        let conn = map.get(&handle)
            .ok_or_else(|| VmError::new(format!("pg_error: invalid handle {}", handle)))?;
        Ok(Value::Str(Rc::new(conn.last_error.clone())))
    })
}
// ─────────────────────────────────────────────────────────────────────────────
// Shared helpers
// ─────────────────────────────────────────────────────────────────────────────
fn str_arg(val: &Value, ctx: &str) -> VmResult<String> {
    match val {
        Value::Str(s) => Ok(s.as_str().to_string()),
        _ => Err(VmError::new(format!("{}: argument must be str", ctx))),
    }
}
fn int_arg(args: &[Value], idx: usize, ctx: &str) -> VmResult<i64> {
    match args.get(idx) {
        Some(Value::Int(n)) => Ok(*n),
        _ => Err(VmError::new(format!("{}: argument {} must be int", ctx, idx))),
    }
}