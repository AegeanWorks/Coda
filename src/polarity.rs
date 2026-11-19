//! Polarity inversion for 16-bit stereo Coda WAVs.

fn validate(wav: &[u8]) -> Result<(), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be polarity-inverted".into());
    }
    let length = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]) as usize;
    if wav.len() != 44 + length || !length.is_multiple_of(4) {
        return Err("WAV data length does not match the file size".into());
    }
    Ok(())
}

fn sample(wav: &[u8], index: usize) -> (i16, i16) {
    (
        i16::from_le_bytes([wav[index], wav[index + 1]]),
        i16::from_le_bytes([wav[index + 2], wav[index + 3]]),
    )
}

/// Negate each sample in place in the i16 domain. Wrapping at `i16::MIN`
/// makes the transform an exact involution for every possible sample; the
/// header is copied unchanged.
pub fn invert_polarity(wav: &[u8]) -> Result<Vec<u8>, String> {
    validate(wav)?;
    let mut out = wav.to_vec();
    for index in (44..out.len()).step_by(4) {
        let (left, right) = sample(&out, index);
        out[index..index + 2].copy_from_slice(&left.wrapping_neg().to_le_bytes());
        out[index + 2..index + 4].copy_from_slice(&right.wrapping_neg().to_le_bytes());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn sample_wav() -> Vec<u8> {
        encode(&[(0.5, -0.25), (-0.8, 0.3), (0.0, 1.0)], 8_000)
    }

    #[test]
    fn flips_each_sample_sign() {
        let out = invert_polarity(&sample_wav()).unwrap();
        let before = sample(&sample_wav(), 44);
        let after = sample(&out, 44);
        assert_eq!(after.0, -before.0);
        assert_eq!(after.1, -before.1);
        let second = sample(&sample_wav(), 52);
        assert_eq!(sample(&out, 52), (-second.0, -second.1));
    }

    #[test]
    fn inverting_twice_is_byte_identical() {
        let once = invert_polarity(&sample_wav()).unwrap();
        assert_eq!(invert_polarity(&once).unwrap(), sample_wav());
    }

    #[test]
    fn wrapping_keeps_the_i16_extreme_an_involution() {
        let mut raw = encode(&[(0.5, 0.5)], 8_000);
        raw[44..46].copy_from_slice(&i16::MIN.to_le_bytes());
        let once = invert_polarity(&raw).unwrap();
        assert_eq!(sample(&once, 44).0, i16::MIN, "wraps at the extreme");
        assert_eq!(invert_polarity(&once).unwrap(), raw);
    }

    #[test]
    fn rejects_bad_input() {
        assert!(invert_polarity(b"bad").is_err());
    }
}
