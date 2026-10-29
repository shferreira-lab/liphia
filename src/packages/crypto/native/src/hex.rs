// packages/crypto/native/src/hex.rs
//
// Hex encoding for binary values. Output is always lowercase. Input accepts
// both cases but nothing else: no prefix, no separators, no whitespace.

const DIGITS: &[u8; 16] = b"0123456789abcdef";

pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(DIGITS[(b >> 4) as usize] as char);
        out.push(DIGITS[(b & 0x0f) as usize] as char);
    }
    out
}

// Strict decoder. The character check runs first, so "zz" is reported as an
// invalid character rather than as an odd length.
pub fn decode(text: &str) -> Result<Vec<u8>, String> {
    if let Some((index, c)) = text
        .chars()
        .enumerate()
        .find(|(_, c)| !c.is_ascii_hexdigit())
    {
        return Err(format!("invalid hex character {:?} at position {}", c, index));
    }
    if text.len() % 2 != 0 {
        return Err(format!("odd number of hex digits ({})", text.len()));
    }
    // Every character is an ASCII hex digit now, so bytes and characters
    // are the same thing.
    Ok(text
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| (nibble(pair[0]) << 4) | nibble(pair[1]))
        .collect())
}

// Value of one validated hex digit.
fn nibble(digit: u8) -> u8 {
    match digit {
        b'0'..=b'9' => digit - b'0',
        b'a'..=b'f' => digit - b'a' + 10,
        _ => digit - b'A' + 10,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_lowercase() {
        assert_eq!(encode(&[0x00, 0x0f, 0xab, 0xff]), "000fabff");
        assert_eq!(encode(&[]), "");
    }

    #[test]
    fn decodes_both_cases() {
        assert_eq!(decode("000fABff").unwrap(), vec![0x00, 0x0f, 0xab, 0xff]);
        assert_eq!(decode("").unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn round_trips_every_byte() {
        let all: Vec<u8> = (0..=255).collect();
        assert_eq!(decode(&encode(&all)).unwrap(), all);
    }

    #[test]
    fn rejects_odd_length() {
        assert_eq!(decode("abc").unwrap_err(), "odd number of hex digits (3)");
    }

    #[test]
    fn rejects_invalid_characters() {
        assert_eq!(
            decode("12zz").unwrap_err(),
            "invalid hex character 'z' at position 2"
        );
        assert!(decode("0x12").is_err());
        assert!(decode("12 34").is_err());
        assert!(decode("12\n").is_err());
        // The position counts characters, not bytes.
        assert_eq!(
            decode("aé").unwrap_err(),
            "invalid hex character 'é' at position 1"
        );
    }
}
