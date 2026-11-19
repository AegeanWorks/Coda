//! Deterministic one-pole low-pass filter for 16-bit stereo Coda WAVs.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be filtered".into());
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

/// Smooth each channel with `y = y + alpha * (x - y)`.
/// `alpha = 1` passes audio unchanged; `alpha = 0` holds the first sample.
pub fn apply_lowpass(wav: &[u8], alpha: f64) -> Result<Vec<u8>, String> {
    if !alpha.is_finite() || !(0.0..=1.0).contains(&alpha) {
        return Err("alpha must be between 0 and 1".into());
    }
    let (rate, frames) = decode(wav)?;
    let mut out = Vec::with_capacity(frames.len());
    let mut state = (0.0_f64, 0.0_f64);
    let mut first = true;
    for &(left, right) in &frames {
        if first {
            state = (left, right);
            first = false;
        } else {
            state.0 += alpha * (left - state.0);
            state.1 += alpha * (right - state.1);
        }
        out.push(state);
    }
    Ok(wav_bytes(&out, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn sample() -> Vec<u8> {
        encode(&[(0.2, -0.2), (0.8, 0.4), (-0.6, 0.6), (0.1, -0.1)], 8_000)
    }

    #[test]
    fn full_alpha_is_an_identity() {
        assert_eq!(apply_lowpass(&sample(), 1.0).unwrap(), sample());
    }

    #[test]
    fn zero_alpha_holds_the_first_sample() {
        let out = apply_lowpass(&sample(), 0.0).unwrap();
        let (_, frames) = decode(&out).unwrap();
        let (_, source) = decode(&sample()).unwrap();
        assert!(frames.iter().all(|f| *f == source[0]));
    }

    #[test]
    fn half_alpha_smooths_a_step() {
        let wav = encode(&[(0.0, 0.0), (1.0, 1.0), (1.0, 1.0)], 8_000);
        let out = apply_lowpass(&wav, 0.5).unwrap();
        let (_, frames) = decode(&out).unwrap();
        assert_eq!(frames[0], (0.0, 0.0));
        assert!((frames[1].0 - 0.5).abs() < 0.01, "{}", frames[1].0);
        assert!((frames[2].0 - 0.75).abs() < 0.01, "{}", frames[2].0);
    }

    #[test]
    fn is_deterministic_and_rejects_bad_inputs() {
        assert_eq!(
            apply_lowpass(&sample(), 0.3).unwrap(),
            apply_lowpass(&sample(), 0.3).unwrap()
        );
        assert!(apply_lowpass(b"bad", 0.5).is_err());
        assert!(apply_lowpass(&sample(), 1.5).is_err());
    }
}
