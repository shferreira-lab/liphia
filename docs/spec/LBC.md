# LBC — Liphia Bytecode Format

Version: **4** · Reference implementation: `liphia_engine/crates/liphia_bytecode`

LBC is the contract between the Liphia compiler and any Liphia VM. A VM
does not need the compiler: it only needs to read this format and
execute the opcodes with the semantics described in the VM spec.

Any change to the layout, to a tag value, or to an operand encoding
requires bumping `FORMAT_VERSION`. A reader must reject versions it does
not know (`UnsupportedVersion`), never guess.

## Encoding rules

- All integers are **little-endian**.
- `u32` operands (addresses, counts) are unsigned 32-bit.
- `str` = `u32` byte length followed by that many bytes of **UTF-8**.
- A reader must bounds-check every read and fail on truncated input,
  unknown tags, invalid UTF-8, or bytes left after the last opcode.

## Header (20 bytes)

| Offset | Size | Field            | Notes                                   |
|-------:|-----:|------------------|-----------------------------------------|
| 0      | 4    | `magic`          | `4C 42 43 00` (`"LBC\0"`)               |
| 4      | 4    | `format_version` | `u32`, currently `4`                    |
| 8      | 8    | `source_hash`    | `u64`, FNV-1a of the resolved AST       |
| 16     | 4    | `op_count`       | `u32`, number of opcodes that follow    |

`source_hash` is only used by the CLI cache to detect stale files. A VM
executing a program may ignore it.

## Opcodes

Each opcode is one tag byte followed by its operands. Addresses are
opcode indices (not byte offsets), starting at 0.

| Tag  | Opcode         | Operands                 |
|------|----------------|--------------------------|
| 0x01 | PushInt        | `i64`                    |
| 0x02 | PushFloat      | `f64` (IEEE 754 bits)    |
| 0x03 | PushString     | `str`                    |
| 0x04 | PushBool       | `u8` (0 = false)         |
| 0x05 | PushNull       | —                        |
| 0x06 | PushEnum       | `str` enum, `str` variant|
| 0x10 | LoadVar        | `u16` local slot         |
| 0x11 | StoreVar       | `u16` local slot         |
| 0x12 | LoadGlobal     | `str` name               |
| 0x13 | StoreGlobal    | `str` name               |
| 0x20 | Add            | —                        |
| 0x21 | Sub            | —                        |
| 0x22 | Mul            | —                        |
| 0x23 | Div            | —                        |
| 0x24 | Eq             | —                        |
| 0x25 | Neq            | —                        |
| 0x26 | Gt             | —                        |
| 0x27 | Lt             | —                        |
| 0x28 | Gte            | —                        |
| 0x29 | Lte            | —                        |
| 0x2A | And            | —                        |
| 0x2B | Or             | —                        |
| 0x2C | Not            | —                        |
| 0x30 | Input          | —                        |
| 0x31 | Print          | `u32` arg count          |
| 0x40 | Jump           | `u32` address            |
| 0x41 | JumpIfFalse    | `u32` address            |
| 0x50 | CallNamed      | `str` name, `u32` argc   |
| 0x51 | Call           | `u32` address, `u32` argc|
| 0x52 | Return         | —                        |
| 0x60 | BuildList      | `u32` count              |
| 0x61 | GetIndex       | —                        |
| 0x62 | SetIndex       | —                        |
| 0x63 | Pop            | —                        |
| 0x64 | BuildMap       | `u32` pair count         |
| 0x70 | Suspend        | —                        |
| 0x71 | Spawn          | `u32` address, `u32` argc|
| 0x80 | PushHandler    | `u32` catch address      |
| 0x81 | PopHandler     | —                        |
| 0xFF | Halt           | —                        |

## Planned for version 5

Not implemented yet; listed so VM authors know where the format is going:

- Constant pool for strings, so `PushString`, `LoadGlobal`, `StoreGlobal`
  and `CallNamed` carry a `u32` index instead of an inline string.
- Globals resolved to slot indices at compile time.
- Native functions resolved to indices at load time instead of by name
  on every call.
