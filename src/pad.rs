//! Add exact silence before and after a Coda WAV.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be padded".into());
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

pub fn pad(wav: &[u8], before: f64, after: f64) -> Result<Vec<u8>, String> {
    if !before.is_finite() || !after.is_finite() || before < 0.0 || after < 0.0 {
        return Err("padding durations must be non-negative numbers".into());
    }
    let (rate, frames) = decode(wav)?;
    let leading = (before * rate as f64).round() as usize;
    let trailing = (after * rate as f64).round() as usize;
    let mut out = vec![(0.0_f64, 0.0_f64); leading];
    out.extend_from_slice(&frames);
    out.extend(vec![(0.0_f64, 0.0_f64); trailing]);
    Ok(wav_bytes(&out, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn sample() -> Vec<u8> {
        encode(&[(0.5, 0.25); 8], 8_000)
    }

    #[test]
    fn inserts_exact_silence_and_shifts_audio() {
        let out = pad(&sample(), 0.001, 0.0005).unwrap();
        let (_, frames) = decode(&out).unwrap();
        assert_eq!(frames.len(), 8 + 8 + 4);
        assert!(frames[..8].iter().all(|f| f.0 == 0.0 && f.1 == 0.0));
        assert!((frames[8].0 - 0.5).abs() < 1e-4);
        assert!(frames[20..].iter().all(|f| f.0 == 0.0 && f.1 == 0.0));
    }

    #[test]
    fn zero_padding_is_byte_identical() {
        assert_eq!(pad(&sample(), 0.0, 0.0).unwrap(), sample());
    }

    #[test]
    fn rejects_bad_inputs() {
        assert!(pad(b"bad", 0.0, 0.0).is_err());
        assert!(pad(&sample(), -1.0, 0.0).is_err());
    }
}
