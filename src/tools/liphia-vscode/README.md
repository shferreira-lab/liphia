# Liphia Language Support for VS Code

> Syntax highlighting and snippets for the Liphia programming language (`.lph`).

[![Extension](https://img.shields.io/badge/extension-0.0.2-blueviolet)](https://github.com/shferreira-lab/liphia)
[![Engine](https://img.shields.io/badge/engine-0.9.0-blueviolet)](https://github.com/shferreira-lab/liphia)
[![Language](https://img.shields.io/badge/rust-core%20engine-orange)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/status-active%20development-yellow)](https://github.com/shferreira-lab)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](./licenses)

---

## Features

- Syntax highlighting for all Liphia keywords, types, operators, and literals
- Highlighting for all stdlib modules: `ai`, `math`, `stats`, `fs`, `http`, `ws`, `net`, `json`, `db`
- Code snippets for common patterns
- Comment toggling with `#`

---

## What's new in 0.0.2

- Added keywords: `var`, `const`, `enum`, `async`, `await`, `spawn`
- Added types: `list`, `null`
- Added all stdlib functions as highlighted builtins (grouped by module)
- Added escape sequence highlighting inside strings
- Operators split into categories: comparison, arithmetic, assignment, logical
- New snippets: async fn, HTTP server boilerplate, SQLite, neural network forward pass, Adam/SGD optimizers, classification metrics

---

## Snippets

| Prefix        | Description                              |
|---------------|------------------------------------------|
| `print`       | `print(value)`                           |
| `var`         | Variable (type inferred)                 |
| `vart`        | Variable (typed)                         |
| `const`       | Constant declaration                     |
| `int`         | Integer variable                         |
| `float`       | Float variable                           |
| `str`         | String variable                          |
| `bool`        | Boolean variable                         |
| `list`        | List variable                            |
| `if`          | If statement                             |
| `ifelse`      | If/else                                  |
| `ifelifelse`  | If/elif/else                             |
| `while`       | While loop                               |
| `for`         | For loop                                 |
| `forstep`     | For loop with step                       |
| `fn`          | Function declaration                     |
| `fnr`         | Function with return value               |
| `async`       | Async function                           |
| `asyncawait`  | Async function with await                |
| `spawn`       | Spawn async task                         |
| `enum`        | Enum declaration                         |
| `import`      | Import local file                        |
| `importfrom`  | Import stdlib module                     |
| `importai`    | `import from "ai"`                       |
| `importmath`  | `import from "math"`                     |
| `importhttp`  | `import from "http"`                     |
| `importdb`    | `import from "db"`                       |
| `importjson`  | `import from "json"`                     |
| `httpserver`  | Full HTTP server boilerplate             |
| `sqlite`      | SQLite open, create, query               |
| `nnforward`   | Neural network forward pass              |
| `sgd`         | SGD weight update                        |
| `adam`        | Adam optimizer update                    |
| `metrics`     | Print accuracy, precision, recall, f1    |
| `div`         | Section divider comment                  |

---

## Syntax — Engine 0.9.0

### Comments

```lph
# This is a comment
print("Hello, world!")
```

### Types

| Type    | Description                             |
|---------|-----------------------------------------|
| `int`   | 64-bit integer                          |
| `float` | 64-bit floating-point                   |
| `str`   | UTF-8 string                            |
| `bool`  | `true` or `false`                       |
| `list`  | Dynamic list                            |
| `void`  | Return type for functions with no value |
| `null`  | Null literal                            |

### Variables

```lph
age: int = 20
username: str = "Alice"
var score = 100
const MAX = 999
```

### Functions

```lph
fn add(a: int, b: int) -> int:
    return a + b

async fn server_loop() -> void:
    while true:
        var got: bool = await http_accept()
        if got:
            route()
```

### Loops

```lph
while i < 10:
    i = i + 1

for i from 0 to 10 step 2:
    print(i)
```

### Lists

```lph
var values: list = [1, 2, 3]
append(values, 4)
var last = pop(values)
print(values[0])
print(values[-1])
```

### Enums

```lph
enum Status:
    Ok
    Error

var s = Status.Ok
```

### Async and concurrency

```lph
async fn worker(id: int) -> void:
    var data = await http_get("http://api.example.com")
    print("done", id)

spawn worker(1)
spawn worker(2)
```

### Stdlib modules

```lph
import from "ai"
import from "math"
import from "db"

var conn: int = db_open("data.sqlite")
db_exec(conn, "CREATE TABLE IF NOT EXISTS t (id INTEGER PRIMARY KEY, name TEXT)")
db_exec(conn, "INSERT INTO t (name) VALUES ('Alice')")
var rows = db_query_rows(conn, "SELECT * FROM t")
print(len(rows))
db_close(conn)
```

---

## How to run Liphia

**Requirement:** [Rust](https://rustup.rs/) installed.

```bash
cd src
cargo build --release -p liphia_cli
```

Then run any `.lph` file:

```bash
liphia path/to/program.lph
```

See the [main repository](https://github.com/shferreira-lab/liphia) for full documentation.

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
