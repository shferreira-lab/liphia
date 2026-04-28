# Liphia (.lph)

> A general-purpose programming language with Python-style indentation syntax,
> strong static typing, and a bytecode virtual machine written in Rust.

[![Engine](https://img.shields.io/badge/engine-0.9.0-blueviolet)](https://github.com/shferreira-lab/liphia)
[![Language](https://img.shields.io/badge/rust-core%20engine-orange)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/status-active%20development-yellow)](https://github.com/shferreira-lab)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](./LICENSES)

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
- [How to run](#how-to-run)
- [Current syntax — Engine 0.9.0](#current-syntax--engine-090)
  - [Comments](#comments)
  - [Primitive types](#primitive-types)
  - [Variables](#variables)
  - [Input and output](#input-and-output)
  - [Operators](#operators)
  - [Conditionals](#conditionals)
  - [Functions](#functions)
  - [File imports](#file-imports)
- [Full example](#full-example)
- [Roadmap](#roadmap)
- [License](#license)

---

## Repository layout

```
liphia/
│
├── examples/ # example .lph programs
│
├── licenses/ # licenses
│
│
└── liphia_engine/
│    └── crates/
│         ├── liphia_cli/ # CLI runner + REPL shell
│         ├── liphia_compiler/ # lexer, parser, AST, bytecode compiler
│         ├── liphia_core_native/ # Core string and value utilities.
│         └── liphia_virtual_machine/ # bytecode VM
│
├── stdlib/ # Liphia standard library
│    ├── lph/ # .lph modules (import from "...")
│    └── native/ # Rust native stdlib bindings
│
├── tests/ # benchmarks/tests
│
├── tools/
│      └── liphia-vscode/ # Extension for VS Code
│
├── CHANGELOG.md
└── README.md
```

---

## How to run

**Requirement:** [Rust installed](https://rustup.rs/).

Inside the `liphia_engine/` workspace, run any `.lph` file with Cargo:

```bash
cargo run -p liphia_cli -- path/to/program.lph
```

**Linux / macOS:**
```bash
cargo run -p liphia_cli -- ../examples/hello.lph
```

**Windows (PowerShell):**
```powershell
cargo run -p liphia_cli -- ..\examples\hello.lph
```

After the first build you can run the binary directly:

```powershell
.\target\debug\liphia_cli.exe path\to\program.lph
```

Input and output (`print`, `input`) happen directly in the terminal.

---
**Bytecode cache**

The CLI automatically caches compiled bytecode into:

```
examples/liphia_cache/*.lbc
```
So future runs are instant unless the source changes.


**REPL / Shell**


Liphia includes an interactive shell:
```
cargo run -p liphia_cli -- --repl
```
Or just:
```
cargo run -p liphia_cli
```


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
| `int`   | Integer numbers                         |
| `float` | Floating-point numbers                  |
| `str`   | Strings                                 |
| `bool`  | Boolean values (`true` or `false`)      |
| `void`  | Return type for functions with no value |
| `list`  | List type                               |
| `null`  | Null literal                            |
---

### Variables

```lph
name: type = value
```

```lph
age: int = 20
height: float = 1.80
username: str = "Peter"
active: bool = true
nothing: null = null
```

---

### Input and output

**Output:**
```lph
print("Hello, world!")
print("Age: ", 20)
```

**Input:**
```lph
username: str = input("Enter your name: ")
print("Hello, ", username)
```

> `input()` always returns `str` in the current engine.
> Type conversion will be added in a future release.

---

### Operators

**Arithmetic:**

| Operator | Operation      |
|----------|----------------|
| `+`      | Addition       |
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

Comparisons return `bool`.

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

### Functions

```lph
fn name(param: type, ...) -> return_type:
    ...
    return value
```

**Example:**
```lph
fn add(a: int, b: int) -> int:
    return a + b

result: int = add(10, 5)
print("Result: ", result)
```

**Void function (no return value):**
```lph
fn greet(name: str) -> void:
    print("Hello, ", name)
```

> `void` is part of the type system spec. The compiler accepts it
> as a return type annotation; full validation is planned for engine 1.0.


### List

```lph
values: list = [1, 2, 3]
words: list = ["a", "b", "c"]

```


### Indexing

```lph
values: list = [10, 20, 30]

print(values[0])    # 10
print(values[-1])   # 30

```



---

### File imports

Relative import:

```lph
import "./utils.lph"
```


Absolute import (Windows):

```lph
import "C:/Dev/project/utils.lph"
```


or on Linux/macOS:

```lph
import "/home/user/project/utils.lph"
```
File imports are resolved relative to the current script.
Cycles are detected and skipped.



### Stdlib modules use:

```lph
import from "math"
import from "ai"
```
Stdlib modules are resolved from stdlib/lph/.



> In the current engine, `import` works as a **source-level include**:
> the contents of the imported file are inserted before compilation.
> Recursive imports are supported. Import cycles are detected and
> skipped automatically.
>
> import { OnlyOne } from "prelude" is planned for **Engine 1.0**:
> 

---

## Full example

```lph
import from "math"

print("=== Liphia Demo ===")

fn add(a: int, b: int) -> int:
    return a + b

x: int = add(10, 5)
print("10 + 5 = ", x)

values: list = [1, 2, 3, 4]
print("first = ", values[0])
print("last = ", values[-1])

print("sqrt(16) = ", sqrt(16))

print("Done.")
```

---

## Roadmap

### ✅ Engine 0.9.0 — current
Lexer + Parser + AST
- [X] Indentation blocks (Indent / Dedent)
- [X] Typed variables
- [X] Conditionals (if, elif, else)
- [X] Functions (fn, return)
- [X] Bytecode compiler + VM execution
- [X] REPL / Shell (--repl)
- [X] File import system (import "file.lph")
- [X] Stdlib import system (import from "module")
- [X] Lists + indexing (x[0], x[-1])
- [X] Better structured errors (compiler + VM)
- [X] Bytecode cache (.lbc)


### 🎯 Engine 1.0 — Stable release *(planned)*

- [ ] Real module system with exports/imports
- [ ] map / dictionaries
- [ ] Better diagnostics with source context
- [ ] Official package manager / module loader
- [ ] Stable stdlib API

---

## License

Licensed under either of:

- [MIT License](./LICENSE-MIT)
- [Apache License, Version 2.0](./LICENSE-APACHE)

at your option.

---

<p align="center">
  Built with ❤️ &nbsp;·&nbsp; Engine written in Rust &nbsp;·&nbsp;
  <a href="https://github.com/shferreira-lab">shferreira-lab</a>
</p>
