# Liphia (.lph)

> A statically typed, indentation-based programming language powered by a Rust bytecode VM.
> Created by Sergio H. Ferreira — started in late 2025.

[![Engine](https://img.shields.io/badge/engine-1.0.0-blueviolet)](https://github.com/shferreira-lab/liphia)
[![Stdlib](https://img.shields.io/badge/stdlib-1.0.0-blueviolet)](https://github.com/shferreira-lab/liphia)
[![Language](https://img.shields.io/badge/rust-core%20engine-orange)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/status-1.0.0%20release-brightgreen)](https://github.com/shferreira-lab)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](./licenses)

Liphia is a general-purpose programming language with indentation-based syntax
(similar to Python), explicit static typing, and a bytecode compiler and virtual
machine implemented entirely in Rust.

Liphia ships with its own compiler, bytecode format, and runtime VM — no external
runtime, no interpreter dependency, no garbage collector overhead. Compiled
programs are fully self-contained.

Two ways to run a Liphia program:

- **`liphia_cli`** — the standard interpreter/compiler. Runs any `.lph` file,
  provides the package manager (`init`/`install`) and an interactive REPL.
- **`liphia_cli_gui`** — an optional, separately built executable for programs
  that open a native window (desktop apps, simple games), built on `egui`.
  It never ships as part of the standard `liphia_cli` binary.

This is the **1.0.0 release** — the first stable, functionally complete version
of the engine and standard library. See [Versions](#versions) below.

---

## Table of Contents

- [Repository layout](#repository-layout)
- [How to build](#how-to-build)
- [Where the binaries live](#where-the-binaries-live)
- [How to run a program](#how-to-run-a-program)
- [Starting a new project](#starting-a-new-project)
- [Installing stdlib modules](#installing-stdlib-modules)
- [Language reference and stdlib docs](#language-reference-and-stdlib-docs)
- [Versions](#versions)
- [Changelog](#changelog)
- [License](#license)

---

## Repository layout

```
liphia/
│
├── docs/                          # documentation
├── licenses/                      # licenses
│
├── src/
│   ├── liphia_engine/
│   │   └── crates/
│   │       ├── liphia_cli/                 # CLI runner + REPL + package manager
│   │       ├── liphia_cli_gui/             # optional windowed runtime (egui)
│   │       ├── liphia_compiler/            # lexer, parser, AST, bytecode compiler
│   │       ├── liphia_core_native/         # core string, list, map and value utilities
│   │       ├── liphia_gui_native/          # GUI native functions (toolkit-agnostic)
│   │       ├── liphia_pipeline/            # shared import-resolution + compile pipeline
│   │       └── liphia_virtual_machine/     # bytecode VM
│   │
│   ├── stdlib/
│   │   ├── modules/               # .lph modules (import from "...")
│   │   └── native/                # Rust native stdlib bindings
│   │
│   ├── tests/                     # tests and benchmarks
│   └── tools/
│       └── liphia-vscode/         # VS Code extension
│
└── README.md
```

---

## How to build

**Requirement:** [Rust](https://rustup.rs/) installed.

The workspace root is `src/`. All build commands must be run from there.

**Standard CLI** (everyone needs this one):

```bash
cd src
cargo build --release -p liphia_cli
```

**GUI runtime** (only if you're building windowed apps/games — separate,
optional binary, never required for normal `.lph` programs):

```bash
cargo build --release -p liphia_cli_gui
```

A plain `cargo build --release` (no `-p`) only builds the standard engine
crates — `liphia_cli_gui` is intentionally excluded from the default build,
so it never affects the size or dependencies of the main binary.

---

## Where the binaries live

After building, the executables are produced at:

```
src/target/release/liphia_cli        # Linux / macOS
src/target/release/liphia_cli.exe    # Windows
src/target/release/liphia_cli_gui        # Linux / macOS (if built)
src/target/release/liphia_cli_gui.exe    # Windows (if built)
```

`target/release/` is a build artifact folder, not meant to be your permanent
install location. Copy the executable(s) somewhere central and stable instead:

**Windows:**

```
C:\liphia\liphia_cli.exe
C:\liphia\liphia_cli_gui.exe
```

**Linux / macOS:**

```bash
sudo cp target/release/liphia_cli /usr/local/bin/liphia
sudo cp target/release/liphia_cli_gui /usr/local/bin/liphia_gui   # if built
```

Adding that folder to your system `PATH` lets you call `liphia`/`liphia_cli`
from any terminal, in any folder, without typing the full path each time.

---

## How to run a program

Whether or not the folder is in your `PATH`, the pattern is the same — call
the executable with the `.lph` file as the argument:

```bash
# with PATH configured
liphia my_program.lph

# calling the executable directly (Windows PowerShell example)
& "C:\liphia\liphia_cli.exe" my_program.lph

# GUI programs, same pattern, different executable
& "C:\liphia\liphia_cli_gui.exe" my_window_app.lph
```

**Bytecode cache** — the CLI automatically caches compiled bytecode next to
your source file, at `liphia_cache/<name>.lbc`. Subsequent runs are instant
unless the source changed. Force a fresh compile with `--no-cache`:

```bash
liphia my_program.lph --no-cache
```

**REPL** — `liphia` with no arguments, or `liphia --repl`, opens the
interactive shell. Multi-line input (functions, `if`, `while`, `try`, ...)
accumulates until you type `run` on its own line. *(The REPL is still
early/developer-facing at this point — see [Versions](#versions).)*

---

## Starting a new project

```bash
# 1. from an empty folder, create liphia.toml
& "C:\liphia\liphia_cli.exe" init

# 2. install whichever stdlib modules you need
& "C:\liphia\liphia_cli.exe" install http json

# 3. run your entry-point file
& "C:\liphia\liphia_cli.exe" main.lph
```

`liphia install` with no module name installs everything already listed in
your `liphia.toml`. `liphia install --list` shows every available stdlib
module and whether it's already installed.

---

## Installing stdlib modules

```bash
liphia install math
liphia install stats fs json
```

Modules are downloaded into `liphia_modules/` in your project folder. Once
installed, import them at the top of a `.lph` file:

```lph
import from "math"
import from "json"
```

See the [standard library README](./docs/stdlib/README.md) for the full
module list, versions, and what each one provides.

---

## Language reference and stdlib docs

This README intentionally stays focused on **building, running, and starting
a project**. Full language syntax (types, control flow, functions, maps,
async/await, error handling, imports) lives in
[`docs/language/README.md`](./docs/language/README.md); the complete
standard library API lives in [`docs/stdlib/`](./docs/stdlib/README.md).

---

## Versions

This is the project's first stable release — everything below started at
**1.0.0** together. From here on, each crate/module version moves
independently as it's fixed or extended.

**Engine (Rust crates):**

| Crate                     | Version |
|----------------------------|---------|
| `liphia_cli`               | 1.0.0   |
| `liphia_compiler`          | 1.0.0   |
| `liphia_virtual_machine`   | 1.0.0   |
| `liphia_core_native`       | 1.0.0   |
| `liphia_pipeline`          | 1.0.0   |
| `liphia_gui_native`        | 1.0.0   |
| `liphia_cli_gui`           | 1.0.0   |

**Standard library:** `stdlib` 1.0.0 overall — see the
[stdlib README](./docs/stdlib/README.md) for per-module versions
(`ai`, `db`, `fs`, `http`, `json`, `math`, `net`, `stats`, `ws`).

**Tooling:**

| Component                  | Version | Notes |
|------------------------------|---------|-------|
| VS Code extension          | 1.0.0   | Syntax highlighting, snippets |
| REPL                       | 0.4.0   | Still developer-facing / decorative — will move to 1.0.0 once it gets real, tested workflows |

---

## Changelog

See [`CHANGELOG.md`](./CHANGELOG.md) for what changed in each release,
starting with 1.0.0.

---

## License

Licensed under either of:

- [MIT License](./licenses/LICENSE-MIT)
- [Apache License, Version 2.0](./licenses/LICENSE-APACHE)

at your option.

---

<p align="center">
  Built with ❤️ &nbsp;·&nbsp; Engine written in Rust &nbsp;·&nbsp;
  <a href="https://github.com/shferreira-lab">shferreira-lab</a>
</p>