# Changelog

All notable changes to the Liphia engine are documented here.

---

## [0.9.0] — Engine 0.9.0

### Added

**Language**
- Indentation-based block syntax (Indent / Dedent tokens)
- Typed variable declarations (`name: type = value`)
- Inferred declarations (`var name = value`)
- Constants (`const name = value`)
- Optional types (`T?`, e.g. `int?`, `str?`)
- Conditionals: `if`, `elif`, `else`
- Loops: `while`, `for from/to/step`, `break`, `continue`
- Functions: `fn`, `return`, full recursion support
- Async functions: `async fn`, `await`
- Concurrency: `spawn` (fire-and-forget coroutine launch)
- Lists: literals, indexing (`x[0]`, `x[-1]`), `append`, `pop`
- Enums: `enum` declaration and variant access (`Enum.Variant`)
- Logical operators: `and`, `or`, `not` (also `&&`, `||`, `!`)
- String concatenation via `+`

**Compiler**
- Lexer with structured error reporting (line + column)
- Parser producing a typed AST
- Static type checker with forward-reference support for functions and enums
- Bytecode compiler with two-pass function resolution
- `async fn` compiled to suspendable coroutine entry points
- `spawn` resolved to `Opcode::Spawn` at compile time
- `await` resolved to `Opcode::Suspend` with polling semantics
- `break` and `continue` with correct patch-back to loop boundaries
- `import "file.lph"` — source-level file inclusion with cycle detection
- `import from "module"` — stdlib module resolution

**Virtual Machine**
- Stack-based bytecode VM with call frames and local variable slots
- Cooperative event-loop scheduler: round-robin task queue (`VecDeque<Task>`)
- `Opcode::Suspend` — polling model for async native calls
- `Opcode::Spawn` — creates independent coroutine tasks
- Quantum-based scheduling (256 instructions per task per tick)
- Values: `Int`, `Float`, `Bool`, `Str`, `List`, `EnumVariant`, `Null`
- `Rc<RefCell<Vec<Value>>>` for shared mutable lists
- Division by zero runtime error
- Stack underflow detection with context label

**CLI**
- `liphia <file.lph>` — compile and run
- `liphia --repl` — interactive shell
- `liphia init` — initialize `liphia.toml` in current directory
- `liphia install <module>` — download module from registry to `liphia_modules/`
- `liphia install` — install all dependencies from `liphia.toml`
- `liphia install --list` — list available stdlib modules
- `--no-cache` flag to force recompilation
- Bytecode cache: `.lbc` files in `liphia_cache/` alongside source
- FNV-based source hash for cache invalidation
- Binary cache format with magic bytes, version, and hash validation

**Core native functions** (always available, no import required)
- `len`, `to_int`, `to_float`, `to_str`
- `trim`, `upper`, `lower`
- `contains`, `starts_with`, `ends_with`
- `replace`, `split`
- `append`, `pop`

**Standard library** (install via `liphia install <name>`)
- `ai` — `sigmoid`, `relu`, `softmax`, `argmax`, `dot`, `norm`, `matrix_new`, `matrix_mul`, `matrix_add`
- `math` — `sqrt`, `pow`, `abs`, `floor`, `ceil`, `round`, `min`, `max`, `pi`, `e`, `log`, `log10`, `sin`, `cos`, `tan`
- `stats` — `sum`, `mean`, `median`, `variance`, `stdev`, `min_list`, `max_list`, `count`
- `fs` — `read_file`, `write_file`, `append_file`, `file_exists`
- `http` — `http_listen`, `http_accept`, `http_method`, `http_path`, `http_body`, `http_respond`, `http_respond_json`, `http_get`, `http_post`, `http_put`, `http_patch`, `http_delete`, `http_status`, `http_header`, `http_query`
- `ws` — `ws_listen`, `ws_accept`, `ws_clients`, `ws_send`, `ws_recv`, `ws_broadcast`, `ws_close`
- `net` — `tcp_connect`, `tcp_send`, `tcp_recv`, `tcp_close`, `udp_send`
- `json` — `json_encode`, `json_decode`, `json_get`, `json_has`

**Tooling**
- VS Code extension with syntax highlighting for `.lph` files
- Workspace layout: `src/` as unified Cargo workspace covering `liphia_engine` and `stdlib/native`

---

## [Unreleased] — Engine 1.0 *(planned)*

- Maps / dictionaries
- Selective imports: `import { fn } from "module"`
- Better diagnostics with source context lines and fix suggestions
- Stable stdlib API with full documentation
- GitHub Actions: automated binary releases for Windows, Linux, macOS