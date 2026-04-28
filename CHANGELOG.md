# Changelog — Liphia Engine

All notable changes to the Liphia engine are documented here.

---

## [0.9.0] — Data structures & stdlib expansion

### Added

- List literals: `[1, 2, 3]`
- List indexing:
  - Positive indexing: `x[0]`
  - Negative indexing: `x[-1]`
- Runtime validation for invalid indexes
- Expanded standard library modules (native Rust stdlib)

### Improved

- Modular test suite split (`test_08_noai.lph`, `test_08_ai.lph`, runner)

---

## [0.8.0] — Type checking

### Added

- Type checking phase (`TypeChecker`)
- Variable type validation:
  - prevents assigning incompatible values
- Function call argument validation (arity + types)
- Basic return type checking

### Improved

- More structured compilation pipeline:
  - lex → parse → typecheck → bytecode
- Better error consistency across compiler stages

---

## [0.7.0] — Error reporting

### Added

- Unified structured error system (`LiphiaError`, `ErrorKind`)
- Standardized error messages with:
  - error code
  - line/column
  - friendly context message

### Improved

- Removed multiple `panic!()` cases and replaced with structured errors
- Clearer runtime VM errors

---

## [0.6.0] — Control flow & loops

### Added

- `while condition:`
- `for i from start to end:`
- `step` support
- `break` and `continue`
- Compiler validation:
  - `break` outside loops is rejected
  - `continue` outside loops is rejected

---

## [0.5.1] — Performance & VM optimization

### Fixed / Improved

- VM main loop now borrows opcodes by reference (`&program[pc]`)
  instead of cloning — eliminates heap allocation on every instruction
- Local variable storage replaced from `HashMap` per frame to a flat
  `Vec<Value>` indexed by slot — zero allocation per function call
- `Value::Str` now wraps `Rc<String>` instead of a plain `String` —
  string clones are reference-counted and cheap

### Performance (benchmark: `fibonacci(30)` — ~2.7M recursive calls)

| Version | Time    | vs CPython 3 |
|---------|---------|--------------|
| 0.5.0   | ~6000ms | ~36× slower  |
| 0.5.1   | ~146ms  | faster       |

---

## [0.5.0] — Imports and caching

### Added

- File imports with recursive loading
- Import cycle detection (already-visited files are skipped automatically)
- Bytecode cache (`.lbc`) for faster execution

### Improved

- Import resolution supports stdlib search paths and environment variables

---

## [0.4.0] — Functions

### Added

- Function declarations:
  - `fn name(...) -> type:`
- `return expr`
- Bytecode instructions:
  - `Call`
  - `Return`
- VM call stack with `Frame` structs (`return_pc`, `base`)
- Local variables scoped per function call via indexed `Vec<Value>`

---

## [0.3.0] — Conditionals and indentation blocks

### Added

- `if` / `elif` / `else`
- Indentation-based blocks using:
  - `Indent`
  - `Dedent`
- Bytecode instructions:
  - `Jump`
  - `JumpIfFalse`

---

## [0.2.0] — Expressions and input

### Added

- `input("prompt")` builtin — reads a line from stdin, returns `str`
- Boolean literals: `true`, `false`
- Comparison operators: `==`, `!=`, `>`, `<`, `>=`, `<=`
- Logical operators: `and`, `or`, `not`

---

## [0.1.0] — Foundation

### Added

- Rust workspace with three crates:
  - `liphia_cli`
  - `liphia_compiler`
  - `liphia_virtual_machine`
- Lexer, parser, AST, and bytecode compiler
- Stack-based VM with initial bytecode execution
- `print()` builtin
- Arithmetic operators: `+`, `-`, `*`, `/`

---

# Planned releases

## [1.0.0] — Stable release

- Real module system (`import` / `export`)
- Minimal official standard library
- Official CLI:
  - `liphia run file.lph`
- Public documentation and stable language spec
- First stable release