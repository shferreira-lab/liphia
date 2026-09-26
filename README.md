# Liphia (.lph)

> A statically typed, indentation-based programming language powered by a Rust bytecode VM.
> Created by Sergio H. Ferreira — started in late 2025.

[![Engine](https://img.shields.io/badge/engine-2.0.0-blueviolet)](https://github.com/shferreira-lab/liphia/releases)
[![Language](https://img.shields.io/badge/rust-core%20engine-orange)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/status-2.0.0%20release-brightgreen)](https://github.com/shferreira-lab/liphia/releases)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](./licenses)

Liphia is a general-purpose programming language with indentation-based syntax
(similar to Python), explicit static typing, and a bytecode compiler and virtual
machine implemented entirely in Rust.

Liphia ships as a single native executable that contains the compiler, the
bytecode VM, the REPL and the package manager — no external runtime to
install. Memory is managed by reference counting, so there are no garbage
collector pauses (reference cycles are not collected; see
[`docs/spec/VM.md`](./docs/spec/VM.md)).

Two executables:

- **`liphia`** — runs `.lph` programs, manages packages (`init`, `install`,
  `update`...) and provides an interactive REPL.
- **`liphia-gui`** — an optional runtime for programs that open a native
  window (desktop apps, simple games), built on `egui`.

---

## Table of Contents

- [Install](#install)
- [Quick start](#quick-start)
- [Projects and packages](#projects-and-packages)
- [What comes with Liphia](#what-comes-with-liphia)
- [Documentation](#documentation)
- [Building from source](#building-from-source)
- [Repository layout](#repository-layout)
- [Versions](#versions)
- [License](#license)

---

## Install

The installers put `liphia` (and optionally `liphia-gui`) on your `PATH`, so
it works from any terminal or editor. Nothing needs to be compiled: the
engine is prebuilt, and packages are downloaded prebuilt for your platform
when you install them.

**Windows** — download `liphia-2.0.0-windows-x86_64-setup.exe` from the
[latest release](https://github.com/shferreira-lab/liphia/releases/latest)
and run it. It installs for your user without admin rights (or for all
users, if you choose), lets you include or skip the GUI runtime, and adds
Liphia to `PATH`.

Or from PowerShell:

```powershell
irm https://raw.githubusercontent.com/shferreira-lab/liphia/main/installer/windows/install.ps1 | iex
```

**Linux (x86_64) and macOS (Apple silicon):**

```bash
curl -fsSL https://raw.githubusercontent.com/shferreira-lab/liphia/main/installer/unix/install.sh | sh
```

This installs into `~/.liphia/bin` and adds it to `PATH` in your shell's
startup file. The Linux build includes the CLI only; `liphia-gui` is
available on Windows and macOS.

After installing, open a **new** terminal (editors such as VS Code must be
restarted too) and check:

```bash
liphia version
```

To install a specific version, set `LIPHIA_VERSION` (e.g. `2.0.0`) before
running either script. Every release also has plain `.zip` / `.tar.gz`
archives if you prefer to place the executables yourself.

---

## Quick start

```lph
# hello.lph
fn greet(name: str) -> str:
    return f"Hello, {name}!"

print(greet("Liphia"))
```

```bash
liphia hello.lph              # compile (cached) and run
liphia hello.lph --no-cache   # force a fresh compile
liphia                        # interactive REPL
liphia help                   # all commands
```

Compiled bytecode is cached next to the source in `liphia_cache/<name>.lbc`
and reused until the source changes. In the REPL, multi-line input (`fn`,
`if`, `while`, `try`...) accumulates until you type `run` on its own line.

The [`examples/`](./examples/README.md) folder has runnable programs for
every part of the language and the standard packages.

---

## Projects and packages

```bash
mkdir my_app && cd my_app
liphia init                     # creates liphia.toml
liphia install stats            # latest version, saved as "^1.0.0" (installs num too)
liphia install num@1.2.9        # an exact version
liphia install db:sqlite        # a package plus one of its subpackages
liphia update                   # newest versions allowed by liphia.toml
liphia remove stats
liphia list                     # published packages and installed versions
```

```lph
import from "stats"
print(compare_groups([5.1, 4.9, 5.3], [5.6, 5.4, 5.9], false))
```

`liphia install` writes **`liphia.lock`** with the exact version of every
package, including transitive ones. Commit it: `liphia install` on another
machine reproduces the same set, and `liphia install --frozen` (for CI)
installs exactly the lock or fails.

Version requirements follow npm: `"1.2.9"` is exact, `"^1.2.9"` accepts
compatible versions (same major), `"~1.2.9"` accepts patches only.

Packages are installed into `liphia_modules/` in the project and cached per
machine in `~/.liphia/cache`. Every published version stays available, so an
older project keeps installing the versions it locked.

---

## What comes with Liphia

**Core** — built into the executable, no import needed: strings, lists,
maps, conversions, scalar math, random numbers, aggregation (`sum`, `mean`,
`min_list`, `max_list`), JSON, files, TCP/UDP, an HTTP server and client,
and a WebSocket server. Full reference: [`docs/spec/core.md`](./docs/spec/core.md).

**Official packages** — installed per project with `liphia install`:

| Package | Contents |
|---------|----------|
| `num` | vectors, matrices, descriptive statistics, correlation |
| `stats` | hypothesis tests, p-values, `compare_groups` (depends on `num`) |
| `learn` | activations, losses, optimizers, classification metrics (depends on `num`) |
| `db` | SQLite (embedded) and PostgreSQL (`db:sqlite`, `db:postgres`) |
| `wire` | JSON response helpers for HTTP APIs |

Each package's entry file (`<name>.lph`) documents its functions; see
[`src/packages/README.md`](./src/packages/README.md) for how packages are
built, versioned and published.

---

## Documentation

| Document | Contents |
|----------|----------|
| [`docs/language/README.md`](./docs/language/README.md) | language reference: syntax, types, functions, async, imports |
| [`docs/spec/core.md`](./docs/spec/core.md) | every core native, with signatures and rules |
| [`docs/spec/VM.md`](./docs/spec/VM.md) | VM semantics |
| [`docs/spec/LBC.md`](./docs/spec/LBC.md) | bytecode file format |
| [`src/packages/README.md`](./src/packages/README.md) | packages: layout, versions, engine compatibility |
| [`examples/README.md`](./examples/README.md) | runnable examples |
| [`docs/language/CHANGELOG.md`](./docs/language/CHANGELOG.md) | release history |

---

## Building from source

Only needed to work on Liphia itself; the installers above already contain
everything.

**Requirement:** [rustup](https://rustup.rs/). The exact Rust version is
pinned in `src/rust-toolchain.toml` and installed automatically the first
time you build.

The Cargo workspace root is `src/`:

```bash
cd src
cargo build --release                      # liphia (target/release/liphia[.exe])
cargo build --release -p liphia_cli_gui    # liphia-gui, optional
cargo test -p liphia_cli --test conformance
```

The conformance suite (`src/conformance/cases/`) runs `.lph` programs and
compares their output with the expected result. It defines the language's
behavior: any implementation of the Liphia VM must pass it.

Native packages (`num`, `stats`, `learn`, `db`) are separate libraries.
Build them into `src/packages/<name>/lib/` and install from that folder
instead of GitHub:

```powershell
powershell -ExecutionPolicy Bypass -File src\packages\build.ps1   # Windows
```

```bash
sh src/packages/build.sh                                         # Linux / macOS
LIPHIA_REGISTRY=$PWD/src/packages liphia install stats
```

A native package library only loads in an engine built with the same Rust
version from the same engine version; rebuild the packages after changing
the VM or the toolchain.

---

## Repository layout

```
liphia/
├── .github/workflows/      CI and release automation (engine and packages)
├── docs/
│   ├── language/           language reference and changelog
│   └── spec/               core natives, VM semantics, bytecode format
├── examples/               runnable examples (core and packages)
├── installer/
│   ├── windows/            Inno Setup script and install.ps1
│   └── unix/               install.sh
├── licenses/               MIT, Apache-2.0
└── src/                    Cargo workspace
    ├── rust-toolchain.toml pinned Rust version
    ├── about.toml          third-party license report (cargo-about)
    ├── conformance/        language conformance suite
    ├── liphia_engine/crates/
    │   ├── liphia_bytecode/          .lbc encoder / decoder
    │   ├── liphia_cli/               the `liphia` executable: runner, REPL, package manager
    │   ├── liphia_cli_gui/           the `liphia-gui` executable (egui)
    │   ├── liphia_compiler/          lexer, parser, type checker, bytecode compiler
    │   ├── liphia_core_native/       core natives
    │   ├── liphia_gui_native/        GUI natives (toolkit-agnostic)
    │   ├── liphia_manifest/          liphia.toml, liphia.lock, package.toml formats
    │   ├── liphia_pipeline/          import resolution + compile pipeline
    │   └── liphia_virtual_machine/   bytecode VM and native package loader
    ├── packages/           official packages (num, stats, learn, db, wire)
    └── tools/liphia-vscode/ VS Code extension
```

---

## Versions

**Engine: 2.0.0.** Every engine crate shares this version; it is the
version `liphia version` reports and the one releases are tagged with
(`v2.0.0`).

**Packages** have their own versions, released independently with tags
`<name>-v<version>`:

| Package | Version |
|---------|---------|
| `num`   | 1.0.0 |
| `stats` | 1.0.0 |
| `learn` | 1.0.0 |
| `db`    | 2.0.0 |
| `wire`  | 1.0.0 |

**Tooling:**

| Component | Version | Notes |
|-----------|---------|-------|
| VS Code extension | 1.0.0 | syntax highlighting and snippets; an update for 2.0.0 is planned |
| REPL | — | part of `liphia`; still developer-facing |

All versions follow [Semantic Versioning](https://semver.org/). See the
[changelog](./docs/language/CHANGELOG.md) for what changed in each release.

---

## License

Licensed under either of:

- [MIT License](./licenses/LICENSE-MIT)
- [Apache License, Version 2.0](./licenses/LICENSE-APACHE)

at your option. Third-party components included in the release binaries are
listed with their licenses in `THIRD_PARTY_LICENSES.txt`, shipped with every
release.

---

<p align="center">
  Built with ❤️ &nbsp;·&nbsp; Engine written in Rust &nbsp;·&nbsp;
  <a href="https://github.com/shferreira-lab">shferreira-lab</a>
</p>
