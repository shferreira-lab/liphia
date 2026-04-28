# Liphia (.lph)

> A general-purpose programming language with Python-style indentation syntax,
> strong static typing, and a bytecode virtual machine written in Rust.

[![Engine](https://img.shields.io/badge/engine-0.5.1-blueviolet)](https://github.com/shferreira-lab/liphia)
[![Language](https://img.shields.io/badge/rust-core%20engine-orange)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/status-active%20development-yellow)](https://github.com/shferreira-lab)
[![License](https://img.shields.io/badge/license-MIT%20AND%20Apache--2.0-blue)](./LICENSE-MIT)

Liphia is a general-purpose language with indentation-delimited syntax
(similar to Python), explicit static typing, and a bytecode compiler
and virtual machine both written in Rust.

**Not a toy language.** Liphia has its own compiler, bytecode format,
and VM — no external runtime, no interpreter dependency, no GC overhead.
The compiled binary is fully self-contained.

> **Notice:** The project is under active development. Full documentation,
> stable releases, and the standard library will be published when the
> engine reaches sufficient stability.

---

## Table of Contents

- [Repository layout](#repository-layout)
- [How to run](#how-to-run)
- [Current syntax — Engine 0.5.1](#current-syntax--engine-051)
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
└── liphia_engine/
    ├── crates/
    │   ├── liphia_cli/              # entry point — reads .lph files and runs the VM
    │   │   ├── Cargo.toml
    │   │   └── src/main.rs
    │   ├── liphia_compiler/         # lexer, parser, AST, bytecode compiler
    │   │   ├── Cargo.toml
    │   │   └── src/
    │   │       ├── ast.rs
    │   │       ├── bytecode.rs
    │   │       ├── lexer.rs
    │   │       ├── lib.rs
    │   │       └── parser.rs
    │   └── liphia_virtual_machine/  # stack-based VM that executes bytecode
    │       ├── Cargo.toml
    │       └── src/
    │           ├── lib.rs
    │           ├── opcode.rs
    │           ├── value.rs
    │           └── vm.rs
    ├── Cargo.lock
    └── Cargo.toml
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
cargo run -p liphia_cli -- ../liphia-examples/hello.lph
```

**Windows (PowerShell):**
```powershell
cargo run -p liphia_cli -- ..\liphia-examples\hello.lph
```

After the first build you can run the binary directly:

```powershell
.\target\debug\liphia_cli.exe path\to\program.lph
```

Input and output (`print`, `input`) happen directly in the terminal.

---

## Current syntax — Engine 0.5.1

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

---

### Variables

```lph
name: type = value
```

```lph
age: int = 20
height: float = 1.80
username: str = "Cristiane"
active: bool = true
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
> as a return type annotation; full validation is planned for engine 0.8.

---

### File imports

```lph
import "./utils.lph"
```

> In the current engine, `import` works as a **source-level include**:
> the contents of the imported file are inserted before compilation.
> Recursive imports are supported. Import cycles are detected and
> skipped automatically.
>
> Real module support is planned for **Engine 1.0**:
> ```lph
> import "io"
> ```

---

## Full example

```lph
import "./utils.lph"

print("=== Liphia Demo ===")

username: str = input("Enter your name: ")
print("Hello, ", username)

fn add(a: int, b: int) -> int:
    return a + b

x: int = add(10, 5)
print("10 + 5 = ", x)

age: int = 17

if age >= 18:
    print("Adult")
elif age == 17:
    print("Almost there")
else:
    print("Minor")

print("Done.")
```

---

## Roadmap

### ✅ Engine 0.5.1 — current

- [x] Statically typed variables
- [x] Output and input (`print`, `input`)
- [x] Conditionals (`if`, `elif`, `else`)
- [x] Typed functions with explicit return
- [x] File import by source include
- [x] Arithmetic, comparison, and logical operators
- [x] Performance: Fibonacci(30) ~146ms (below CPython 3 ~161ms on the same hardware)

### 🔧 Engine 0.6 — Loops *(planned)*

- [ ] `while`
- [ ] `for`
- [ ] `break`
- [ ] `continue`

### 🔧 Engine 0.7 — Better errors *(planned)*

- [ ] Error reporting without `panic!`
- [ ] Line and column in error messages
- [ ] Friendly, readable error output

### 🔧 Engine 0.8 — Real type checking *(planned)*

- [ ] Type validation in expressions
- [ ] Return type validation
- [ ] Parameter type validation

### 🔧 Engine 0.9 — Data structures *(planned)*

- [ ] Lists: `list[int]`
- [ ] Maps: `map[str -> int]`
- [ ] Indexing: `x[0]`

### 🎯 Engine 1.0 — Stable release *(planned)*

- [ ] Real module system (`import` / `export`)
- [ ] Minimal official standard library
- [ ] Official CLI: `liphia run file.lph`
- [ ] Public license and documentation
- [ ] First stable release

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
