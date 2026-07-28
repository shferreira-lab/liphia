# Liphia (.lph)

> A statically typed, indentation-based programming language powered by a Rust bytecode VM.
> Created by Sergio H. Ferreira — started in late 2025.

[![Engine](https://img.shields.io/badge/engine-0.10.0-blueviolet)](https://github.com/shferreira-lab/liphia)
[![Compiler](https://img.shields.io/badge/compiler-0.10.0-blueviolet)](https://github.com/shferreira-lab/liphia)
[![Language](https://img.shields.io/badge/rust-core%20engine-orange)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/status-active%20development-yellow)](https://github.com/shferreira-lab)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](./licenses)

Liphia is a general-purpose programming language with indentation-based syntax
(similar to Python), explicit static typing, and a bytecode compiler and virtual machine
implemented entirely in Rust.

Liphia ships with its own compiler, bytecode format, and runtime VM — no external runtime,
no interpreter dependency, and no garbage collector overhead.
Compiled programs are fully self-contained.

> **Notice:** The project is under active development. Full documentation,
> stable releases, and the standard library will be published when the
> engine reaches sufficient stability.

---

## Table of Contents

- [Repository layout](#repository-layout)
- [How to build](#how-to-build)
- [How to run](#how-to-run)
- [Package manager](#package-manager)
- [Current syntax — Engine 0.10.0](#current-syntax--engine-0100)
  - [Comments](#comments)
  - [Primitive types](#primitive-types)
  - [Variables](#variables)
  - [Input and output](#input-and-output)
  - [Operators](#operators)
  - [Conditionals](#conditionals)
  - [Loops](#loops)
  - [Functions](#functions)
  - [Lists](#lists)
  - [Maps](#maps)
  - [Enums](#enums)
  - [Error handling — try/catch](#error-handling--trycatch)
  - [String interpolation — f-strings](#string-interpolation--f-strings)
  - [Async and concurrency](#async-and-concurrency)
  - [File imports](#file-imports)
  - [Stdlib modules](#stdlib-modules)
- [Core native functions](#core-native-functions)
- [Standard library](#standard-library)
- [Full example](#full-example)
- [Roadmap](#roadmap)
- [License](#license)

---

## Repository layout

```
liphia/
│
├── docs/                          # documentation
├── licenses/                      # licenses
│
├── src/
│   ├── liphia_engine/
│   │   └── crates/
│   │       ├── liphia_cli/                 # CLI runner + REPL shell
│   │       ├── liphia_compiler/            # lexer, parser, AST, bytecode compiler
│   │       ├── liphia_core_native/         # core string, list, map and value utilities
│   │       └── liphia_virtual_machine/     # bytecode VM
│   │
│   ├── stdlib/
│   │   ├── modules/               # .lph modules (import from "...")
│   │   └── native/                # Rust native stdlib bindings
│   │       └── src/
│   │           ├── cdf.rs         # internal CDF / special functions (not exported)
│   │           ├── http.rs        # HTTP server + client (v1.1.0 — native CORS)
│   │           ├── json.rs        # JSON encode/decode (v2.0.0 — objects decode to map)
│   │           ├── math.rs
│   │           ├── stats.rs
│   │           └── ...
│   │
│   ├── tests/                     # tests and benchmarks
│   └── tools/
│       └── liphia-vscode/         # VS Code extension
│
└── README.md
```

---

## How to build

**Requirement:** [Rust](https://rustup.rs/) installed.

The workspace root is `src/`. All build commands must be run from there.

```bash
cd src
cargo build --release -p liphia_cli
```

The binary is produced at:

```
src/target/release/liphia_cli        # Linux / macOS
src/target/release/liphia_cli.exe    # Windows
```

You can copy it to any location on your system and add it to your `PATH`.

**Linux / macOS — add to PATH:**

```bash
sudo cp target/release/liphia_cli /usr/local/bin/liphia
```

**Windows — add to PATH manually or use the installer:**

Copy `liphia_cli.exe` to a folder of your choice (e.g. `C:\liphia\`) and add
that folder to your system `PATH`, or run the official Windows installer which
handles this automatically.

Once in `PATH`, you can call `liphia` from any terminal in any folder.

> **⚠️ Roadmap note:** pre-built binaries for Windows/Linux/macOS via automated
> CI releases are not available yet — see [Roadmap](#roadmap). For now, building
> from source is the only distribution path.

---

## How to run

```bash
liphia path/to/program.lph
```

**Examples:**

```bash
# Linux / macOS
liphia hello.lph

# Windows (PowerShell)
liphia .\hello.lph

# Without PATH configured
.\target\release\liphia_cli.exe hello.lph
```

**Bytecode cache**

The CLI automatically caches compiled bytecode alongside your source file:

```
liphia_cache/program.lbc
```

Subsequent runs are instant unless the source changes. To force recompilation:

```bash
liphia program.lph --no-cache
```

**REPL / interactive shell**

```bash
liphia          # opens the REPL
liphia --repl   # same
```

---

## Package manager

Liphia includes a built-in package manager for stdlib modules.

**Initialize a project:**

```bash
liphia init
```

Creates `liphia.toml` in the current directory.

**Install a module:**

```bash
liphia install math
liphia install stats fs json
```

Downloads the module from the Liphia registry into `liphia_modules/`.

> **⚠️ Note on built-in modules:** all 9 stdlib modules listed below are already
> compiled into the `liphia` binary you built or downloaded. `liphia install`
> currently still performs a network fetch even for these — that fetch only
> retrieves documentation, not functionality that wasn't already present.
> This is being tightened up (see [Roadmap](#roadmap)) so `install` skips the
> network call for anything already built-in.

**Install all dependencies from `liphia.toml`:**

```bash
liphia install
```

**List available modules:**

```bash
liphia install --list
```

Available modules: `ai`, `math`, `stats`, `fs`, `http`, `ws`, `net`, `json`, `db`.

Once installed, import in your `.lph` file:

```lph
import from "math"
import from "stats"
```

---

## Current syntax — Engine 0.10.0

### Comments

```lph
# This is a comment
print("Hello, world!")
```

---

### Primitive types

| Type    | Description                             |
|---------|------------------------------------------|
| `int`   | 64-bit integer                          |
| `float` | 64-bit floating-point                   |
| `str`   | UTF-8 string                            |
| `bool`  | Boolean: `true` or `false`              |
| `list`  | Dynamic list                            |
| `map`   | Associative key → value collection      |
| `void`  | Return type for functions with no value |
| `null`  | Null literal                            |

---

### Variables

Typed declaration:

```lph
name: type = value
```

Inferred declaration:

```lph
var name = value
```

Constant:

```lph
const PI = 3.14159
```

**Examples:**

```lph
age: int = 20
height: float = 1.80
username: str = "Alice"
active: bool = true
var score = 100
const MAX: int = 999
```

---

### Input and output

```lph
print("Hello, world!")
print("Age:", 20)
print("Name:", username, "Score:", score)
```

```lph
name: str = input("Enter your name: ")
print("Hello,", name)
```

> `input()` always returns `str`. Use `to_int()` or `to_float()` to convert.

```lph
raw: str = input("Enter a number: ")
n: int = to_int(raw)
print("Double:", n * 2)
```

---

### Operators

**Arithmetic:**

| Operator | Operation                            |
|----------|---------------------------------------|
| `+`      | Addition (also string concatenation) |
| `-`      | Subtraction                          |
| `*`      | Multiplication                       |
| `/`      | Division                             |

**Comparison:**

| Operator | Meaning               |
|----------|------------------------|
| `==`     | Equal to              |
| `!=`     | Not equal to          |
| `>`      | Greater than          |
| `<`      | Less than             |
| `>=`     | Greater than or equal |
| `<=`     | Less than or equal    |

**Logical:**

| Operator | Meaning     |
|----------|-------------|
| `and`    | Logical AND |
| `or`     | Logical OR  |
| `not`    | Logical NOT |

---

### Conditionals

```lph
if condition:
    ...
elif other_condition:
    ...
else:
    ...
```

**Example:**

```lph
age: int = 17
if age >= 18:
    print("Adult")
elif age == 17:
    print("Almost there")
else:
    print("Minor")
```

---

### Loops

**While:**

```lph
var i = 0
while i < 5:
    print("i =", i)
    i = i + 1
```

**For (range):**

```lph
for i from 0 to 5:
    print(i)
```

With step:

```lph
for i from 0 to 10 step 2:
    print(i)
```

**Break and continue:**

```lph
for i from 0 to 10:
    if i == 5:
        break
    if i == 3:
        continue
    print(i)
```

---

### Functions

```lph
fn name(param: type, ...) -> return_type:
    ...
    return value
```

**Examples:**

```lph
fn add(a: int, b: int) -> int:
    return a + b

fn greet(name: str) -> void:
    print("Hello,", name)

fn factorial(n: int) -> int:
    if n <= 1:
        return 1
    return n * factorial(n - 1)

print(add(10, 5))
print(factorial(10))
```

---

### Lists

```lph
var values: list = [1, 2, 3, 4, 5]
print(values[0])    # 1
print(values[-1])   # 5
values[0] = 99
append(values, 6)
var last = pop(values)
print("length:", len(values))
```

> **See also:** for key → value data (JSON-like structures, config, records),
> use [`map`](#maps) instead of a flat list — this used to be the only
> option before Engine 0.10.0.

---

### Maps

*(new in Engine 0.10.0)*

```lph
var user: map = {"name": "Alice", "age": 30}
print(user["name"])       # Alice
user["age"] = 31
user["city"] = "Recife"

print(map_keys(user))     # [name, age, city]
print(map_values(user))   # [Alice, 31, Recife]
print(map_has(user, "city"))  # true
map_remove(user, "city")
print(len(user))          # 2
```

Maps can hold any value type, including nested maps and lists:

```lph
var config: map = {
    "debug": true,
    "limits": {"max_users": 100, "timeout": 30}
}
print(config["limits"]["max_users"])  # 100
```

> **Note:** map literals must currently be written on a single line — the
> lexer doesn't yet support multi-line `{}` literals (the same limitation
> already applies to multi-line list literals).

---

### Enums

```lph
enum Direction:
    North
    South
    East
    West

var d = Direction.North
if d == Direction.North:
    print("Going north")
```

---

### Error handling — try/catch

*(new in Engine 0.10.0)*

Any runtime error — including from stdlib calls like `db_open`, `http_get`,
or `json_decode` — can be caught instead of crashing the whole program.
The caught value is always a `str` containing the error message.

```lph
fn risky() -> void:
    try:
        var conn: int = db_open("some/invalid/path.sqlite")
        db_exec(conn, "INSERT INTO x VALUES (1)")
    catch e:
        print("caught:", e)
    print("execution continues normally")

risky()
print("program did not crash")
```

> **Known limitation:** `break`/`continue` used directly inside a `try` block
> that sits inside a loop can leave a stale handler active until the
> enclosing function returns. Avoid combining `try` with `break`/`continue`
> in the same block for now.

---

### String interpolation — f-strings

*(new in Engine 0.10.0)*

```lph
var name: str = "Alice"
var age: int = 30

print(f"Hello {name}, you are {age} years old")
print(f"{{literal braces}} still work, name is {name}")
print(f"math: {1 + 2 * 3}")
```

Any expression is allowed inside `{}` — it's converted with the same rules
as `to_str()`. Use `{{` and `}}` for a literal `{` / `}`.

> **Known limitation:** an interpolated expression that itself contains a
> string literal with `"` (e.g. `f"{some_fn(\"x\")}"`) does not parse
> correctly yet — avoid nested string literals inside `{}` for now.

---

### Async and concurrency

Functions can be declared `async` and awaited inside other async functions.
The VM runs tasks cooperatively in a single-threaded event loop.

```lph
async fn fetch(url: str) -> str:
    var result = await http_get(url)
    return result
```

**Spawn** launches a task concurrently (fire-and-forget):

```lph
async fn worker(id: int) -> void:
    print("worker", id, "running")

spawn worker(1)
spawn worker(2)
```

---

### File imports

```lph
import "utils.lph"
import "./helpers/math_utils.lph"
```

Import cycles are detected and skipped automatically.

**Selective import** *(new in Engine 0.10.0)* — only the listed names are
made available to the importing file:

```lph
import { format_name } from "./utils.lph"
```

**Qualified import** *(new in Engine 0.10.0)* — everything from the file is
imported, but only reachable through the given alias, avoiding name
collisions between files:

```lph
import database from "./database.lph"
import routes from "./routes.lph"

var conn: int = database.connect()
database.init_schema(conn)
print(routes.get_users(conn))
```

If two imported files declare a symbol with the same name without one of
them being qualified, compilation now fails with a clear collision error
instead of silently overwriting one with the other.

---

### Stdlib modules

Install a module first, then import it:

```bash
liphia install math
```

```lph
import from "math"
print(sqrt(16.0))
print(pow(2.0, 10.0))
print(pi())
```

---

## Core native functions

These functions are always available — no import required.

**String and value:**

| Function                   | Returns | Description                        |
|------------------------------|---------|-------------------------------------|
| `len(s)`                   | `int`   | Length of string, list, or map     |
| `to_str(value)`            | `str`   | Convert any value to string        |
| `to_int(value)`            | `int`   | Parse string or float to int       |
| `to_float(value)`          | `float` | Parse string or int to float       |
| `trim(s)`                  | `str`   | Remove leading/trailing whitespace |
| `upper(s)`                 | `str`   | Convert to uppercase               |
| `lower(s)`                 | `str`   | Convert to lowercase               |
| `contains(s, sub)`         | `bool`  | True if `s` contains `sub`         |
| `starts_with(s, prefix)`   | `bool`  | True if `s` starts with `prefix`   |
| `ends_with(s, suffix)`     | `bool`  | True if `s` ends with `suffix`     |
| `replace(s, from, to)`     | `str`   | Replace all occurrences            |
| `split(s, sep)`            | `list`  | Split string by separator          |

**List:**

| Function          | Returns | Description                    |
|-------------------|---------|----------------------------------|
| `len(list)`       | `int`   | Number of elements             |
| `append(list, v)` | `void`  | Add element to end (in-place)  |
| `pop(list)`       | `any`   | Remove and return last element |
| `keys(list)`      | `list`  | Returns list of indices [0..n] |

**Map** *(new in Engine 0.10.0)*:

| Function              | Returns | Description                              |
|-------------------------|---------|--------------------------------------------|
| `map_keys(map)`       | `list`  | All keys, in insertion order             |
| `map_values(map)`     | `list`  | All values, in insertion order           |
| `map_has(map, key)`   | `bool`  | True if `key` exists                     |
| `map_remove(map, key)`| `void`  | Removes the entry for `key`, if present  |

---

## Standard library

Install modules with `liphia install <name>`.
All stdlib functions are native — implemented in Rust and compiled into the CLI binary.

---

### `http` — HTTP server and client

> **Version 1.1.0** — Native CORS support added.

```bash
liphia install http
```

All responses automatically include CORS headers:

```
Access-Control-Allow-Origin: *
Access-Control-Allow-Methods: GET, POST, PUT, PATCH, DELETE, OPTIONS
Access-Control-Allow-Headers: Content-Type, Authorization, X-Requested-With
Access-Control-Max-Age: 86400
```

To handle browser preflight requests, add an `OPTIONS` branch to your router:

```lph
import from "http"
import from "json"

fn route() -> void:
    var method: str = http_method()
    var path:   str = http_path()

    if method == "OPTIONS":
        http_respond_json(200, "{}")
    else:
        if method == "POST" and path == "/login":
            handle_login()
        else:
            http_respond_json(404, "{\"error\": \"not found\"}")

http_listen(8000)
print("Server running on port 8000")

while true:
    var got: bool = http_accept()
    if got:
        route()
```

**Client:**

```lph
var body = http_get("http://example.com")
print("status:", http_status())

var resp = http_post("http://api.example.com/data", "{\"key\": \"value\"}")
```

**Full function list:**

| Function                             | Returns | Description                          |
|-----------------------------------------|---------|-----------------------------------------|
| `http_listen(port)`                  | `bool`  | Bind server to port                  |
| `http_accept()`                      | `bool`  | True if a request is ready           |
| `http_method()`                      | `str`   | Current request method               |
| `http_path()`                        | `str`   | Current request path                 |
| `http_query()`                       | `str`   | Query string after `?`               |
| `http_body()`                        | `str`   | Raw request body                     |
| `http_header(name)`                  | `str`   | Request header value (lowercase key) |
| `http_respond(status, body)`         | `bool`  | Send text/plain response             |
| `http_respond_json(status, body)`    | `bool`  | Send application/json response       |
| `http_get(url)`                      | `str`   | GET request                          |
| `http_post(url, body)`               | `str`   | POST request                         |
| `http_put(url, body)`                | `str`   | PUT request                          |
| `http_patch(url, body)`              | `str`   | PATCH request                        |
| `http_delete(url)`                   | `str`   | DELETE request                       |
| `http_status()`                      | `int`   | Last client response status code     |

---

### `math` — mathematical functions

```bash
liphia install math
```

```lph
import from "math"
print(sqrt(16.0))           # 4.0
print(pow(2.0, 10.0))       # 1024.0
print(log2(1024.0))         # 10.0
print(hypot(3.0, 4.0))      # 5.0
print(deg_to_rad(180.0))    # 3.14159...
print(factorial(10))        # 3628800
print(gcd(48, 18))          # 6
print(clamp(15.0, 0.0, 10.0)) # 10.0
```

**Full function list:**

| Category            | Functions |
|------------------------|-----------|
| Basic               | `sqrt`, `pow`, `abs`, `floor`, `ceil`, `round`, `min`, `max` |
| Constants           | `pi`, `e` |
| Logarithm / exp     | `log`, `log10`, `log2`, `log_base`, `exp` |
| Trigonometry        | `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2` |
| Hyperbolic          | `sinh`, `cosh`, `tanh` |
| Number theory       | `factorial`, `gcd`, `lcm` |
| Geometry            | `hypot`, `deg_to_rad`, `rad_to_deg` |
| Utilities           | `sign`, `clamp`, `is_nan`, `is_inf` |

---

### `stats` — statistical functions

```bash
liphia install stats
```

```lph
import from "stats"
var data: list = [4.0, 7.0, 13.0, 2.0, 1.0]
print(mean(data))             # 5.4
print(stdev_sample(data))     # 4.827...
print(percentile(data, 75.0)) # Q3
print(pearson_r(data, data))  # 1.0
print(p_value_t_ind(data, data)) # 1.0
```

**Full function list:**

| Category              | Functions |
|--------------------------|-----------|
| Descriptive           | `sum`, `mean`, `median`, `min_list`, `max_list`, `count`, `mode`, `range_stat` |
| Variance / spread     | `variance`, `stdev`, `variance_sample`, `stdev_sample`, `percentile`, `iqr` |
| Normalisation         | `zscore`, `covariance` |
| Correlation           | `pearson_r`, `spearman_r`, `kendall_tau` |
| Test statistics       | `t_stat_independent`, `t_degrees_of_freedom`, `t_stat_paired`, `mann_whitney_u`, `wilcoxon_w` |
| Normality             | `shapiro_wilk_w` |
| p-values              | `p_value_t_ind`, `p_value_t_paired`, `p_value_normal`, `p_value_mann_whitney` |

---

### `ai` — machine learning primitives

```bash
liphia install ai
```

```lph
import from "ai"
print(sigmoid(0.0))           # 0.5
print(relu(-3.0))             # 0.0
print(dot([1.0, 2.0], [3.0, 4.0]))  # 11.0
seed(42)
var w = rand_normal(4, 0.0, 0.1)
w = sgd_update(w, rand_normal(4, 0.0, 0.01), 0.001)
```

Full function list: `sigmoid`, `relu`, `leaky_relu`, `tanh_act`, `elu`, `gelu`, `swish`,
`dot`, `norm`, `vec_add`, `vec_sub`, `vec_mul`, `vec_scale`, `vec_sum`,
`softmax`, `argmax`,
`matrix_new`, `matrix_mul`, `matrix_add`, `transpose`,
`normalize`, `standardize`, `clip`, `linspace`, `arange`,
`mse`, `mae`, `cross_entropy`, `binary_cross_entropy`,
`seed`, `rand_uniform`, `rand_normal`, `rand_int`, `shuffle`,
`gradient_clip`, `sgd_update`, `adam_update`,
`accuracy`, `precision`, `recall`, `f1_score`,
`cosine_similarity`, `euclidean_dist`, `manhattan_dist`.

---

### `fs` — file system

```bash
liphia install fs
```

```lph
import from "fs"
write_file("hello.txt", "Hello from Liphia!")
var content = read_file("hello.txt")
print(content)
print(file_exists("hello.txt"))  # true
```

---

### `json` — JSON encoding and decoding

> **Version 2.0.0 — ⚠️ breaking change.** `json_decode` used to return a flat
> list `["key", value, "key", value, ...]` for JSON objects. As of Engine
> 0.10.0, it returns a real `map` instead. Code written against the old
> flat-list format needs to be updated.

```bash
liphia install json
```

```lph
import from "json"

var original: map = {"name": "Alice", "age": 25, "tags": [1, 2, 3]}
var text: str = json_encode(original)
print(text)   # {"name":"Alice","age":25,"tags":[1,2,3]}

var decoded: map = json_decode(text)
print(decoded["name"])   # Alice
print(decoded["tags"][1]) # 2
```

`json_get` / `json_has` are unchanged — they still work directly on the raw
JSON text and don't require decoding first:

```lph
var raw = "{\"name\": \"Alice\", \"age\": 30}"
print(json_get(raw, "name"))   # Alice
print(json_has(raw, "email"))  # false
```

---

### `db` — SQLite and PostgreSQL

```bash
liphia install db
```

```lph
import from "db"

var conn: int = db_open("data.sqlite")
db_exec(conn, "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)")
db_exec(conn, "INSERT INTO users (name) VALUES ('Alice')")
var rows = db_query_rows(conn, "SELECT * FROM users")
print("rows:", len(rows))
db_close(conn)
```

> **⚠️ Also usable now:** with `try`/`catch` (see [above](#error-handling--trycatch)),
> DB errors no longer have to crash the whole program — wrap risky queries in
> a `try` block to handle failures gracefully.

---

### `ws` — WebSockets

```bash
liphia install ws
```

```lph
import from "ws"

ws_listen(8081)

async fn accept_loop() -> void:
    var client: int = await ws_accept()
    ws_send(client, "Welcome!")
    ws_broadcast("New client connected")
```

---

### `net` — TCP/UDP sockets

```bash
liphia install net
```

```lph
import from "net"

var conn: int = tcp_connect("127.0.0.1", 9000)
tcp_send(conn, "ping")
var reply = tcp_recv(conn)
print(reply)
tcp_close(conn)
```

---

## Full example

```lph
import from "math"
import from "stats"
import from "ai"

print("=== Liphia Demo ===")

fn add(a: int, b: int) -> int:
    return a + b

fn user_factorial(n: int) -> int:
    if n <= 1:
        return 1
    return n * user_factorial(n - 1)

var x: int = add(10, 5)
print("10 + 5 =", x)
print("10! =", user_factorial(10))

print("sqrt(144) =", sqrt(144.0))
print("pi =", pi())
print("log2(1024) =", log2(1024.0))
print("hypot(3,4) =", hypot(3.0, 4.0))

var scores: list = [72.0, 85.0, 90.0, 68.0, 77.0, 95.0, 82.0]
print("mean:   ", mean(scores))
print("stdev:  ", stdev_sample(scores))
print("IQR:    ", iqr(scores))

var v: list = [1.0, 2.0, 3.0]
print("norm([1,2,3]) =", norm(v))
print("sigmoid(1)    =", sigmoid(1.0))

var profile: map = {"name": "Alice", "score": x}
print(f"Done — {profile["name"]} scored {profile["score"]}")
```

---

## Roadmap

### ✅ Engine 0.10.0 — current

- [x] Indentation blocks (Indent / Dedent)
- [x] Typed variables (`name: type = value`), `var`, `const`
- [x] Conditionals (`if`, `elif`, `else`)
- [x] Loops (`while`, `for from/to/step`, `break`, `continue`)
- [x] Functions (`fn`, `return`, recursion)
- [x] Async functions (`async fn`, `await`)
- [x] Concurrency via `spawn` (cooperative event-loop VM)
- [x] Bytecode compiler + VM execution
- [x] REPL / interactive shell (`--repl`)
- [x] File import system (`import "file.lph"`)
- [x] Stdlib import system (`import from "module"`)
- [x] Package manager (`liphia init`, `liphia install`, `liphia install --list`)
- [x] Lists and indexing (`x[0]`, `x[-1]`, `append`, `pop`, `keys`)
- [x] **Maps / dictionaries** (`map`, `{key: value}` literals, `map_keys`, `map_values`, `map_has`, `map_remove`)
- [x] Enums and variant matching
- [x] **Recoverable error handling** (`try` / `catch`), no longer crashes the whole process on runtime errors
- [x] **Real module system** — qualified imports (`import alias from "..."`), selective imports (`import { fn } from "..."`), and compile-time name-collision detection across imported files
- [x] **String interpolation** (f-strings: `f"{expr}"`, with `{{`/`}}` escaping)
- [x] Structured error messages with **accurate line/column reporting** (lexer position-tracking fix)
- [x] Bytecode cache (`.lbc`)
- [x] Core native functions — string, list, map, type conversion (always available)
- [x] Standard library: `ai`, `math`, `stats`, `fs`, `http`, `ws`, `net`, `json`, `db`
- [x] `http` module v1.1.0: native CORS headers on all responses
- [x] `json` module v2.0.0: `json_decode` returns real `map`/`list` values instead of a flat list (**breaking change**)
- [x] `db` module: SQLite (bundled) + PostgreSQL (pure TCP wire protocol)
- [x] `ai` module: activations, vectors, matrices, preprocessing, loss, random, optimization, metrics, distances
- [x] `math` module v1.0.2: trig, inverse trig, hyperbolic, exp/log, number theory, geometry, utilities (35 functions)
- [x] `stats` module v1.0.3: descriptive, sample statistics, percentiles, correlation (Pearson/Spearman/Kendall), hypothesis tests (t-test, Mann-Whitney, Wilcoxon, Shapiro-Wilk), p-values via pure-Rust CDF (26 functions)
- [x] VS Code extension v0.0.2 (syntax highlighting + snippets)

### 🎯 Engine 1.0 — Stable release *(planned)*

- [ ] GitHub Actions — automated binary releases for Windows, Linux, macOS
- [ ] Cargo feature flags per stdlib module (leaner custom builds)
- [ ] `install` skips network fetch for modules already built into the current binary
- [ ] Consolidated, versioned stdlib API documentation
- [ ] Better diagnostics with source context and inline suggestions
- [ ] Null safety / optional types (`T?`) — polish and enforcement
- [ ] Real `.lph`-native multi-file packages (beyond stdlib), with a `liphia.toml` `[dependencies]` manifest and a `liphia.lock` for pinned versions

---

## License

Licensed under either of:

- [MIT License](./licenses/LICENSE-MIT)
- [Apache License, Version 2.0](./licenses/LICENSE-APACHE)

at your option.

---

<p align="center">
  Built with ❤️ &nbsp;·&nbsp; Engine written in Rust &nbsp;·&nbsp;
  <a href="https://github.com/shferreira-lab">shferreira-lab</a>
</p>