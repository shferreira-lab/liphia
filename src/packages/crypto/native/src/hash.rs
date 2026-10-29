// packages/crypto/native/src/hash.rs
//
// Hash functions: SHA-1 (legacy), SHA-256, SHA-512, SHA3-256 and BLAKE3.
//
// Functions registered. Every digest is a lowercase hex string; the text
// form hashes the UTF-8 bytes of a str, the _hex form hashes the bytes that
// a hex string encodes (upper or lower case, even length, no separators).
//
//   crypto_sha1(text)     crypto_sha1_hex(hex)      -> str   40 hex chars
//   crypto_sha256(text)   crypto_sha256_hex(hex)    -> str   64 hex chars
//   crypto_sha512(text)   crypto_sha512_hex(hex)    -> str  128 hex chars
//   crypto_sha3_256(text) crypto_sha3_256_hex(hex)  -> str   64 hex chars
//   crypto_blake3(text)   crypto_blake3_hex(hex)    -> str   64 hex chars
//
// SHA-1 is broken for collision resistance. It stays for protocols that
// require it (WebSocket handshake, legacy MySQL auth, git); never use it to
// sign or to prove integrity. None of these functions is a password hash:
// use a slow, salted one (crypto:pwhash, planned) for passwords.

use liphia_virtual_machine::value::Value;
use liphia_virtual_machine::vm::{VmError, VmResult, VM};
use sha2::Digest;

use crate::hex;
use crate::util::{expect_args, str_ref, str_value};

pub fn register(vm: &mut VM) {
    vm.register_native("crypto_sha1",         native_sha1);
    vm.register_native("crypto_sha1_hex",     native_sha1_hex);
    vm.register_native("crypto_sha256",       native_sha256);
    vm.register_native("crypto_sha256_hex",   native_sha256_hex);
    vm.register_native("crypto_sha512",       native_sha512);
    vm.register_native("crypto_sha512_hex",   native_sha512_hex);
    vm.register_native("crypto_sha3_256",     native_sha3_256);
    vm.register_native("crypto_sha3_256_hex", native_sha3_256_hex);
    vm.register_native("crypto_blake3",       native_blake3);
    vm.register_native("crypto_blake3_hex",   native_blake3_hex);
}

// ── Digests ───────────────────────────────────────────────────────────────────

// RustCrypto hashes share one trait, so one generic function covers them.
fn digest_hex<D: Digest>(data: &[u8]) -> String {
    hex::encode(D::digest(data).as_slice())
}

fn blake3_hex(data: &[u8]) -> String {
    blake3::hash(data).to_hex().to_string()
}

// ── Natives ───────────────────────────────────────────────────────────────────

// Defines the text and _hex natives of one algorithm. The natives are plain
// fn pointers (NativeFn), so each one needs its own function item.
macro_rules! hash_natives {
    ($text_fn:ident, $hex_fn:ident, $text_name:literal, $hex_name:literal, $digest:expr) => {
        fn $text_fn(args: Vec<Value>) -> VmResult<Value> {
            expect_args($text_name, &args, 1)?;
            let text = str_ref(&args[0], $text_name)?;
            Ok(str_value($digest(text.as_bytes())))
        }

        fn $hex_fn(args: Vec<Value>) -> VmResult<Value> {
            expect_args($hex_name, &args, 1)?;
            let encoded = str_ref(&args[0], $hex_name)?;
            let bytes = hex::decode(encoded)
                .map_err(|e| VmError::new(format!("{}(): {}", $hex_name, e)))?;
            Ok(str_value($digest(&bytes)))
        }
    };
}

hash_natives!(native_sha1, native_sha1_hex, "crypto_sha1", "crypto_sha1_hex", digest_hex::<sha1::Sha1>);
hash_natives!(native_sha256, native_sha256_hex, "crypto_sha256", "crypto_sha256_hex", digest_hex::<sha2::Sha256>);
hash_natives!(native_sha512, native_sha512_hex, "crypto_sha512", "crypto_sha512_hex", digest_hex::<sha2::Sha512>);
hash_natives!(native_sha3_256, native_sha3_256_hex, "crypto_sha3_256", "crypto_sha3_256_hex", digest_hex::<sha3::Sha3_256>);
hash_natives!(native_blake3, native_blake3_hex, "crypto_blake3", "crypto_blake3_hex", blake3_hex);

#[cfg(test)]
mod tests {
    use super::*;

    // Published test vectors (FIPS 180-4, FIPS 202, RFC 6455, BLAKE3 repo).
    #[test]
    fn known_answers_for_abc() {
        assert_eq!(
            digest_hex::<sha1::Sha1>(b"abc"),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            digest_hex::<sha2::Sha256>(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            digest_hex::<sha3::Sha3_256>(b"abc"),
            "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532"
        );
    }

    #[test]
    fn known_answer_for_empty_blake3() {
        assert_eq!(
            blake3_hex(b""),
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );
    }
}
