# Liphia Language Reference — Engine 2.0.0

Full syntax reference for the Liphia language. For installation and project
setup, see the [root README](../../README.md). For the built-in functions,
see [`docs/spec/core.md`](../spec/core.md).

---

## Table of Contents

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
- [Core and packages](#core-and-packages)
- [Known limitations](#known-limitations)

---

## Comments

```lph
# This is a comment
print("Hello, world!")
```

---

## Primitive types

| Type    | Description                              |
|---------|------------------------------------------|
| `int`   | 64-bit signed integer                    |
| `float` | 64-bit floating-point                    |
| `str`   | UTF-8 string                             |
| `bool`  | Boolean: `true` or `false`               |
| `list`  | Dynamic list                             |
| `map`   | Associative key → value collection       |
| `void`  | Return type for functions with no value  |
| `null`  | Null literal                             |

---

## Variables

```lph
name: type = value      # typed declaration
var name: type = value  # same, with var
var name = value        # inferred declaration
const NAME: type = value
```

```lph
age: int = 20
height: float = 1.80
username: str = "Alice"
active: bool = true
var score = 100
const MAX: int = 999
```

---

## Input and output

```lph
print("Hello, world!")
print("Age:", 20)
print("Name:", username, "Score:", score)
```

```lph
name: str = input("Enter your name: ")
print("Hello,", name)
```

`input()` always returns `str`. Use `to_int()` / `to_float()` to convert:

```lph
raw: str = input("Enter a number: ")
n: int = to_int(raw)
print("Double:", n * 2)
```

---

## Operators

**Arithmetic:** `+` (also string concatenation), `-`, `*`, `/`
**Comparison:** `==`, `!=`, `>`, `<`, `>=`, `<=`
**Logical:** `and`, `or`, `not` — `and` and `or` short-circuit: the right
side is evaluated only when it can change the result.

Arithmetic rules:

- **`int` and `float` never mix implicitly.** `1 + 2.0` is a type error;
  convert one side with `to_float()` or `to_int()`.
- **`/` between two ints is integer division**, truncating toward zero:
  `7 / 2` is `3`, `-7 / 2` is `-3`. Use floats for a fractional result.
- **Integer overflow is an error**, never a wrap-around: `9223372036854775807 + 1`
  stops with `integer overflow`.
- **Division by zero is an error**, for ints and floats alike. Infinity and
  NaN only exist through `inf()` and `nan()`.

`%` and `**` are not available yet; use `pow()` and integer arithmetic
until they arrive (see [Known limitations](#known-limitations)).

---

## Conditionals

```lph
if condition:
    ...
elif other_condition:
    ...
else:
    ...
```

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

## Loops

**While:**
```lph
var i = 0
while i < 5:
    print("i =", i)
    i = i + 1
```

**For (range)** — the end value is excluded:
```lph
for i from 0 to 5:
    print(i)            # 0 1 2 3 4

for i from 0 to 10 step 2:
    print(i)            # 0 2 4 6 8
```

**Break / continue:**
```lph
for i from 0 to 10:
    if i == 5:
        break
    if i == 3:
        continue
    print(i)
```

---

## Functions

```lph
fn name(param: type, ...) -> return_type:
    ...
    return value
```

```lph
fn add(a: int, b: int) -> int:
    return a + b

fn greet(name: str) -> void:
    print("Hello,", name)

fn factorial(n: int) -> int:
    if n <= 1:
        return 1
    return n * factorial(n - 1)
```

---

## Lists

```lph
var values: list = [1, 2, 3, 4, 5]
print(values[0])    # 1
print(values[-1])   # 5
values[0] = 99
append(values, 6)
var last = pop(values)
print("length:", len(values))
```

Long literals can span several lines, with an optional trailing comma:

```lph
var matrix: list = [
    [1, 2, 3],
    [4, 5, 6],
]
```

For key → value data, use [`map`](#maps) instead of a flat list.

---

## Maps

```lph
var user: map = {"name": "Alice", "age": 30}
print(user["name"])
user["age"] = 31
user["city"] = "Recife"

print(map_keys(user))
print(map_values(user))
print(map_has(user, "city"))
map_remove(user, "city")
```

Maps keep insertion order and can hold any value type, including nested
maps and lists:

```lph
var config: map = {
    "debug": true,
    "limits": {"max_users": 100, "timeout": 30},
}
print(config["limits"]["max_users"])
```

---

## Enums

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

## Error handling — try/catch

Any runtime error — including errors raised by natives — can be caught
instead of stopping the program. The caught value is always a `str` with
the error message.

```lph
fn safe_div(a: int, b: int) -> int:
    try:
        return a / b
    catch e:
        print("caught:", e)
        return 0

print(safe_div(10, 2))   # 5
print(safe_div(1, 0))    # caught: division by zero, then 0
```

> `break`/`continue` directly inside a `try` block inside a loop can leave a
> stale handler active until the enclosing function returns — avoid
> combining them in the same block for now.

---

## String interpolation — f-strings

```lph
var name: str = "Alice"
var age: int = 30

print(f"Hello {name}, you are {age} years old")
print(f"{{literal braces}} still work, name is {name}")
print(f"math: {1 + 2 * 3}")
```

Any expression works inside `{}`, converted with the same rules as
`to_str()`. Use `{{`/`}}` for a literal brace.

---

## Async and concurrency

Functions can be declared `async`; the VM runs tasks cooperatively on a
single thread, in round-robin.

`spawn` starts a task and returns immediately:

```lph
async fn worker(name: str, steps: int) -> void:
    var i: int = 0
    while i < steps:
        print(name, "step", i)
        i = i + 1

spawn worker("a", 2)
spawn worker("b", 3)
```

`await` suspends the current task until a value is ready. On a polling
native — one that returns `false` or `null` while nothing is available,
such as `http_accept()` or `gui_next_frame()` — the task is parked and
polled again on the next scheduler tick, so other tasks keep running:

```lph
async fn serve(port: int) -> void:
    http_listen(port)
    while true:
        await http_accept()
        http_respond_json(200, json_encode({"path": http_path()}))

spawn serve(8080)
```

`await` on a user-defined `async fn` runs it to completion within the same
tick. Blocking natives (`http_get`, `tcp_recv`, `read_file`...) block the
whole VM while they run; `await` does not make them asynchronous.

---

## File imports

```lph
import "utils.lph"
import "./helpers/math_utils.lph"
```

Import cycles are detected and skipped automatically.

**Selective import** — only the listed names are made available:
```lph
import { format_name } from "./utils.lph"
```

**Qualified import** — everything is imported, but only reachable through an
alias, avoiding name collisions:
```lph
import database from "./database.lph"
import routes from "./routes.lph"

var conn: int = database.connect()
print(routes.get_users(conn))
```

If two imported files declare the same symbol without one of them being
qualified, compilation fails with a collision error instead of silently
overwriting one.

---

## Core and packages

The **core** is built into the `liphia` executable and needs no import:
strings, lists, maps, conversions, math, random, `sum`/`mean`, JSON, files,
TCP/UDP, HTTP and WebSocket. The complete list, with signatures, is in
[`docs/spec/core.md`](../spec/core.md).

```lph
print(sqrt(16), round(2.5), mean([1, 2, 3]))
write_json("data.json", {"ok": true})
```

Everything else is a **package**, installed per project and imported by
name:

```bash
liphia install num
```
```lph
import from "num"
print(dot([1.0, 2.0], [3.0, 4.0]))
```

Official packages: `num`, `stats`, `learn`, `db`, `wire`. See the
[root README](../../README.md#projects-and-packages) for installing,
versions and `liphia.lock`.

---

## Known limitations

- **`%` and `**` operators** are not implemented yet; they arrive with the
  next bytecode format (LBC v5). Use `pow()` meanwhile.
- **`try` + `break`/`continue`** in the same block, inside a loop, can leave
  a stale error handler active until the enclosing function returns.
- **Functions are not values.** They cannot be stored in variables or passed
  as arguments, which also rules out callbacks such as
  `ws_on_message(handler)` and routers that receive handlers.
- **No `sleep` or clock in the core yet**, so polling loops (like a
  WebSocket server) keep a CPU core busy.
- **Floats with no fractional part display without `.0`**: `print(4.0)`,
  `to_str(4.0)` and f-strings show `4`. `json_encode` keeps the type and
  writes `4.0`.
- **The HTTP client supports `http://` only**, not `https://`.
- **Package natives are global and unchecked by the compiler.** Once a
  package is installed its functions can be called even without `import`,
  and their arguments are validated at runtime only.
- **Reference cycles are not collected** (reference counting); see
  [`docs/spec/VM.md`](../spec/VM.md).
