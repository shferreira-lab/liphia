# Liphia Language Support for VS Code

> Syntax highlighting and snippets for the Liphia programming language (`.lph`).

[![Extension](https://img.shields.io/badge/extension-1.0.0-blueviolet)](https://github.com/shferreira-lab/liphia)
[![Engine](https://img.shields.io/badge/engine-1.0.0-blueviolet)](https://github.com/shferreira-lab/liphia)
[![Language](https://img.shields.io/badge/rust-core%20engine-orange)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/status-active%20development-yellow)](https://github.com/shferreira-lab)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](./licenses)

---

## Features

- Syntax highlighting for all Liphia keywords, types, operators, and literals
- Highlighting for all stdlib modules: `ai`, `math`, `stats`, `fs`, `http`, `ws`, `net`, `json`, `db`
- Interpolated string highlighting (`f"{expr}"`)
- Code snippets for common patterns
- Auto-indent after `:` (blocks, `if`/`for`/`fn`/`try`/`catch`)
- Comment toggling with `#`

For the language reference, syntax guide, and standard library documentation,
see the [main repository](https://github.com/shferreira-lab/liphia).

---

## Snippets

| Prefix              | Description                                    |
|----------------------|--------------------------------------------------|
| `print`             | `print(value)`                                 |
| `var`               | Variable (type inferred)                       |
| `vart`              | Variable (typed)                               |
| `const`             | Constant declaration                           |
| `int` / `float` / `str` / `bool` | Typed scalar variable            |
| `list`              | List variable                                  |
| `map`               | Map (dictionary) variable                      |
| `mapkeys` / `mapvalues` / `maphas` / `mapremove` | Map helper calls  |
| `if` / `ifelse` / `ifelifelse` | Conditionals                         |
| `while` / `for` / `forstep`    | Loops                                 |
| `try`               | `try` / `catch` block                          |
| `fn` / `fnr`        | Function declaration (with/without return)     |
| `async` / `asyncawait` / `spawn` | Async function, await, spawn        |
| `enum`              | Enum declaration                               |
| `fstr`              | F-string with interpolation                    |
| `import`            | Import local file (unqualified)                |
| `importsel`         | Selective import `{ name }`                    |
| `importqual`        | Qualified import (alias)                       |
| `importfrom`        | Import stdlib module                           |
| `importai` / `importmath` / `importhttp` / `importdb` / `importjson` | Import a specific stdlib module |
| `importdbsqlite` / `importdbpostgres` | Import a `db` submodule (requires `liphia install db:sqlite` / `db:postgres`) |
| `httpserver`        | HTTP server boilerplate                        |
| `sqlite`            | SQLite open, create table, query               |
| `dbtry`             | DB calls wrapped in `try`/`catch`               |
| `jsondecode`        | Decode JSON into a `map`                       |
| `nnforward`         | Neural network forward pass                    |
| `sgd` / `adam`      | Optimizer update                               |
| `metrics`           | Print classification metrics                   |
| `div`               | Section divider comment                        |

---

## How to run Liphia

**Requirement:** [Rust](https://rustup.rs/) installed.

```bash
cd src
cargo build --release -p liphia_cli
```

Then run any `.lph` file:

```bash
liphia path/to/program.lph
```

See the [main repository](https://github.com/shferreira-lab/liphia) for the
full language reference, standard library documentation, and package manager
usage.

---

## Changelog

### 0.1.0
Substantial update following Engine 0.10.0. Too many changes to list as a
patch — see the [main repository's CHANGELOG](https://github.com/shferreira-lab/liphia)
for full details on the language and stdlib side.

- Highlighting for `map` type and `map_*` functions
- Highlighting for `try` / `catch`
- Interpolated f-string highlighting (`f"{expr}"`)
- Highlighting for qualified module calls (`alias.function()`)
- Highlighting for additional `math`/`stats`/`db` functions added since 0.0.2
- New snippets: map declaration and helpers, try/catch, f-string, selective
  and qualified imports, db submodule imports, DB calls wrapped in try/catch,
  JSON decode to map
- Auto-indent rules after `:` for blocks

### 0.0.2
- Added keywords: `var`, `const`, `enum`, `async`, `await`, `spawn`
- Added types: `list`, `null`
- Added all stdlib functions as highlighted builtins (grouped by module)
- Added escape sequence highlighting inside strings
- Operators split into categories: comparison, arithmetic, assignment, logical
- New snippets: async fn, HTTP server boilerplate, SQLite, neural network
  forward pass, Adam/SGD optimizers, classification metrics

### 0.0.1
- Initial release: basic syntax highlighting and snippets

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