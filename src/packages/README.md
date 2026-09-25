# Liphia packages

Everything Liphia offers outside the core natives (see `docs/core.md`)
is a package. Packages live here, in the same repository and Cargo
workspace as the engine, and `liphia install` downloads them from this
folder into a project's `liphia_modules/`.

| Package | Kind | Depends on | Contents |
|---------|------|------------|----------|
| `num` | native | — | vectors, matrices, descriptive statistics, correlation |
| `stats` | native | num | hypothesis tests, p-values, `compare_groups` |
| `learn` | native | num | activations, losses, optimizers, classification metrics |
| `db` | native | — | SQLite (embedded) and PostgreSQL (TCP wire protocol) |
| `wire` | pure | — | JSON response helpers over the core http natives |

```bash
liphia install num
liphia install stats        # also installs num
liphia install db:postgres  # db plus the postgres helper subpackage
```

```lph
import from "stats"
```

## Layout

```
packages/<name>/
  <name>.lph      entry file: documentation + imports of composed files
  module.toml     metadata, file list, [dependencies], [external.libs]
  composed/       functions written in Liphia on top of the natives
  index.lph       native packages only: load manifest for the VM
  lib/            native packages only: prebuilt libraries (committed)
  native/         native packages only: Rust source (never downloaded)
```

A pure package is only `.lph` files. A native package also has a cdylib
built from `native/`, loaded by `liphia_virtual_machine::external` when the
package is installed.

## Building native packages

The libraries use the Rust ABI, so they must be built with the same rustc
and the same `liphia_virtual_machine` as the `liphia` binary. After any
change to the VM or to a package, rebuild and commit the libraries:

```powershell
powershell -ExecutionPolicy Bypass -File src\packages\build.ps1
```

```bash
sh src/packages/build.sh
```

Both build every native package in release mode and copy the library into
`packages/<name>/lib/`.

## Known limits

- Natives are global: once a package is loaded its functions are callable
  even without `import`. Import it anyway; a later version will enforce it.
- The VM currently loads every installed native package at startup, not
  only the imported ones.
- Package signatures are not known to the type checker: calls to package
  natives type-check as `unknown` (accepted anywhere) and are validated
  by the native at runtime.
