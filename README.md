# Liphia (.lph)

> A general-purpose programming language with Python-style indentation syntax,
> strong static typing, and a bytecode virtual machine written in Rust.

[![Engine](https://img.shields.io/badge/engine-0.9.0-blueviolet)](https://github.com/shferreira-lab/liphia)
[![Language](https://img.shields.io/badge/rust-core%20engine-orange)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/status-active%20development-yellow)](https://github.com/shferreira-lab)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](./licenses)

Liphia is a general-purpose language with indentation-delimited syntax
(similar to Python), explicit static typing, and a bytecode compiler
and virtual machine both written in Rust.

Liphia has its own compiler, bytecode format, and VM — no external runtime,
no interpreter dependency, no GC overhead.
The compiled binary is fully self-contained.

> **Notice:** The project is under active development. Full documentation,
> stable releases, and the standard library will be published when the
> engine reaches sufficient stability.

---

## Table of Contents

- [Repository layout](#repository-layout)
- [How to build](#how-to-build)
- [How to run](#how-to-run)
- [Package manager](#package-manager)
- [Current syntax — Engine 0.9.0](#current-syntax--engine-090)
  - [Comments](#comments)
  - [Primitive types](#primitive-types)
  - [Variables](#variables)
  - [Input and output](#input-and-output)
  - [Operators](#operators)
  - [Conditionals](#conditionals)
  - [Loops](#loops)
  - [Functions](#functions)
  - [Lists](#lists)
  - [Enums](#enums)
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
│   │       ├── liphia_cli/        # CLI runner + REPL shell
│   │       ├── liphia_compiler/   # lexer, parser, AST, bytecode compiler
│   │       ├── liphia_core_native/# core string and value utilities
│   │       └── liphia_virtual_machine/ # bytecode VM
│   │
│   ├── stdlib/
│   │   ├── modules/               # .lph modules (import from "...")
│   │   └── native/                # Rust native stdlib bindings
│   │
│   ├── tests/                     # tests and benchmarks
│   └── tools/
│       └── liphia-vscode/         # VS Code extension
│
├── CHANGELOG.md
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
liphia install ai
liphia install math fs json
```

Downloads the module from the Liphia registry and saves it to
`liphia_modules/` in the current directory — similar to `npm install`.

**Install all dependencies from `liphia.toml`:**

```bash
liphia install
```

**List available modules:**

```bash
liphia install --list
```

Available modules: `ai`, `math`, `stats`, `fs`, `http`, `ws`, `net`, `json`.

Once installed, import in your `.lph` file:

```lph
import from "ai"
import from "math"
```

---

## Current syntax — Engine 0.9.0

### Comments

```lph
# This is a comment
print("Hello, world!")
```

---

### Primitive types

| Type      | Description                              |
|-----------|------------------------------------------|
| `int`     | 64-bit integer                           |
| `float`   | 64-bit floating-point                    |
| `str`     | UTF-8 string                             |
| `bool`    | Boolean: `true` or `false`               |
| `list`    | Dynamic list                             |
| `void`    | Return type for functions with no value  |
| `null`    | Null literal                             |
| `T?`      | Optional type (e.g. `int?`, `str?`)      |

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
nothing: null = null

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

| Operator | Operation      |
|----------|----------------|
| `+`      | Addition (also string concatenation) |
| `-`      | Subtraction    |
| `*`      | Multiplication |
| `/`      | Division       |

**Comparison:**

| Operator | Meaning               |
|----------|-----------------------|
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

fn is_even(n: int) -> bool:
    return n / 2 * 2 == n

result: int = add(10, 5)
print("10 + 5 =", result)
greet("Alice")
```

---

### Lists

```lph
values: list = [1, 2, 3, 4, 5]
words: list = ["apple", "banana", "cherry"]

print(values[0])    # 1
print(values[-1])   # 5

# modify
values[0] = 99
print(values[0])    # 99

# append and pop
append(values, 6)
var last = pop(values)
print("length:", len(values))
```

---

### Enums

```lph
enum Direction:
    North
    South
    East
    West

var d: Direction = Direction.North

if d == Direction.North:
    print("Going north")
```

---

### Async and concurrency

Functions can be declared `async` and awaited inside other async functions.
The VM runs tasks cooperatively in a single-threaded event loop.

```lph
async fn fetch_data(url: str) -> str:
    var result: str = await http_get(url)
    return result

async fn main_loop() -> void:
    var data: str = await fetch_data("http://example.com")
    print("received:", len(data), "bytes")
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

Relative import (resolved from the current file's directory):

```lph
import "utils.lph"
import "./helpers/math_utils.lph"
```

Absolute import:

```lph
# Windows
import "C:/Dev/myproject/utils.lph"

# Linux / macOS
import "/home/user/myproject/utils.lph"
```

Import cycles are detected and skipped automatically.

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

Multiple modules:

```lph
import from "math"
import from "ai"
import from "json"
```

---

## Core native functions

These functions are always available — no import required.

**String:**

| Function                        | Returns | Description                        |
|---------------------------------|---------|------------------------------------|
| `len(s)`                        | `int`   | Length of string or list           |
| `to_str(value)`                 | `str`   | Convert any value to string        |
| `to_int(value)`                 | `int`   | Parse string or float to int       |
| `to_float(value)`               | `float` | Parse string or int to float       |
| `trim(s)`                       | `str`   | Remove leading/trailing whitespace |
| `upper(s)`                      | `str`   | Convert to uppercase               |
| `lower(s)`                      | `str`   | Convert to lowercase               |
| `contains(s, sub)`              | `bool`  | True if `s` contains `sub`         |
| `starts_with(s, prefix)`        | `bool`  | True if `s` starts with `prefix`   |
| `ends_with(s, suffix)`          | `bool`  | True if `s` ends with `suffix`     |
| `replace(s, from, to)`          | `str`   | Replace all occurrences            |
| `split(s, sep)`                 | `list`  | Split string by separator          |

**List:**

| Function          | Returns | Description              |
|-------------------|---------|--------------------------|
| `len(list)`       | `int`   | Number of elements       |
| `append(list, v)` | `void`  | Add element to end       |
| `pop(list)`       | `any`   | Remove and return last   |

**Example:**

```lph
var s: str = "  Hello, World!  "
print(trim(s))                        # Hello, World!
print(upper(s))                       # HELLO, WORLD!
print(contains(s, "World"))           # true
print(replace(s, "World", "Liphia"))  # Hello, Liphia!

var parts: list = split("a,b,c", ",")
print(parts[0])   # a
print(len(parts)) # 3
```

---

## Standard library

Install modules with `liphia install <name>`. All functions from stdlib
modules are native — implemented in Rust and compiled into the CLI binary.

### `ai` — machine learning primitives

```bash
liphia install ai
```

```lph
import from "ai"

# activation functions
print("sigmoid(0) =", sigmoid(0.0))     # 0.5
print("relu(-3) =", relu(-3.0))         # 0
print("relu(5) =", relu(5.0))           # 5

# vector operations
var v1: list = [1.0, 2.0, 3.0]
var v2: list = [4.0, 5.0, 6.0]
print("dot =", dot(v1, v2))             # 32
print("norm =", norm(v1))               # 3.741...

# softmax + argmax
var probs: list = softmax([1.0, 2.0, 3.0])
print("argmax =", argmax(probs))        # 2
```

| Function                                   | Returns | Description                           |
|--------------------------------------------|---------|---------------------------------------|
| `sigmoid(x)`                               | `float` | 1 / (1 + e^(-x))                      |
| `relu(x)`                                  | `float` | max(0, x)                             |
| `softmax(v)`                               | `list`  | Probability distribution from list    |
| `argmax(v)`                                | `int`   | Index of maximum value                |
| `dot(a, b)`                                | `float` | Vector dot product                    |
| `norm(v)`                                  | `float` | L2 (Euclidean) norm                   |
| `matrix_new(rows, cols, fill)`             | `list`  | Flat matrix filled with value         |
| `matrix_mul(a, b, rows, inner, cols)`      | `list`  | Matrix multiplication                 |
| `matrix_add(a, b)`                         | `list`  | Element-wise addition                 |

### `math` — mathematical functions

```bash
liphia install math
```

```lph
import from "math"

print(sqrt(16.0))          # 4
print(pow(2.0, 8.0))       # 256
print(abs(-7.5))            # 7.5
print(floor(3.9))           # 3
print(ceil(3.1))            # 4
print(round(3.5))           # 4
print(pi())                 # 3.14159...
print(sin(0.0))             # 0
print(log(1.0))             # 0
```

### `stats` — statistical functions

```bash
liphia install stats
```

```lph
import from "stats"

var data: list = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]
print("mean =", mean(data))
print("stdev =", stdev(data))
print("median =", median(data))
print("sum =", sum(data))
```

### `fs` — file system

```bash
liphia install fs
```

```lph
import from "fs"

write_file("hello.txt", "Hello from Liphia!")
var content: str = read_file("hello.txt")
print(content)
print("exists:", file_exists("hello.txt"))
```

### `json` — JSON encoding and decoding

```bash
liphia install json
```

```lph
import from "json"

var raw: str = "{\"name\": \"Alice\", \"age\": 30}"
print(json_get(raw, "name"))    # Alice
print(json_has(raw, "email"))   # false
```

### `http` — HTTP server and client

```bash
liphia install http
```

```lph
import from "http"

# client
var body: str = http_get("http://example.com")
print("status:", http_status())

# server (async)
http_listen(8080)
async fn handle() -> void:
    var req = await http_accept()
    http_respond(200, "Hello from Liphia!")

spawn handle()
```

### `net` — TCP/UDP sockets

```bash
liphia install net
```

```lph
import from "net"

var conn: int = tcp_connect("127.0.0.1", 9000)
tcp_send(conn, "ping")
var reply: str = tcp_recv(conn)
print(reply)
tcp_close(conn)
```

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

## Full example

```lph
import from "math"
import from "ai"

print("=== Liphia Demo ===")

# functions
fn add(a: int, b: int) -> int:
    return a + b

fn factorial(n: int) -> int:
    if n <= 1:
        return 1
    return n * factorial(n - 1)

# arithmetic
var x: int = add(10, 5)
print("10 + 5 =", x)
print("10! =", factorial(10))

# lists
var values: list = [1, 2, 3, 4, 5]
print("first =", values[0])
print("last =", values[-1])
append(values, 6)
print("length =", len(values))

# math stdlib
print("sqrt(144) =", sqrt(144.0))
print("pi =", pi())

# ai stdlib
var v: list = [1.0, 2.0, 3.0]
print("norm([1,2,3]) =", norm(v))
print("sigmoid(1) =", sigmoid(1.0))

# string operations
var msg: str = "  hello, liphia!  "
print(trim(upper(msg)))

print("Done.")
```

---

## Roadmap

### ✅ Engine 0.9.0 — current

- [x] Indentation blocks (Indent / Dedent)
- [x] Typed variables, `var`, `const`
- [x] Conditionals (`if`, `elif`, `else`)
- [x] Loops (`while`, `for from/to/step`, `break`, `continue`)
- [x] Functions (`fn`, `return`, recursion)
- [x] Async functions and `await`
- [x] Concurrency via `spawn` (cooperative event-loop VM)
- [x] Bytecode compiler + VM execution
- [x] REPL / interactive shell (`--repl`)
- [x] File import system (`import "file.lph"`)
- [x] Stdlib import system (`import from "module"`)
- [x] Package manager (`liphia init`, `liphia install`, `liphia install --list`)
- [x] Lists and indexing (`x[0]`, `x[-1]`, `append`, `pop`)
- [x] Enums and variant matching
- [x] Optional types (`T?`)
- [x] Structured error messages with line/column (lexer + parser + VM)
- [x] Bytecode cache (`.lbc`)
- [x] Core native functions (string, list, type conversion) — always available
- [x] Standard library: `ai`, `math`, `stats`, `fs`, `http`, `ws`, `net`, `json`
- [x] VS Code extension (syntax highlighting)

### 🎯 Engine 1.0 — Stable release *(planned)*

- [ ] Maps / dictionaries
- [ ] Real module system with selective exports (`import { fn } from "module"`)
- [ ] Better diagnostics with source context and suggestions
- [ ] Stable stdlib API with full documentation
- [ ] GitHub Actions — automated binary releases for Windows, Linux, macOS

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