//! Deterministic crossfade of two 16-bit stereo Coda WAVs.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be crossfaded".into());
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

/// Overlap the tail of `a` with the head of `b` for `duration` seconds.
/// Zero duration concatenates; longer overlaps are capped to the shorter input.
pub fn crossfade_wavs(a: &[u8], b: &[u8], duration: f64) -> Result<Vec<u8>, String> {
    if !duration.is_finite() || duration < 0.0 {
        return Err("duration must be a non-negative number".into());
    }
    let (rate_a, left) = decode(a)?;
    let (rate_b, right) = decode(b)?;
    if rate_a != rate_b {
        return Err(format!("sample-rate mismatch: {rate_a} Hz vs {rate_b} Hz"));
    }
    let overlap = ((duration * rate_a as f64).round() as usize)
        .min(left.len())
        .min(right.len());
    let mut frames = Vec::with_capacity(left.len() + right.len() - overlap);
    frames.extend_from_slice(&left[..left.len() - overlap]);
    for index in 0..overlap {
        let weight = index as f64 / overlap as f64;
        let x = left[left.len() - overlap + index];
        let y = right[index];
        frames.push((
            (x.0 * (1.0 - weight) + y.0 * weight).clamp(-1.0, 1.0),
            (x.1 * (1.0 - weight) + y.1 * weight).clamp(-1.0, 1.0),
        ));
    }
    frames.extend_from_slice(&right[overlap..]);
    Ok(wav_bytes(&frames, rate_a))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    #[test]
    fn zero_duration_concatenates() {
        let a = encode(&[(0.2, 0.2)], 8_000);
        let b = encode(&[(0.8, 0.8)], 8_000);
        let (_, frames) = decode(&crossfade_wavs(&a, &b, 0.0).unwrap()).unwrap();
        assert_eq!(frames.len(), 2);
        assert!((frames[0].0 - 0.2).abs() < 1e-4);
        assert!((frames[1].0 - 0.8).abs() < 1e-4);
    }

    #[test]
    fn overlap_blends_with_a_linear_weight() {
        let a = encode(&[(0.0, 0.0); 2], 8_000);
        let b = encode(&[(1.0, 1.0); 2], 8_000);
        let (_, frames) = decode(&crossfade_wavs(&a, &b, 0.00025).unwrap()).unwrap();
        assert_eq!(frames.len(), 2);
        assert!(frames[0].0.abs() < 1e-4, "{}", frames[0].0);
        assert!((frames[1].0 - 0.5).abs() < 1e-4, "{}", frames[1].0);
    }

    #[test]
    fn length_and_rate_are_preserved() {
        let a = encode(&[(0.1, 0.1); 4], 8_000);
        let b = encode(&[(0.9, 0.9); 4], 8_000);
        let (rate, frames) = decode(&crossfade_wavs(&a, &b, 0.00025).unwrap()).unwrap();
        assert_eq!(rate, 8_000);
        assert_eq!(frames.len(), 6, "4 + 4 - 2 overlap");
    }

    #[test]
    fn rejects_bad_inputs() {
        let a = encode(&[(0.5, 0.5)], 8_000);
        let other = encode(&[(0.5, 0.5)], 16_000);
        assert!(crossfade_wavs(&a, &other, 0.0).is_err());
        assert!(crossfade_wavs(b"bad", &a, 0.0).is_err());
        assert!(crossfade_wavs(&a, &a, -1.0).is_err());
    }
}
