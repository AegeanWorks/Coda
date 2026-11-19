//! FNV-1a 64-bit fingerprint over rendered bytes.
//!
//! Not cryptographic: it exists so a rendered piece can be named, compared
//! and checked for reproducibility with a short, stable string.

const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01b3;

pub fn fingerprint(bytes: &[u8]) -> String {
    let mut acc = OFFSET;
    for &b in bytes {
        acc ^= b as u64;
        acc = acc.wrapping_mul(PRIME);
    }
    format!("{acc:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vectors() {
        assert_eq!(fingerprint(b""), "cbf29ce484222325");
        assert_eq!(fingerprint(b"a"), "af63dc4c8601ec8c");
        assert_eq!(fingerprint(b"hello"), "a430d84680aabd0b");
    }

    #[test]
    fn sensitive_to_one_byte() {
        assert_ne!(fingerprint(b"coda"), fingerprint(b"codb"));
    }
}
