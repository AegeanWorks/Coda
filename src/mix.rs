//! Deterministic overlay of two 16-bit stereo Coda WAV buffers.

use crate::render::{wav_bytes, Frame};

fn read_frames(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be mixed".into());
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

pub fn mix_wavs(a: &[u8], b: &[u8], gain_b: f64) -> Result<Vec<u8>, String> {
    if !gain_b.is_finite() || !(0.0..=1.0).contains(&gain_b) {
        return Err("gain_b must be between 0 and 1".into());
    }
    let (rate_a, left) = read_frames(a)?;
    let (rate_b, right) = read_frames(b)?;
    if rate_a != rate_b {
        return Err(format!("sample-rate mismatch: {rate_a} Hz vs {rate_b} Hz"));
    }

    let mut frames = vec![(0.0_f64, 0.0_f64); left.len().max(right.len())];
    for (index, frame) in frames.iter_mut().enumerate() {
        let a = left.get(index).copied().unwrap_or((0.0, 0.0));
        let b = right.get(index).copied().unwrap_or((0.0, 0.0));
        frame.0 = (a.0 + b.0 * gain_b).clamp(-1.0, 1.0);
        frame.1 = (a.1 + b.1 * gain_b).clamp(-1.0, 1.0);
    }
    Ok(wav_bytes(&frames, rate_a))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    #[test]
    fn zero_gain_is_an_identity_for_the_first_input() {
        let a = encode(&[(0.5, -0.5), (0.25, 0.0)], 8_000);
        let b = encode(&[(1.0, 1.0)], 8_000);
        assert_eq!(mix_wavs(&a, &b, 0.0).unwrap(), a);
    }

    #[test]
    fn mixes_frames_and_uses_the_longer_length() {
        let a = encode(&[(0.5, 0.5)], 8_000);
        let b = encode(&[(0.5, 0.5), (0.4, 0.4)], 8_000);
        let mixed = mix_wavs(&a, &b, 0.5).unwrap();
        let (_, frames) = read_frames(&mixed).unwrap();
        assert_eq!(frames.len(), 2);
        assert!((frames[0].0 - 0.75).abs() < 1e-4);
        assert!((frames[1].0 - 0.2).abs() < 1e-4);
    }

    #[test]
    fn clamps_without_exceeding_range() {
        let a = encode(&[(1.0, 1.0)], 8_000);
        let b = encode(&[(1.0, 1.0)], 8_000);
        let (_, frames) = read_frames(&mix_wavs(&a, &b, 1.0).unwrap()).unwrap();
        assert!(frames.iter().all(|f| f.0 <= 1.0 && f.1 <= 1.0));
    }

    #[test]
    fn rejects_bad_inputs() {
        let a = encode(&[(0.5, 0.5)], 8_000);
        let c = encode(&[(0.5, 0.5)], 16_000);
        assert!(mix_wavs(&a, &c, 0.5)
            .unwrap_err()
            .contains("sample-rate mismatch"));
        assert!(mix_wavs(b"bad", &a, 0.5).is_err());
        assert!(mix_wavs(&a, &a, 2.0).is_err());
    }
}
