//! Byte comparison helper for score-to-audio verification.

pub fn first_difference(expected: &[u8], actual: &[u8]) -> Option<usize> {
    let shared = expected.len().min(actual.len());
    for index in 0..shared {
        if expected[index] != actual[index] {
            return Some(index);
        }
    }
    if expected.len() == actual.len() {
        None
    } else {
        Some(shared)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_buffers_have_no_difference() {
        assert_eq!(first_difference(b"coda", b"coda"), None);
        assert_eq!(first_difference(b"", b""), None);
    }

    #[test]
    fn locates_a_single_changed_byte() {
        assert_eq!(first_difference(b"coda", b"codb"), Some(3));
        assert_eq!(first_difference(b"coda", b"xoda"), Some(0));
    }

    #[test]
    fn locates_a_length_difference() {
        assert_eq!(first_difference(b"coda", b"cod"), Some(3));
        assert_eq!(first_difference(b"cod", b"coda"), Some(3));
    }
}
