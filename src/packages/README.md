# Liphia packages

Everything Liphia offers outside the core natives (see `docs/spec/core.md`)
is a package. Packages live here, in the same repository and Cargo
workspace as the engine.

| Package | Kind | Depends on | Contents |
|---------|------|------------|----------|
| `num` | native | — | vectors, matrices, descriptive statistics, correlation |
| `stats` | native | num | hypothesis tests, p-values, `compare_groups` |
| `learn` | native | num | activations, losses, optimizers, classification metrics |
| `db` | native | — | SQLite (embedded) and PostgreSQL (TCP wire protocol) |
| `wire` | pure | — | JSON response helpers over the core http natives |

```bash
liphia install stats          # latest version, saved as "^1.0.0"; installs num too
liphia install num@1.2.9      # exactly 1.2.9
liphia install db:sqlite      # db plus its sqlite subpackage
liphia update                 # newest versions allowed by liphia.toml
liphia install --frozen       # exactly liphia.lock (CI)
```

## Layout

```
packages/
  index.toml        published versions of every package
  <name>/
    package.toml    version, engine requirement, files, dependencies, [native]
    <name>.lph      entry file: documentation + imports of composed files
    composed/       functions written in Liphia on top of the natives
    index.lph       native packages only: load manifest read by the VM
    native/         native packages only: Rust source of the library
    lib/            local builds only (ignored by git)
```

## Versions and releases

Each package has its own version, independent of the engine. A release is a
git tag `<name>-v<version>`; the `release-package` workflow builds the native
library for Windows, Linux and macOS and attaches it to the GitHub Release.
`liphia install` reads the package files at that tag and downloads the
library from that release, so every published version stays installable.

To publish a version:

1. bump `version` in `<name>/package.toml`
2. add the version to `index.toml` and commit
3. `git tag <name>-v<version>` and `git push origin <name>-v<version>`

## Engine compatibility

`engine` in `package.toml` is checked by `liphia install`. Pure packages
accept a range (`"^2.0.0"`). Native packages pin one engine exactly
(`"=2.0.0"`): their library uses the Rust ABI, and the VM refuses a library
whose ABI tag (engine version + rustc) differs from its own. Every engine
release therefore needs a rebuild of the native packages, published as a
new patch version. A C ABI (`c-1`) will remove this constraint.

## Testing packages locally

Build the native libraries into `packages/<name>/lib/` and point the
installer at this folder instead of GitHub:

```powershell
powershell -ExecutionPolicy Bypass -File src\packages\build.ps1
$env:LIPHIA_REGISTRY = "C:\Dev\liphia\src\packages"
liphia install stats
```

```bash
sh src/packages/build.sh
LIPHIA_REGISTRY=$PWD/src/packages liphia install stats
```

## Known limits

- Natives are global: once a package is loaded its functions are callable
  even without `import`. Import it anyway; a later version will enforce it.
- The VM loads every installed native package at startup, not only the
  imported ones.
- Package signatures are not known to the type checker: calls to package
  natives type-check as `unknown` and are validated by the native at runtime.
