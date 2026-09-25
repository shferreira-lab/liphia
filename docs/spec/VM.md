# Liphia VM semantics

Rules every Liphia VM must follow, independent of implementation.
Each rule marked with a case number is enforced by the conformance
suite (`conformance/cases/`).

## Numbers

- `int` is a signed 64-bit integer; `float` is IEEE 754 double.
- **No implicit coercion.** Mixing `int` and `float` in `+ - * /` or
  `< > <= >=` is a compile error when both types are known (case 103).
  The VM also rejects it at runtime as a safety net. Convert explicitly
  with `to_float()` / `to_int()`.
- **Integer overflow is an error**, never a wrap-around, in every build
  mode. This includes `i64::MIN / -1` and `0 - i64::MIN` (cases 011, 104).
- **Division by zero is an error** for both `int` and `float` (cases 007,
  012, 102). Integer division truncates toward zero (`7 / 2 == 3`).
- Non-finite floats are only reachable through the core constants
  `inf()` and `nan()` (case 013).
- All runtime errors above are catchable with `try/catch`.

## Logic

- `and` / `or` short-circuit: the right side runs only when it can change
  the result (case 010).
- Both operands must be `bool`; the result is always `bool`.

## Memory

- Lists and maps are reference-counted and shared by reference.
- **Known limitation:** reference cycles are never freed. A list that
  contains itself (`append(xs, xs)`) leaks for the life of the process.
  A cycle collector is future work.

## Input

- `input(prompt)` prints the prompt, then reads one line.
- A host without a terminal (the GUI) supplies lines through
  `VM::set_input_hook`. While the hook has no line, the task yields and
  retries, so the host's event loop is never blocked.
