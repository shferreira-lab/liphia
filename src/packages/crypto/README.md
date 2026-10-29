# crypto

Cryptographic primitives for [Liphia](../../../README.md), implemented in Rust
on top of the [RustCrypto](https://github.com/RustCrypto) and
[BLAKE3](https://github.com/BLAKE3-team/BLAKE3) libraries. Native package,
licensed MIT OR Apache-2.0.

**Version 0.1.0 covers hashing only.** MACs, random bytes, password hashing
and authenticated encryption are planned and will arrive as new functions of
this same package.

```bash
liphia install crypto
```

```lph
import from "crypto"

print(crypto_sha256("abc"))
# ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
```

## Hashing

Every function returns the digest as a lowercase hex string.

| Text form | Hex form | Digest | Length | Notes |
|-----------|----------|--------|--------|-------|
| `crypto_sha256(text)` | `crypto_sha256_hex(hex)` | SHA-256 | 64 | the default choice |
| `crypto_sha512(text)` | `crypto_sha512_hex(hex)` | SHA-512 | 128 | |
| `crypto_sha3_256(text)` | `crypto_sha3_256_hex(hex)` | SHA3-256 | 64 | Keccak |
| `crypto_blake3(text)` | `crypto_blake3_hex(hex)` | BLAKE3 | 64 | fast, not a NIST standard |
| `crypto_sha1(text)` | `crypto_sha1_hex(hex)` | SHA-1 | 40 | **legacy**, see below |

The text form hashes the UTF-8 bytes of a `str`. The hex form hashes the
bytes that a hex string encodes, which is how digests are chained:

```lph
var inner: str = crypto_sha256("hello")
var outer: str = crypto_sha256_hex(inner)   # sha256(sha256("hello"))
```

## Binary values

The language has no bytes type yet, so every binary value is a lowercase hex
string. Hex input may be upper or lower case; it must have an even length and
no prefix (`0x`), spaces or separators. A function that receives anything
else raises a runtime error that `try`/`catch` can handle:

```lph
try:
    crypto_sha256_hex("abc")
catch e:
    print(e)   # crypto_sha256_hex(): odd number of hex digits (3)
```

A wrong number of arguments or a non-`str` argument is an error too, never a
silent conversion.

## What these functions are not

- **Not password hashes.** SHA-256 and the others are fast on purpose. Never
  store `crypto_sha256(password)`; use a slow, salted hash (argon2 or bcrypt,
  planned for this package).
- **SHA-1 is broken** for collision resistance. It is included only for
  protocols that require it (the WebSocket handshake, git, legacy MySQL
  authentication). Do not use it for signatures or integrity checks.
- **Not a random source.** `rand_int`, `shuffle` and the other core random
  functions are not cryptographically secure either. A secure random
  generator is planned for this package.

## Names

Package natives share one global namespace with the core and with each other,
so every function of this package starts with `crypto_`.

## Tests

The cases in [`conformance/cases/crypto/`](../../conformance/cases/crypto)
use published test vectors (NIST FIPS 180-4 and 202, RFC 6455, the BLAKE3
repository) and the error paths. Run them with:

```bash
cargo test -p liphia_cli --test packages
```

## License

MIT OR Apache-2.0, at your option. Dependencies are `sha1`, `sha2`, `sha3`
(MIT OR Apache-2.0) and `blake3` (CC0-1.0 OR Apache-2.0).
