# Changelog

All notable changes to Liphia are documented here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/); versions below 1.0.0 are
summarized from the project's pre-release history rather than tracked
entry-by-entry.

---

## [1.0.0] — 2026-08-01

First stable release. Engine crates, the standard library, and the VS Code
extension all start at 1.0.0 together; each moves independently from here.

### Added

- **GUI/games support**, as a fully separate compiled binary (`liphia_cli_gui`),
  never part of the standard `liphia_cli` build:
  - New `liphia_gui_native` crate — toolkit-agnostic native functions
    (`gui_heading`, `gui_label`, `gui_separator`, `gui_button`, `gui_next_frame`)
    that push commands into a thread-local queue instead of depending on
    `egui` directly.
  - New `liphia_cli_gui` crate — hosts an `eframe`/`egui` window, ticks the
    VM once per real frame via a new `VmSession::tick()` API, and renders
    queued GUI commands.
  - Widgets are driven entirely from `.lph` scripts (`while true: ...draws...
    await gui_next_frame()`), not hardcoded in the host.
  - Root `Cargo.toml` gained `default-members` so `liphia_cli_gui` and
    `liphia_gui_native` never build on a plain `cargo build` — opt-in only
    via `-p liphia_cli_gui`.
- **New `liphia_pipeline` crate** — import resolution and the
  compile/type-check pipeline extracted from `liphia_cli`'s `main.rs`, shared
  between `liphia_cli` and `liphia_cli_gui`. Also gained
  `compile_with_externals`, letting a host (like the GUI) register natives
  the standard `TypeChecker` doesn't know about.
- **REPL**: multi-line input (`fn`, `if`, `while`, `try`, ...) now
  accumulates until an explicit `run` command, instead of guessing when a
  block is "done" from indentation.
- **Composed `.lph` layers** added across the standard library — pure-Liphia
  helper functions built on top of existing natives (see stdlib versions
  below for the full list per module): `ai` (`classification_report`,
  `is_better`), `stats` (`describe`, `compare_groups`), `http` (`ok_json`,
  `error_json`, and friends), `fs` (`read_json`, `write_json`,
  `append_json_line`), `net` (`tcp_recv_all`, `tcp_send_json`,
  `tcp_recv_json`), `ws` (`ws_send_json`, `ws_broadcast_json`).

### Fixed

- **Async scheduler — task never re-polled.** `Opcode::Suspend` didn't
  rewind the task's program counter back to the preceding native call when
  the awaited value wasn't ready, so a polled native (e.g. `http_accept()`)
  was only ever invoked once per task, then looped forever checking a stale
  value. Fixed in `vm.rs`.
- **Async scheduler — `void` async fn treated as "not ready".** A `void`
  `async fn`'s implicit `Value::Null` return was indistinguishable from a
  native's genuine "not ready yet" polling signal, causing an already-completed
  async function to be re-invoked forever and crash on invalid state. Fixed at
  the root in `bytecode.rs`: `Opcode::Suspend` is now only ever emitted after
  a call to a native, never after a call to a user-defined `async fn` (which
  always completes synchronously within the same scheduler tick).
- Together, these two fixes produced the first fully working async Liphia
  HTTP server end-to-end (validated against a real multi-route REST example
  under sustained curl + browser traffic).

### Changed

- Standard library module versions: `ai` 1.1.0, `db` 1.1.0, `fs` 1.1.0,
  `http` 1.2.0, `json` 1.2.0, `math` 1.1.0, `net` 1.1.0, `stats` 1.1.0,
  `ws` 1.1.0. Overall `stdlib` package: 1.0.0.
- VS Code extension: 1.0.0.
- REPL stays at 0.4.0 — still developer-facing, not part of the 1.0.0
  stability claim.

### Documentation

- Trimmed the root `README.md` down to build/run/project-setup — full syntax
  moved to `docs/language/README.md`.
- New consolidated stdlib reference (`docs/stdlib/REFERENCE.md`) — every
  module's native vs. composed functions in one versioned place, replacing
  scattered per-module descriptions.

---

## [0.10.0] and earlier — pre-1.0 development

Summarized from the pre-release development history:

### Added
- Indentation-based blocks, typed/inferred/const variable declarations
- Conditionals, loops (`while`, `for from/to/step`), `break`/`continue`
- Functions, recursion, `async fn` / `await` / `spawn` (cooperative
  single-threaded event-loop VM)
- Bytecode compiler and VM, REPL, bytecode cache (`.lbc`)
- File import system (`import "file.lph"`) and stdlib import system
  (`import from "module"`)
- Package manager (`liphia init`, `liphia install`, `liphia install --list`)
- Lists and indexing (`append`, `pop`, `keys`, negative indices)
- **Maps/dictionaries** — `map` type, `{key: value}` literals, `map_keys`,
  `map_values`, `map_has`, `map_remove`
- Enums and variant matching
- **Recoverable error handling** (`try`/`catch`) — runtime errors no longer
  crash the whole process unconditionally
- **Real module system** — qualified imports (`import alias from "..."`),
  selective imports (`import { fn } from "..."`), compile-time
  name-collision detection across imported files
- **String interpolation** (f-strings)
- Accurate line/column error reporting
- Standard library: `ai`, `math`, `stats`, `fs`, `http`, `ws`, `net`, `json`,
  `db` — `http` gained native CORS (1.1.0); `json` gained real `map`/`list`
  decoding instead of a flat list (2.0.0, breaking); `db` gained SQLite
  (bundled) + PostgreSQL (pure TCP wire protocol) with installable
  `sqlite`/`postgres` submodules
- GitHub Actions cross-platform release automation (tag `v0.10.0`)
- VS Code extension 0.1.0 (syntax highlighting, snippets)