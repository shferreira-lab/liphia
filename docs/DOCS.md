# Liphia Documentation

Welcome to the official Liphia language documentation.

Liphia is a general-purpose programming language with indentation-based
syntax, strong static typing, and a bytecode compiler and virtual machine
written in Rust.


---

## Table of Contents

- [Getting Started](#getting-started)
  - [Requirements](#requirements)
  - [Hello, World!](#hello-world)
  - [Running a file](#running-a-file)
- [Language Reference](#language-reference)
  - [Comments](#comments)
  - [Types](#types)
  - [Variables](#variables)
  - [Operators](#operators)
  - [Input and Output](#input-and-output)
  - [Conditionals](#conditionals)
  - [Functions](#functions)
  - [Imports](#imports)
- [How the engine works](#how-the-engine-works)
- [Engine status](#engine-status)

---

## Getting Started

### Requirements

- [Rust](https://rustup.rs/) (stable toolchain)
- Git

Clone the repository and enter the engine workspace:

```bash
git clone https://github.com/shferreira-lab/liphia.git
cd liphia/liphia_engine
```

---

### Hello, World!

Create a file called `hello.lph`:

```lph
print("Hello, world!")
```

Run it:

```bash
cargo run -p liphia_cli -- hello.lph
```

Expected output:

```
Hello, world!
```

That's it. No boilerplate, no imports required for basic output.

---

### Running a file

The general command is:

```bash
cargo run -p liphia_cli -- path/to/your/file.lph
```

After the first build, you can also use the compiled binary directly:

```bash
# Linux / macOS
./target/debug/liphia_cli path/to/your/file.lph

# Windows
.\target\debug\liphia_cli.exe path\to\your\file.lph
```

---

## Language Reference

### Comments

Lines starting with `#` are comments and are ignored by the compiler.

```lph
# This is a comment
print("This runs")  # inline comment
```

Multi-line comments are **not yet implemented**. Use `#` on each line.

---

### Types

Liphia is statically typed. Every variable must have a declared type.

| Type    | Description                             | Example value  |
|---------|-----------------------------------------|----------------|
| `int`   | Integer number                          | `42`           |
| `float` | Floating-point number                   | `3.14`         |
| `str`   | Text string                             | `"hello"`      |
| `bool`  | Boolean                                 | `true`, `false`|
| `void`  | No return value (functions only)        | —              |

> **Note:** Lists and maps are planned for engine 0.9 and are not
> available yet.

---

### Variables

Variable declaration follows the pattern:

```lph
name: type = value
```

Examples:

```lph
age: int = 25
price: float = 9.99
greeting: str = "Hello"
active: bool = true
```

Variables must be initialized at declaration. There is no `null` or
`undefined` — every value must be assigned explicitly.

---

### Operators

**Arithmetic:**

```lph
x: int = 10 + 5   # 15
y: int = 10 - 3   # 7
z: int = 4 * 3    # 12
w: float = 9 / 2  # 4 (integer division — float division planned)
```

**Comparison** (return `bool`):

```lph
10 == 10   # true
10 != 5    # true
10 > 5     # true
3 < 7      # true
5 >= 5     # true
4 <= 9     # true
```

**Logical:**

```lph
true and false   # false
true or false    # true
not true         # false
```

**Grouping:**

```lph
result: int = (10 + 5) * 2   # 30
```

---

### Input and Output

**Output — `print()`:**

Accepts one or more arguments separated by commas.

```lph
print("Hello!")
print("Name: ", "Sergio")
print("Result: ", 10 + 5)
```

Each call to `print()` outputs a new line.

**Input — `input()`:**

Displays a prompt and reads a line from stdin. Always returns `str`.

```lph
name: str = input("Enter your name: ")
print("Hello, ", name)
```

> Type conversion (e.g. `str` to `int`) is planned for a future release.
> For now, all input is treated as text.

---

### Conditionals

Liphia uses `if`, `elif`, and `else`. Blocks are delimited by
indentation (4 spaces recommended).

```lph
if condition:
    # block executed if condition is true
elif other_condition:
    # block executed if the first was false and this is true
else:
    # block executed if all conditions above were false
```

`elif` and `else` are optional.

**Example:**

```lph
score: int = 75

if score >= 90:
    print("A")
elif score >= 75:
    print("B")
elif score >= 60:
    print("C")
else:
    print("F")
```

**Compound conditions:**

```lph
age: int = 22

if age >= 18 and age <= 65:
    print("Working age")
```

---

### Functions

Functions are declared with `fn`, receive typed parameters, and declare
their return type with `->`.

```lph
fn name(param1: type, param2: type) -> return_type:
    ...
    return value
```

**Example:**

```lph
fn add(a: int, b: int) -> int:
    return a + b

result: int = add(10, 5)
print("10 + 5 = ", result)
```

**Void function (no return value):**

```lph
fn greet(name: str) -> void:
    print("Hello, ", name)

greet("Sergio")
```

**Recursive function:**

```lph
fn fibonacci(n: int) -> int:
    if n <= 1:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)

print("fib(10) = ", fibonacci(10))
```

> Functions must be called after they are defined in the current engine.
> Forward declarations are not yet supported.

---

### Imports

You can split your code into multiple `.lph` files and import them.

```lph
import "./utils.lph"
```

The imported file's contents are inserted at that point before
compilation — it works like a source-level include.

**utils.lph:**
```lph
fn double(x: int) -> int:
    return x * 2
```

**main.lph:**
```lph
import "./utils.lph"

result: int = double(7)
print(result)
```

Rules:
- Paths are relative to the importing file
- Recursive imports are supported
- Import cycles are detected and skipped automatically — a file is
  never included more than once

> Real module support (named imports, exports, standard library modules)
> is planned for **Engine 1.0**.

---

## How the engine works

When you run a `.lph` file, the engine goes through these stages:

```
Source file (.lph)
      │
      ▼
  [CLI] load_file_recursive()
      │  resolves imports, assembles full source string
      ▼
  [Lexer] Lexer::next_token()
      │  tokenizes source into a stream of tokens
      │  handles indentation → Indent / Dedent tokens
      ▼
  [Parser] Parser::parse()
      │  builds an Abstract Syntax Tree (AST)
      │  from the token stream
      ▼
  [Compiler] generate_bytecode()
      │  walks the AST and emits a flat Vec<Opcode>
      │  resolves function addresses (CallNamed → Call)
      ▼
  [VM] VM::run()
       executes the opcode list on a stack-based virtual machine
       stack: operand stack (Vec<Value>)
       locals: flat Vec<Value> indexed by slot per frame
       globals: HashMap<String, Value>
       frames: Vec<Frame> — call stack with return address and base slot
```

The VM is purely stack-based. Values on the stack are:
`Value::Int(i64)`, `Value::Float(f64)`, `Value::Bool(bool)`,
`Value::Str(Rc<String>)`.

There is no garbage collector — memory is managed by Rust's ownership
system and `Rc` for shared string values.

---

## Engine status

Current version: **0.9.0**

| Feature                        | Status         |
|-------------------------------|----------------|
| Variables (int, float, str, bool) | ✅ Implemented |
| `print()` and `input()`        | ✅ Implemented |
| Arithmetic operators           | ✅ Implemented |
| Comparison operators           | ✅ Implemented |
| Logical operators (`and`, `or`, `not`) | ✅ Implemented |
| Conditionals (`if`, `elif`, `else`) | ✅ Implemented |
| Functions with typed params and return | ✅ Implemented |
| File imports (source include)  | ✅ Implemented |
| Loops (`while`, `for`)         | ✅ Implemented |
| Error reporting (line/column)  | ✅ Implemented |
| Type checking and validation   | ✅ Implemented |
| Lists and maps                 | ✅ Implemented |
| Real module system             | 🔧 Planned 1.0 |
| Standard library               | 🔧 Planned 1.0 |
