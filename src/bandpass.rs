//! Cascaded one-pole high-pass and low-pass filter for Coda WAVs.

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

/// Remove slow drift with a high-pole stage, then soften top end with a low-pole stage.
/// `high_alpha = 1, low_alpha = 1` passes the first difference; `low_alpha = 0` is silence.
pub fn apply_bandpass(wav: &[u8], high_alpha: f64, low_alpha: f64) -> Result<Vec<u8>, String> {
    for alpha in [high_alpha, low_alpha] {
        if !alpha.is_finite() || !(0.0..=1.0).contains(&alpha) {
            return Err("filter coefficients must be between 0 and 1".into());
        }
    }
    let (rate, frames) = decode(wav)?;
    let mut out = Vec::with_capacity(frames.len());
    let mut hp_state = (0.0_f64, 0.0_f64);
    let mut lp_state = (0.0_f64, 0.0_f64);
    let mut first = true;
    for &(left, right) in &frames {
        if first {
            hp_state = (left, right);
            first = false;
        }
        let high = (
            (left - hp_state.0).clamp(-1.0, 1.0),
            (right - hp_state.1).clamp(-1.0, 1.0),
        );
        hp_state.0 += high_alpha * (left - hp_state.0);
        hp_state.1 += high_alpha * (right - hp_state.1);
        lp_state.0 += low_alpha * (high.0 - lp_state.0);
        lp_state.1 += low_alpha * (high.1 - lp_state.1);
        out.push((lp_state.0.clamp(-1.0, 1.0), lp_state.1.clamp(-1.0, 1.0)));
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
    fn full_stages_pass_the_first_difference() {
        let out = apply_bandpass(&sample(), 1.0, 1.0).unwrap();
        let (_, frames) = decode(&out).unwrap();
        let (_, source) = decode(&sample()).unwrap();
        assert_eq!(frames[0], (0.0, 0.0));
        for i in 1..frames.len() {
            let left = (source[i].0 - source[i - 1].0).clamp(-1.0, 1.0);
            let right = (source[i].1 - source[i - 1].1).clamp(-1.0, 1.0);
            assert!(
                (frames[i].0 - left).abs() < 1e-4,
                "{} vs {}",
                frames[i].0,
                left
            );
            assert!(
                (frames[i].1 - right).abs() < 1e-4,
                "{} vs {}",
                frames[i].1,
                right
            );
        }
    }

    #[test]
    fn zero_low_stage_is_silence() {
        let out = apply_bandpass(&sample(), 1.0, 0.0).unwrap();
        let (_, frames) = decode(&out).unwrap();
        assert!(frames.iter().all(|f| f.0 == 0.0 && f.1 == 0.0));
    }

    #[test]
    fn zero_high_stage_removes_the_first_sample_offset() {
        let out = apply_bandpass(&sample(), 0.0, 1.0).unwrap();
        let (_, frames) = decode(&out).unwrap();
        let (_, source) = decode(&sample()).unwrap();
        assert_eq!(frames[0], (0.0, 0.0));
        assert!((frames[1].0 - (source[1].0 - source[0].0)).abs() < 1e-4);
    }

    #[test]
    fn rejects_bad_inputs() {
        assert!(apply_bandpass(b"bad", 0.5, 0.5).is_err());
        assert!(apply_bandpass(&sample(), 1.5, 0.5).is_err());
        assert!(apply_bandpass(&sample(), 0.5, -0.1).is_err());
    }
}
