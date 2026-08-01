# Liphia Language Reference — Engine 1.0.0

Full syntax reference for the Liphia language. For build/run instructions and
project setup, see the [root README](../../README.md). For the standard
library, see [`docs/stdlib/`](../stdlib/README.md).

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
- [Stdlib modules](#stdlib-modules)
- [Known limitations](#known-limitations)

---

## Comments

```lph
# This is a comment
print("Hello, world!")
```

---

## Primitive types

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

## Variables

```lph
name: type = value      # typed declaration
var name = value        # inferred declaration
const NAME = value      # constant
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

**Arithmetic:** `+` (also string concat), `-`, `*`, `/`
**Comparison:** `==`, `!=`, `>`, `<`, `>=`, `<=`
**Logical:** `and`, `or`, `not`

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

**For (range):**
```lph
for i from 0 to 5:
    print(i)

for i from 0 to 10 step 2:
    print(i)
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

Maps can hold any value type, including nested maps and lists:

```lph
var config: map = {"debug": true, "limits": {"max_users": 100, "timeout": 30}}
print(config["limits"]["max_users"])
```

> Map/list literals must be written on a single line — see
> [Known limitations](#known-limitations).

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

Any runtime error — including from stdlib calls — can be caught instead of
crashing the whole program. The caught value is always a `str` with the
error message.

```lph
fn risky() -> void:
    try:
        var conn: int = db_open("some/invalid/path.sqlite")
        db_exec(conn, "INSERT INTO x VALUES (1)")
    catch e:
        print("caught:", e)
    print("execution continues normally")
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

> An interpolated expression containing a string literal with `"` (e.g.
> `f"{some_fn(\"x\")}"`) doesn't parse correctly yet — avoid nested string
> literals inside `{}`.

---

## Async and concurrency

Functions can be declared `async` and awaited inside other async functions.
The VM runs tasks cooperatively in a single-threaded event loop.

```lph
async fn fetch(url: str) -> str:
    var result = await http_get(url)
    return result
```

`spawn` launches a task concurrently (fire-and-forget):

```lph
async fn worker(id: int) -> void:
    print("worker", id, "running")

spawn worker(1)
spawn worker(2)
```

`await` on a native function (e.g. `http_accept()`, `gui_next_frame()`)
polls it once per scheduler tick until it signals ready. `await` on a
user-defined `async fn` runs it to completion synchronously within the
same tick — either way, the pattern reads the same from script.

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

## Stdlib modules

```bash
liphia install math
```
```lph
import from "math"
print(sqrt(16.0))
```

See [`docs/stdlib/README.md`](../stdlib/README.md) for the module list and
[`docs/stdlib/REFERENCE.md`](../stdlib/REFERENCE.md) for the full function
reference.

---

## Known limitations

- **Map/list literals must be single-line.** The lexer doesn't yet suppress
  layout tokens (`Newline`/`Indent`/`Dedent`) inside `{}`/`[]` the way it
  already does inside `()` — a multi-line map/list literal fails to parse.
- **`try` + `break`/`continue`** in the same block, inside a loop, can leave
  a stale error handler active until the enclosing function returns.
- **Nested string literals inside f-string interpolation** don't parse
  correctly (`f"{fn(\"x\")}"`).
- **`ws_on_connect`/`ws_on_message`/`ws_on_disconnect`** (event-callback
  registration for the `ws` module) are planned but not implemented — they
  need native-side dispatch into a Liphia function, which a composed `.lph`
  layer can't provide.