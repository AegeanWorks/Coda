//! Replace a time region of one Coda WAV with another, keeping the base length.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be overwritten".into());
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

/// Replace frames of `base` from `at` seconds with `insert`, truncated to the base length.
pub fn overwrite(base: &[u8], insert: &[u8], at: f64) -> Result<Vec<u8>, String> {
    if !at.is_finite() || at < 0.0 {
        return Err("at must be a non-negative number".into());
    }
    let (base_rate, mut frames) = decode(base)?;
    let (insert_rate, incoming) = decode(insert)?;
    if base_rate != insert_rate {
        return Err(format!(
            "sample-rate mismatch: {base_rate} Hz vs {insert_rate} Hz"
        ));
    }
    let start = (at * base_rate as f64).round() as usize;
    for (index, frame) in incoming.iter().enumerate() {
        let target = start + index;
        if target >= frames.len() {
            break;
        }
        frames[target] = *frame;
    }
    Ok(wav_bytes(&frames, base_rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    #[test]
    fn replaces_a_region_in_place() {
        let base = encode(&[(0.1, 0.1), (0.2, 0.2), (0.3, 0.3), (0.4, 0.4)], 8_000);
        let insert = encode(&[(0.9, 0.9), (0.8, 0.8)], 8_000);
        let out = overwrite(&base, &insert, 0.000125).unwrap(); // frame 1
        let (_, frames) = decode(&out).unwrap();
        let values: Vec<f64> = frames.iter().map(|f| f.0).collect();
        for (actual, expected) in values.iter().zip([0.1, 0.9, 0.8, 0.4].iter()) {
            assert!((actual - expected).abs() < 1e-4, "{actual} vs {expected}");
        }
    }

    #[test]
    fn preserves_length_and_is_a_noop_past_the_end() {
        let base = encode(&[(0.5, 0.5); 4], 8_000);
        let insert = encode(&[(0.9, 0.9); 2], 8_000);
        assert_eq!(overwrite(&base, &insert, 1.0).unwrap(), base);
    }

    #[test]
    fn truncates_an_insert_that_runs_past_the_end() {
        let base = encode(&[(0.1, 0.1); 3], 8_000);
        let insert = encode(&[(0.9, 0.9); 5], 8_000);
        let (_, frames) = decode(&overwrite(&base, &insert, 0.000125).unwrap()).unwrap();
        assert_eq!(frames.len(), 3);
        assert!((frames[0].0 - 0.1).abs() < 1e-4);
        assert!(frames[1].0 > 0.8 && frames[2].0 > 0.8);
    }

    #[test]
    fn rejects_bad_inputs() {
        let a = encode(&[(0.5, 0.5)], 8_000);
        let b = encode(&[(0.5, 0.5)], 16_000);
        assert!(overwrite(&a, &b, 0.0).is_err());
        assert!(overwrite(b"bad", &a, 0.0).is_err());
        assert!(overwrite(&a, &a, -1.0).is_err());
    }
}
