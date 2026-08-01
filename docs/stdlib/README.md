# Liphia Standard Library — 1.0.0

Official package registry for the [Liphia](https://github.com/shferreira-lab/liphia)
language.

All modules are **native** — implemented in Rust and compiled into every
`liphia_cli` / `liphia_cli_gui` binary. `liphia install` doesn't add new
functionality that wasn't already in your executable; it downloads each
module's `.lph` layer (documentation, plus any higher-level composed
functions built on top of the natives) into your project.

---

## Full function reference

For the complete, per-module function list — native vs. composed, with
signatures and descriptions — see [`REFERENCE.md`](./REFERENCE.md).

## Installing modules

```bash
liphia install http
liphia install ws math json
liphia install        # installs everything listed in liphia.toml
liphia install --list # shows every available module and install status
```

Once installed, import a module at the top of a `.lph` file:

```lph
import from "math"
import from "stats"
```

---

## Available modules

| Module  | Version | Description                        |
|---------|---------|-------------------------------------|
| `ai`    | 1.1.0   | AI / neural network primitives — activations, vectors, matrices, preprocessing, loss functions, optimizers, classification metrics, distance functions |
| `db`    | 1.1.0   | SQLite (embedded) and PostgreSQL (pure TCP wire protocol) |
| `fs`    | 1.1.0   | File system operations |
| `http`  | 1.2.0   | HTTP client and server, native CORS |
| `json`  | 1.2.0   | JSON encode/decode — objects decode to `map` |
| `math`  | 1.1.0   | Mathematical functions |
| `net`   | 1.1.0   | Low-level TCP/UDP networking |
| `stats` | 1.1.0   | Statistics and data analysis |
| `ws`    | 1.1.0   | WebSocket server |

---

## Module structure

Each module lives in `modules/<name>/` and contains:

- `<name>.lph` — the module's Liphia-facing entry point: documents the native
  functions, and imports any composed `.lph` helpers built on top of them
- `module.toml` — metadata (name, version, file list)
- `composed/` *(where present)* — higher-level functions written in pure
  Liphia over the module's own natives, sometimes combined with natives from
  other modules (e.g. `http`'s composed layer uses `json` natives directly).
  These only reach a project once the module is installed — the underlying
  natives they call are always in the binary regardless.

Some modules also have `submodules/` — optional, separately installable
extras for one specific backend (e.g. `db:sqlite`, `db:postgres`):

```bash
liphia install db:postgres
```

---

## Notes

- Module natives are always callable once the corresponding stdlib crate is
  compiled into your binary — `import from "<module>"` is primarily a
  documentation/organization convention rather than a hard runtime gate.
- `json` 2.0.0 (bundled in stdlib 1.0.0) is a breaking change from earlier
  versions: `json_decode` now returns a real `map`/`list` instead of a flat
  `["key", value, ...]` list.