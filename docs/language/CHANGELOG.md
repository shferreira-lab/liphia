# Changelog

All notable changes to Liphia are documented here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/); versions below 1.0.0 are
summarized from the project's pre-release history rather than tracked
entry-by-entry.

---

## [2.0.0] — 2026-09-26

The standard library is replaced by a **core** built into the executable
plus independently versioned **packages**, with a versioned package
manager and installers for Windows, Linux and macOS. This release breaks
programs written for 1.0.0; see "Migrating from 1.0.0" below.

### Added

- **Core natives** (`liphia_core_native`), available without import:
  strings, lists, maps, scalar math, random, aggregation, JSON, files,
  TCP/UDP, HTTP server and client, WebSocket server. Reference:
  `docs/spec/core.md`.
- New core natives that replace the old composed `.lph` layers:
  `read_json`, `write_json`, `append_json_line`, `tcp_recv_all`,
  `tcp_send_json`, `tcp_recv_json`, `ws_send_json`, `ws_broadcast_json`.
- **Official packages**, each with its own version: `num` 1.0.0 (vectors,
  matrices, descriptive statistics), `stats` 1.0.0 (hypothesis tests),
  `learn` 1.0.0 (machine learning primitives), `db` 2.0.0 (SQLite and
  PostgreSQL), `wire` 1.0.0 (JSON responses for HTTP APIs).
- `p_value_wilcoxon` in `stats`; `compare_groups(a, b, paired)` now covers
  paired samples (paired t-test or Wilcoxon).
- **Versioned package manager**: `install` with npm-style requirements
  (`num@1.2.9`, `^`, `~`, `@latest`), transitive dependencies, `liphia.lock`,
  `install --frozen`, `update`, `remove`, `list`, a per-machine download
  cache, and `LIPHIA_REGISTRY` for installing from a local folder.
- **`liphia_manifest` crate** for `liphia.toml`, `liphia.lock`,
  `package.toml` and the package index.
- **Native package ABI check**: package libraries carry an ABI tag (engine
  version + rustc) and the VM refuses a mismatched library with an error
  instead of loading it.
- **Installers**: Inno Setup installer for Windows (per-user or all users,
  optional GUI, PATH, `.lph` icon), `install.ps1` and `install.sh`.
- **Release automation**: tags `vX.Y.Z` publish the engine, tags
  `<package>-vX.Y.Z` publish a package, both built for Windows, Linux and
  macOS; `THIRD_PARTY_LICENSES.txt` is generated with cargo-about.
- `liphia version` (engine version and ABI tag) and `liphia help`.
- Conformance cases 201–212 for the core natives.
- New `examples/` folder covering the core and every package.

### Changed

- The executables are now named **`liphia`** and **`liphia-gui`**.
- All engine crates share one version (2.0.0); the Rust toolchain is
  pinned in `src/rust-toolchain.toml`.
- `min`, `max`, `clamp`, `sum`, `min_list` and `max_list` no longer mix int
  and float, and keep the argument type (`sum([1, 2])` is the int `3`).
- `json_decode` is strict (trailing content is an error, duplicated keys
  keep the last value); `json_encode` always writes floats with `.0` or an
  exponent, so they decode back as floats.
- Random numbers use SplitMix64 seeded from the clock; `seed(n)` makes a
  run reproducible; `rand_int` has no modulo bias; `shuffle` accepts any
  element type and returns a new list.
- HTTP server parses each connection on its own thread and rejects bodies
  over 10 MB; the HTTP client sends HTTP/1.0 and rejects `https://`.
- Integer results that do not fit in `int` are errors in `abs`, `gcd`,
  `lcm`, `factorial` and `sum`; `floor`, `ceil` and `round` reject NaN and
  infinity; ports outside 1–65535 are rejected.
- Packages use `package.toml` (replaces `module.toml`) and are downloaded
  from GitHub Releases instead of being committed to the repository.
- The type checker only declares core natives; calls to package natives
  type-check as `unknown`.

### Fixed

- `spawn f(a, b)` passed its arguments in reverse order.
- WebSocket server on Windows: accepted sockets inherited non-blocking mode,
  so every handshake failed; handshakes now run on their own thread, so a
  silent pre-opened browser socket no longer blocks real connections.
- WebSocket frames: no more lost bytes on partial reads; ping is answered,
  close and disconnects remove the client, oversized frames are refused.
- `clamp` with `lo > hi` and `json_decode` of truncated input crashed the VM.
- `json_decode` of surrogate pairs (escaped emoji such as `\ud83d\ude00`) produced `?`.

### Removed

- The `stdlib` (modules `ai`, `fs`, `http`, `json`, `math`, `net`, `stats`,
  `ws`) and its `import from "..."` names: their functions are core
  natives or package functions now.
- Natives `count` (use `len`), `json_get` and `json_has` (use
  `json_decode` + `map_has`), `vec_sum` (use `sum`), `is_better` (use
  `compare_groups(a, b, true)`).
- The Android APK build.

### Migrating from 1.0.0

- Delete `import from "http"`, `"json"`, `"fs"`, `"net"`, `"ws"` and
  `"math"`: those functions need no import now.
- `import from "ai"` becomes `import from "learn"` (plus `"num"` for vector
  and matrix functions); `liphia install learn`.
- Descriptive statistics (`median`, `stdev`, `percentile`, correlation)
  moved from `stats` to `num`.
- Add `to_float()` / `to_int()` where `min`, `max` or `sum` mixed int and
  float.
- Reinstall packages with the 2.0.0 `liphia` (`liphia install`); libraries
  built for 1.0.0 are refused by the ABI check.

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