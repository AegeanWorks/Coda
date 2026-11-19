//! Circular rotation of a Coda WAV's frames.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be rotated".into());
    }
    let length = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]) as usize;
    if wav.len() != 44 + length || !length.is_multiple_of(4) {
        return Err("WAV data length does not match the file size".into());
    }
    let rate = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]);
    let frames: Vec<Frame> = wav[44..]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| {
            (
                i16::from_le_bytes([c[0], c[1]]) as f64 / 32767.0,
                i16::from_le_bytes([c[2], c[3]]) as f64 / 32767.0,
            )
        })
        .collect();
    Ok((rate, frames))
}

/// Rotate frames left by a rounded offset, wrapping the head to the tail.
/// A zero offset is byte-identical; length and rate are preserved.
pub fn rotate(wav: &[u8], seconds: f64) -> Result<Vec<u8>, String> {
    if !seconds.is_finite() || seconds < 0.0 {
        return Err("offset must be a non-negative number".into());
    }
    let (rate, frames) = decode(wav)?;
    if frames.is_empty() {
        return Ok(wav_bytes(&frames, rate));
    }
    let amount = (seconds * rate as f64).round() as usize % frames.len();
    let mut out = Vec::with_capacity(frames.len());
    out.extend_from_slice(&frames[amount..]);
    out.extend_from_slice(&frames[..amount]);
    Ok(wav_bytes(&out, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn sample() -> Vec<u8> {
        encode(&[(0.1, 0.1), (0.2, 0.2), (0.3, 0.3), (0.4, 0.4)], 8_000)
    }

    #[test]
    fn zero_offset_is_an_identity() {
        assert_eq!(rotate(&sample(), 0.0).unwrap(), sample());
    }

    #[test]
    fn wraps_the_head_to_the_tail() {
        let out = rotate(&sample(), 0.00025).unwrap(); // 2 frames
        let (_, frames) = decode(&out).unwrap();
        let values: Vec<f64> = frames.iter().map(|f| f.0).collect();
        assert_eq!(values.len(), 4);
        assert!((values[0] - 0.3).abs() < 1e-4);
        assert!((values[2] - 0.1).abs() < 1e-4);
    }

    #[test]
    fn offset_wraps_past_the_length() {
        let full = rotate(&sample(), 0.0005).unwrap(); // 4 frames == length
        assert_eq!(full, sample());
        let again = rotate(&sample(), 0.000625).unwrap(); // 5 frames == 1
        assert_eq!(again, rotate(&sample(), 0.000125).unwrap());
    }

    #[test]
    fn rejects_bad_input() {
        assert!(rotate(b"bad", 0.0).is_err());
        assert!(rotate(&sample(), f64::NAN).is_err());
        assert!(rotate(&sample(), -0.000125).is_err());
    }
}
