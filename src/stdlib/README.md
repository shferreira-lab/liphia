# Liphia Standard Library

Official package registry for the [Liphia](https://github.com/liphia-lang) language.

## Installing packages
```bash
liphia install http
liphia install ws math json
liphia install        # installs all from liphia.toml
```

## Available modules

| Module  | Description                        |
|---------|------------------------------------|
| http    | HTTP client and server             |
| ws      | WebSocket server                   |
| fs      | File system operations             |
| math    | Mathematical functions             |
| json    | JSON encode/decode                 |
| ai      | AI / neural network primitives     |
| net     | Low-level TCP/UDP networking       |
| stats   | Statistics and data analysis       |

## Module structure

Each module lives in `modules/<name>/` and contains:
- `<name>.lph` — Liphia source (wrappers over native functions)
- `module.toml` — metadata