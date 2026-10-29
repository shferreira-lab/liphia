# Liphia conformance suite

Every `cases/*.lph` is run through a Liphia VM and checked against a
sibling file with the same name:

- `<case>.out` — expected stdout, exact match, exit code 0
- `<case>.err` — the program must fail (non-zero exit) and its output
  must contain this text

Cases are VM-agnostic: any Liphia VM implementation must pass all of
them. Numbering: `0xx` language features, `1xx` expected errors. Semantics are described
in `docs/spec/VM.md`.

Run against the Rust VM:

    cargo test -p liphia_cli --test conformance

To add a case, drop a new `.lph` plus its `.out` or `.err` in `cases/`.
Every bug fix should come with a case that fails before the fix.

## Package suites

Each official package has its own folder, `cases/<package>/` (for example
`cases/crypto/`), with cases written exactly like the ones above. Package
natives only exist once the package is installed, so these folders are run
by a separate test, not by the language suite, which stays VM-agnostic:

    cargo test -p liphia_cli --test packages

For every folder the test builds the package's native library, installs it
into a scratch directory the way `liphia install` does, and runs each case
from there. The numbering is the same: `0xx` features, `1xx` expected errors.
Expected values should come from published test vectors or from an
independent implementation, never from the package's own output.
