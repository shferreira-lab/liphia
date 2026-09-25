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
