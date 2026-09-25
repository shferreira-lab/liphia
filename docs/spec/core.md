# Liphia core natives

Everything in this document is compiled into the Liphia binary
(`liphia_core_native`) and is callable without `import` or `install`.
Anything else is a package (`src/packages/<name>/`), see
`src/packages/README.md`.

Signatures use Liphia types. `number` means int or float. Every native
checks its argument count and types and raises a runtime error (catchable
with `try`/`catch`) on bad input.

## General rules

- **No implicit int/float mixing.** Functions that return "the same type
  as the arguments" (`min`, `max`, `clamp`, `sum`, `min_list`, `max_list`)
  reject a mix of int and float. Functions that take "a number"
  (`sqrt`, `sin`, `pow`...) accept either, because they convert as part of
  their contract.
- **Integer overflow is an error**, never a wrap-around (`abs`, `gcd`,
  `lcm`, `factorial`, `sum`).
- **NaN and infinity** only appear through `nan()` and `inf()`; functions
  that must produce an int (`floor`, `ceil`, `round`) reject them.
- **I/O failures are errors.** Functions returning `bool` for I/O return
  `true` whenever they return at all.

## Strings and conversions

| Signature | Notes |
|-----------|-------|
| `len(value: any) -> int` | characters of a str, elements of a list or map |
| `to_int(value: any) -> int` | from str, int or float (float truncates) |
| `to_float(value: any) -> float` | from str, int or float |
| `to_str(value: any) -> str` | display form of any value |
| `trim(s: str) -> str` | |
| `upper(s: str) -> str` | |
| `lower(s: str) -> str` | |
| `contains(s: str, sub: str) -> bool` | |
| `starts_with(s: str, prefix: str) -> bool` | |
| `ends_with(s: str, suffix: str) -> bool` | |
| `replace(s: str, from: str, to: str) -> str` | replaces every occurrence |
| `split(s: str, sep: str) -> list` | |

## Lists and maps

| Signature | Notes |
|-----------|-------|
| `append(l: list, value: any)` | in place |
| `pop(l: list) -> any` | removes the last element; null when empty |
| `keys(l: list) -> list` | indices `[0, 1, ..., n-1]` |
| `map_keys(m: map) -> list` | insertion order |
| `map_values(m: map) -> list` | insertion order |
| `map_has(m: map, key: any) -> bool` | |
| `map_remove(m: map, key: any)` | no-op when absent |

## Math

| Signature | Notes |
|-----------|-------|
| `sqrt(x: number) -> float` | x >= 0 |
| `pow(base: number, exp: number) -> float` | |
| `exp(x: number) -> float` | |
| `log(x)`, `log10(x)`, `log2(x) -> float` | x > 0 |
| `log_base(x: number, b: number) -> float` | x > 0, b > 0, b != 1 |
| `sin`, `cos`, `tan`, `atan (x) -> float` | radians |
| `asin`, `acos (x) -> float` | x in [-1, 1] |
| `atan2(y: number, x: number) -> float` | |
| `sinh`, `cosh`, `tanh (x) -> float` | |
| `hypot(a: number, b: number) -> float` | |
| `deg_to_rad(x)`, `rad_to_deg(x) -> float` | |
| `pi()`, `e()`, `inf()`, `nan() -> float` | constants |
| `abs(x: number) -> number` | same type as x |
| `floor(x)`, `ceil(x)`, `round(x) -> int` | int passes through; NaN/inf are errors; round is half away from zero |
| `min(a, b)`, `max(a, b) -> number` | both int or both float |
| `clamp(x, lo, hi) -> number` | all int or all float; lo <= hi |
| `sign(x: number) -> int` | -1, 0 or 1 |
| `factorial(n: int) -> int` | 0 <= n <= 20 |
| `gcd(a: int, b: int) -> int` | always >= 0 |
| `lcm(a: int, b: int) -> int` | 0 if either is 0 |
| `is_nan(x)`, `is_inf(x) -> bool` | false for int |

## Random

SplitMix64, seeded from the system clock at startup. Not cryptographically
secure. Native packages keep their own state and do not see `seed()`.

| Signature | Notes |
|-----------|-------|
| `seed(n: int)` | makes the following sequence reproducible |
| `rand_int(low: int, high: int) -> int` | uniform in [low, high), no modulo bias |
| `rand_uniform(n: int, low: number, high: number) -> list` | n floats in [low, high) |
| `rand_normal(n: int, mean: number, std: number) -> list` | n floats, Box-Muller |
| `shuffle(l: list) -> list` | new list, any element type; the input is not changed |

## Aggregation

The list must be all int or all float.

| Signature | Notes |
|-----------|-------|
| `sum(l: list) -> number` | element type; `sum([])` is `0` |
| `mean(l: list) -> float` | error when empty |
| `min_list(l: list) -> number` | element type; error when empty |
| `max_list(l: list) -> number` | element type; error when empty |

## JSON

| Signature | Notes |
|-----------|-------|
| `json_encode(value: any) -> str` | floats always keep a `.` or exponent; NaN/inf become null; enum variants become their name |
| `json_decode(text: str) -> any` | object -> map, array -> list; strict (trailing content is an error); duplicated keys keep the last value |

Integers without `.` or exponent decode as int, everything else as float,
so a float written by `json_encode` comes back as float.

## Files

| Signature | Notes |
|-----------|-------|
| `read_file(path: str) -> str` | |
| `write_file(path: str, content: str) -> bool` | create or overwrite |
| `append_file(path: str, content: str) -> bool` | create or append |
| `file_exists(path: str) -> bool` | |
| `read_json(path: str) -> any` | read_file + json_decode |
| `write_json(path: str, data: any) -> bool` | json_encode + write_file |
| `append_json_line(path: str, data: any) -> bool` | one JSON document per line (JSON Lines) |

## TCP / UDP

Blocking I/O. Connections are int handles. Ports must be in 1..65535.

| Signature | Notes |
|-----------|-------|
| `tcp_connect(host: str, port: int) -> int` | handle |
| `tcp_send(handle: int, data: str) -> bool` | |
| `tcp_recv(handle: int) -> str` | up to 4096 bytes; `""` when the peer closed |
| `tcp_recv_all(handle: int) -> str` | reads until the peer closes |
| `tcp_close(handle: int) -> bool` | false for an unknown handle |
| `udp_send(host: str, port: int, data: str) -> bool` | |
| `tcp_send_json(handle: int, data: any) -> bool` | json_encode + tcp_send |
| `tcp_recv_json(handle: int) -> any` | tcp_recv_all + json_decode |

## HTTP server

`http_listen` starts a background acceptor; each connection is parsed on
its own thread and queued. `http_accept` is non-blocking, so the usual loop
is `await http_accept()` inside an `async fn`. Bodies over 10 MB are
rejected. Every response carries permissive CORS headers
(`Access-Control-Allow-Origin: *`).

| Signature | Notes |
|-----------|-------|
| `http_listen(port: int) -> bool` | |
| `http_accept() -> bool` | true when a request became current |
| `http_method() -> str` | of the current request |
| `http_path() -> str` | without the query string |
| `http_query() -> str` | text after `?` |
| `http_body() -> str` | |
| `http_header(name: str) -> str` | case-insensitive; `""` when absent |
| `http_respond(status: int, body: str) -> bool` | text/plain, closes the request |
| `http_respond_json(status: int, body: str) -> bool` | application/json, closes the request |

## HTTP client

Plain `http://` only (`https://` is an error). Requests go out as HTTP/1.0,
so responses never use chunked encoding.

| Signature | Notes |
|-----------|-------|
| `http_get(url: str) -> str` | response body |
| `http_post(url: str, body: str) -> str` | sent as application/json |
| `http_put(url: str, body: str) -> str` | |
| `http_patch(url: str, body: str) -> str` | |
| `http_delete(url: str) -> str` | |
| `http_status() -> int` | status of the last client response |

## WebSocket server

Text frames only (RFC 6455). Clients are int handles.

| Signature | Notes |
|-----------|-------|
| `ws_listen(port: int) -> bool` | |
| `ws_accept() -> int` | new client handle, 0 when none is pending |
| `ws_clients() -> list` | handles of connected clients |
| `ws_send(handle: int, msg: str) -> bool` | |
| `ws_recv(handle: int) -> str` | `""` when nothing is pending |
| `ws_broadcast(msg: str) -> bool` | drops clients whose socket fails |
| `ws_close(handle: int) -> bool` | |
| `ws_send_json(handle: int, data: any) -> bool` | json_encode + ws_send |
| `ws_broadcast_json(data: any) -> bool` | json_encode + ws_broadcast |

## Console

`print(...)` and `input(prompt: str) -> str` are builtins of the VM, not
natives.
