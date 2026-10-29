# Changelog

All notable changes to the `crypto` package are listed here. Versions follow
[Semantic Versioning](https://semver.org); while the major version is 0, a
minor release may change or remove functions.

## 0.1.0 — unreleased

First version of the package: hashing.

### Added

- `crypto_sha256`, `crypto_sha512`, `crypto_sha3_256`, `crypto_blake3` and
  `crypto_sha1` (legacy), each returning a lowercase hex digest.
- A `_hex` variant of each (`crypto_sha256_hex`, ...) that hashes the bytes a
  hex string encodes, so digests can be chained.
- Strict hex input: upper or lower case, even length, no prefix or
  separators. Invalid input is a catchable runtime error that names the
  offending character and position.
- Conformance cases in `conformance/cases/crypto/`: NIST FIPS 180-4 and 202
  vectors (including one million `a`), the RFC 6455 handshake digest, the
  official BLAKE3 vectors, UTF-8 input and every error path.

### Notes

- Requires engine `2.1.0` exactly (native packages use the Rust ABI).
