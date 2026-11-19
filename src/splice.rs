//! Insert one Coda WAV into another at an exact frame offset.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be spliced".into());
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

/// Insert `insert` into `base` at `at` seconds, shifting the remaining audio.
pub fn splice_wavs(base: &[u8], insert: &[u8], at: f64) -> Result<Vec<u8>, String> {
    if !at.is_finite() || at < 0.0 {
        return Err("at must be a non-negative number".into());
    }
    let (rate_a, a) = decode(base)?;
    let (rate_b, b) = decode(insert)?;
    if rate_a != rate_b {
        return Err(format!("sample-rate mismatch: {rate_a} Hz vs {rate_b} Hz"));
    }
    let offset = (at * rate_a as f64).round() as usize;
    let head = offset.min(a.len());
    let mut frames = Vec::with_capacity(offset.max(a.len()) + b.len());
    frames.extend_from_slice(&a[..head]);
    frames.resize(offset, (0.0_f64, 0.0_f64));
    frames.extend_from_slice(&b);
    if head < a.len() {
        frames.extend_from_slice(&a[head..]);
    }
    Ok(wav_bytes(&frames, rate_a))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn a() -> Vec<u8> {
        encode(&[(0.1, 0.1), (0.2, 0.2), (0.3, 0.3)], 8_000)
    }

    fn b() -> Vec<u8> {
        encode(&[(0.9, 0.9), (0.8, 0.8)], 8_000)
    }

    #[test]
    fn inserts_in_the_middle_and_keeps_the_tail() {
        let out = splice_wavs(&a(), &b(), 0.000125).unwrap(); // 1 frame at 8 kHz
        let (_, frames) = decode(&out).unwrap();
        let values: Vec<f64> = frames.iter().map(|f| f.0).collect();
        assert_eq!(values.len(), 5);
        assert!((values[0] - 0.1).abs() < 1e-4);
        assert!((values[1] - 0.9).abs() < 1e-4);
        assert!((values[2] - 0.8).abs() < 1e-4);
        assert!((values[3] - 0.2).abs() < 1e-4);
        assert!((values[4] - 0.3).abs() < 1e-4);
    }

    #[test]
    fn zero_offset_puts_the_insert_first() {
        let out = splice_wavs(&a(), &b(), 0.0).unwrap();
        let (_, frames) = decode(&out).unwrap();
        assert_eq!(frames.len(), 5);
        assert!((frames[0].0 - 0.9).abs() < 1e-4);
        assert!((frames[4].0 - 0.3).abs() < 1e-4);
    }

    #[test]
    fn beyond_the_end_zero_pads_the_gap() {
        let out = splice_wavs(&a(), &b(), 0.0005).unwrap(); // 4 frames
        let (_, frames) = decode(&out).unwrap();
        assert_eq!(frames.len(), 6);
        assert!(frames[3] == (0.0, 0.0));
        assert!((frames[4].0 - 0.9).abs() < 1e-4);
    }

    #[test]
    fn rejects_bad_inputs() {
        let other_rate = encode(&[(0.5, 0.5)], 16_000);
        assert!(splice_wavs(&a(), &other_rate, 0.0).is_err());
        assert!(splice_wavs(&a(), &b(), -1.0).is_err());
        assert!(splice_wavs(b"bad", &b(), 0.0).is_err());
    }
}
