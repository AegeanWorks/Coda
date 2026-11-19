//! Lossless concatenation of matching-rate Coda WAVs.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be concatenated".into());
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

/// Append every input's frames in order; no overlap, gain or resampling.
pub fn concat_wavs(inputs: &[&[u8]]) -> Result<Vec<u8>, String> {
    let first = inputs
        .first()
        .ok_or_else(|| "no WAVs to concatenate".to_string())?;
    let (rate, frames) = decode(first)?;
    let mut out = frames.clone();
    for wav in inputs.iter().skip(1) {
        let (other_rate, other) = decode(wav)?;
        if other_rate != rate {
            return Err(format!(
                "sample-rate mismatch: {rate} Hz vs {other_rate} Hz"
            ));
        }
        out.extend_from_slice(&other);
    }
    Ok(wav_bytes(&out, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    #[test]
    fn appends_frames_in_order() {
        let a = encode(&[(0.1, 0.1), (0.2, 0.2)], 8_000);
        let b = encode(&[(0.8, 0.8)], 8_000);
        let out = concat_wavs(&[&a, &b]).unwrap();
        let (rate, frames) = decode(&out).unwrap();
        assert_eq!(rate, 8_000);
        assert_eq!(frames.len(), 3);
        let values: Vec<f64> = frames.iter().map(|f| f.0).collect();
        assert!((values[0] - 0.1).abs() < 1e-4);
        assert!((values[1] - 0.2).abs() < 1e-4);
        assert!((values[2] - 0.8).abs() < 1e-4);
    }

    #[test]
    fn single_input_is_an_identity() {
        let a = encode(&[(0.3, -0.3)], 8_000);
        assert_eq!(concat_wavs(&[&a]).unwrap(), a);
    }

    #[test]
    fn rejects_rate_mismatch_bad_input_and_empty_list() {
        let a = encode(&[(0.5, 0.5)], 8_000);
        let b = encode(&[(0.5, 0.5)], 16_000);
        assert!(concat_wavs(&[&a, &b]).is_err());
        assert!(concat_wavs(&[b"bad", &a]).is_err());
        assert!(concat_wavs(&[]).is_err());
    }
}
