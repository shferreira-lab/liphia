# Liphia (.lph)

> A statically typed, indentation-based programming language powered by a Rust bytecode VM.
> Created by Sergio H. Ferreira — started in late 2025.

[![Engine](https://img.shields.io/badge/engine-0.9.0-blueviolet)](https://github.com/shferreira-lab/liphia)
[![Compiler](https://img.shields.io/badge/compiler-0.9.1-blueviolet)](https://github.com/shferreira-lab/liphia)
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
│   │       ├── liphia_cli/                 # CLI runner + REPL shell
│   │       ├── liphia_compiler/            # lexer, parser, AST, bytecode compiler
│   │       ├── liphia_core_native/         # core string, list and value utilities
│   │       └── liphia_virtual_machine/     # bytecode VM
│   │
│   ├── stdlib/
│   │   ├── modules/               # .lph modules (import from "...")
│   │   └── native/                # Rust native stdlib bindings
│   │       └── src/
│   │           ├── cdf.rs         # internal CDF / special functions (not exported)
│   │           ├── math.rs
│   │           ├── stats.rs
│   │           └── ...
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
liphia install math
liphia install stats fs json
```

Downloads the module from the Liphia registry into `liphia_modules/`.

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

## Current syntax — Engine 0.9.0

### Comments

```lph
# This is a comment
print("Hello, world!")
```

---

### Primitive types

| Type    | Description                             |
|---------|-----------------------------------------|
| `int`   | 64-bit integer                          |
| `float` | 64-bit floating-point                   |
| `str`   | UTF-8 string                            |
| `bool`  | Boolean: `true` or `false`              |
| `list`  | Dynamic list                            |
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
|----------|--------------------------------------|
| `+`      | Addition (also string concatenation) |
| `-`      | Subtraction                          |
| `*`      | Multiplication                       |
| `/`      | Division                             |

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
|----------------------------|---------|------------------------------------|
| `len(s)`                   | `int`   | Length of string or list           |
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
|-------------------|---------|--------------------------------|
| `len(list)`       | `int`   | Number of elements             |
| `append(list, v)` | `void`  | Add element to end (in-place)  |
| `pop(list)`       | `any`   | Remove and return last element |
| `keys(list)`      | `list`  | Returns list of indices [0..n] |

---

## Standard library

Install modules with `liphia install <name>`.
All stdlib functions are native — implemented in Rust and compiled into the CLI binary.

---

### `math` — mathematical functions

```bash
liphia install math
```

```lph
import from "math"

# basics
print(sqrt(16.0))           # 4.0
print(pow(2.0, 10.0))       # 1024.0
print(abs(-7))              # 7
print(floor(3.9))           # 3
print(ceil(3.1))            # 4
print(round(3.5))           # 4
print(min(3, 7))            # 3
print(max(3.5, 2.1))        # 3.5
print(pi())                 # 3.141592653589793
print(e())                  # 2.718281828459045

# logarithm / exponential
print(log(e()))             # 1.0
print(log10(100.0))         # 2.0
print(log2(1024.0))         # 10.0
print(log_base(8.0, 2.0))   # 3.0
print(exp(1.0))             # 2.718281828459045

# trigonometry
print(sin(0.0))             # 0.0
print(cos(0.0))             # 1.0
print(tan(0.0))             # 0.0
print(asin(1.0))            # 1.5707963... (π/2)
print(acos(1.0))            # 0.0
print(atan(1.0))            # 0.7853981... (π/4)
print(atan2(1.0, 1.0))      # 0.7853981... (π/4)

# hyperbolic
print(sinh(1.0))            # 1.1752011...
print(cosh(1.0))            # 1.5430806...
print(tanh(1.0))            # 0.7615941...

# number theory
print(factorial(10))        # 3628800
print(gcd(48, 18))          # 6
print(lcm(4, 6))            # 12

# geometry / angle conversion
print(hypot(3.0, 4.0))      # 5.0
print(deg_to_rad(180.0))    # 3.141592653589793
print(rad_to_deg(pi()))     # 180.0

# utilities
print(sign(-7.5))           # -1
print(clamp(15.0, 0.0, 10.0)) # 10.0
print(is_nan(0.0))          # false
print(is_inf(0.0))          # false
```

**Full function list:**

| Category            | Functions |
|---------------------|-----------|
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

# descriptives
print(sum(data))              # 27.0
print(mean(data))             # 5.4
print(median(data))           # 4.0
print(stdev(data))            # 4.029...  (population)
print(stdev_sample(data))     # 4.827...  (sample, n-1)
print(variance(data))         # 16.24
print(variance_sample(data))  # 23.3
print(min_list(data))         # 1.0
print(max_list(data))         # 13.0
print(count(data))            # 5
print(mode(data))             # most frequent value
print(range_stat(data))       # 12.0

# percentiles and spread
var d: list = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]
print(percentile(d, 25.0))    # 3.25  (Q1)
print(percentile(d, 75.0))    # 7.75  (Q3)
print(iqr(d))                 # 4.5

# normalisation
var z: list = zscore(data)    # list of z-scores
print(covariance(d, d))       # same as variance_sample(d)

# correlation
var x: list = [1.0, 2.0, 3.0, 4.0, 5.0]
var y: list = [2.1, 3.9, 6.2, 7.8, 10.1]
print(pearson_r(x, y))        # ≈ 0.999  (strong linear)
print(spearman_r(x, y))       # ≈ 1.0   (perfectly monotone)
print(kendall_tau(x, y))      # ≈ 1.0

# normality check (run before choosing test)
var w: float = shapiro_wilk_w(data)
print(w)                      # W close to 1 → consistent with normal

# hypothesis tests — two independent groups
var ctrl:  list = [5.1, 4.9, 5.0, 5.2, 4.8, 5.1, 4.9, 5.0, 5.1, 4.8]
var treat: list = [6.2, 6.0, 6.3, 6.1, 5.9, 6.4, 6.2, 6.1, 6.3, 6.0]

var p: float = p_value_t_ind(ctrl, treat)
print("p-value (t-test): ", p)          # ≈ 0.000 → significant

var u: float = mann_whitney_u(ctrl, treat)
print("Mann-Whitney U: ", u)            # 0.0 → complete separation

# hypothesis tests — paired
var before: list = [200.0, 212.0, 195.0, 208.0, 220.0]
var after:  list = [185.0, 198.0, 180.0, 192.0, 205.0]

var p2: float = p_value_t_paired(before, after)
print("p-value (paired t): ", p2)       # ≈ 0.000

var w2: float = wilcoxon_w(before, after)
print("Wilcoxon W: ", w2)               # 15.0 (all diffs positive)

# p-value from a z-score
print(p_value_normal(1.96))             # ≈ 0.05
```

**Full function list:**

| Category              | Functions |
|-----------------------|-----------|
| Descriptive           | `sum`, `mean`, `median`, `min_list`, `max_list`, `count`, `mode`, `range_stat` |
| Variance / spread     | `variance`, `stdev`, `variance_sample`, `stdev_sample`, `percentile`, `iqr` |
| Normalisation         | `zscore`, `covariance` |
| Correlation           | `pearson_r`, `spearman_r`, `kendall_tau` |
| Test statistics       | `t_stat_independent`, `t_degrees_of_freedom`, `t_stat_paired`, `mann_whitney_u`, `wilcoxon_w` |
| Normality             | `shapiro_wilk_w` |
| p-values              | `p_value_t_ind`, `p_value_t_paired`, `p_value_normal`, `p_value_mann_whitney` |

> **Note — p-values:** `p_value_t_ind` and `p_value_t_paired` use the t-distribution CDF
> implemented in pure Rust (Lanczos log-gamma + Lentz continued fractions for the
> incomplete beta, no external dependencies). `p_value_mann_whitney` uses the normal
> approximation and requires `min(n1, n2) > 10`.

---

### `ai` — machine learning primitives

```bash
liphia install ai
```

```lph
import from "ai"

# activations
print(sigmoid(0.0))           # 0.5
print(relu(-3.0))             # 0.0
print(leaky_relu(-3.0, 0.01)) # -0.03
print(gelu(1.0))              # 0.841...

# vectors
var a: list = [1.0, 2.0, 3.0]
var b: list = [4.0, 5.0, 6.0]
print(dot(a, b))              # 32.0
print(norm(a))                # 3.741...
print(vec_add(a, b))          # [5, 7, 9]

# preprocessing
print(normalize([2.0, 4.0, 6.0, 8.0]))
print(standardize([2.0, 4.0, 6.0, 8.0]))

# loss functions
var pred:   list = [0.9, 0.1, 0.8]
var target: list = [1.0, 0.0, 1.0]
print(mse(pred, target))
print(binary_cross_entropy(pred, target))

# metrics
print(accuracy(pred, target))
print(f1_score(pred, target))

# distances
print(cosine_similarity([1.0, 0.0], [0.0, 1.0]))  # 0.0
print(euclidean_dist([1.0, 0.0], [0.0, 1.0]))     # 1.414...

# random
seed(42)
var weights = rand_normal(8, 0.0, 0.1)

# optimization
var w: list = [0.5, -0.3, 0.8]
var g: list = [0.1, -0.2, 0.05]
w = sgd_update(w, g, 0.01)
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

```bash
liphia install json
```

```lph
import from "json"

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

# SQLite — embedded, no installation required
var conn: int = db_open("data.sqlite")
db_exec(conn, "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)")
db_exec(conn, "INSERT INTO users (name) VALUES ('Alice')")
var rows = db_query_rows(conn, "SELECT * FROM users")
print("rows:", len(rows))
db_close(conn)

# PostgreSQL — pure TCP wire protocol
var pg = pg_connect("127.0.0.1", 5432, "postgres", "secret", "mydb")
pg_exec(pg, "INSERT INTO products (name) VALUES ('Widget')")
var products = pg_query_rows(pg, "SELECT * FROM products")
pg_close(pg)
```

---

### `http` — HTTP server and client

```bash
liphia install http
```

```lph
import from "http"

# client
var body = http_get("http://example.com")
print("status:", http_status())

# server
http_listen(3000)
async fn server_loop() -> void:
    while true:
        var got: bool = await http_accept()
        if got:
            http_respond_json(200, "{\"status\": \"ok\"}")

spawn server_loop()
```

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

# math
print("sqrt(144) =", sqrt(144.0))
print("pi =", pi())
print("log2(1024) =", log2(1024.0))
print("hypot(3,4) =", hypot(3.0, 4.0))
print("deg_to_rad(90) =", deg_to_rad(90.0))

# stats
var scores: list = [72.0, 85.0, 90.0, 68.0, 77.0, 95.0, 82.0]
print("mean:   ", mean(scores))
print("median: ", median(scores))
print("stdev:  ", stdev_sample(scores))
print("IQR:    ", iqr(scores))
print("Shapiro-Wilk W: ", shapiro_wilk_w(scores))

var group_a: list = [5.1, 4.9, 5.0, 5.2, 4.8, 5.1, 4.9, 5.0, 5.1, 4.8,
                     5.0, 5.1, 4.9, 5.2, 5.0]
var group_b: list = [6.2, 6.0, 6.3, 6.1, 5.9, 6.4, 6.2, 6.1, 6.3, 6.0,
                     6.2, 6.3, 6.1, 6.0, 6.2]
print("p-value (t-test): ", p_value_t_ind(group_a, group_b))

# ai
var v: list = [1.0, 2.0, 3.0]
print("norm([1,2,3]) =", norm(v))
print("sigmoid(1)    =", sigmoid(1.0))

seed(42)
var w = rand_normal(4, 0.0, 0.1)
w = sgd_update(w, rand_normal(4, 0.0, 0.01), 0.001)
print("trained weights:", w)

print("Done.")
```

---

## Roadmap

### ✅ Engine 0.9.0 — current

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
- [x] Enums and variant matching
- [x] Structured error messages with line/column and error codes
- [x] Bytecode cache (`.lbc`)
- [x] Core native functions — string, list, type conversion (always available)
- [x] Standard library: `ai`, `math`, `stats`, `fs`, `http`, `ws`, `net`, `json`, `db`
- [x] `db` module: SQLite (bundled) + PostgreSQL (pure TCP wire protocol)
- [x] `ai` module: activations, vectors, matrices, preprocessing, loss, random, optimization, metrics, distances
- [x] `math` module: trig, inverse trig, hyperbolic, exp/log, number theory, geometry, utilities (35 functions)
- [x] `stats` module: descriptive, sample statistics, percentiles, correlation (Pearson/Spearman/Kendall), hypothesis tests (t-test, Mann-Whitney, Wilcoxon, Shapiro-Wilk), p-values via pure-Rust CDF (26 functions)
- [x] VS Code extension v0.0.2 (syntax highlighting + snippets)

### 🎯 Engine 1.0 — Stable release *(planned)*

- [ ] Maps / dictionaries
- [ ] Real module system with selective exports (`import { fn } from "module"`)
- [ ] Better diagnostics with source context and inline suggestions
- [ ] Null safety / optional types (`T?`)
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