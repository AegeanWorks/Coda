//! Deterministic amplitude tremolo for 16-bit stereo Coda WAVs.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be tremoloed".into());
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

/// Apply a cosine LFO to amplitude: `gain = 1 - depth/2 + (depth/2)*cos(2*pi*rate*t)`.
/// The gain stays within `[1-depth, 1]`, starts at unity, and depth 0 is byte-identical.
pub fn apply_tremolo(wav: &[u8], rate: f64, depth: f64) -> Result<Vec<u8>, String> {
    if !rate.is_finite() || rate <= 0.0 {
        return Err("LFO rate must be a positive number".into());
    }
    if !depth.is_finite() || !(0.0..=1.0).contains(&depth) {
        return Err("LFO depth must be between 0 and 1".into());
    }
    let (sample_rate, mut frames) = decode(wav)?;
    if depth > 0.0 {
        let half = depth / 2.0;
        let step = std::f64::consts::TAU * rate / sample_rate as f64;
        for (index, frame) in frames.iter_mut().enumerate() {
            let gain = 1.0 - half + half * (step * index as f64).cos();
            frame.0 = (frame.0 * gain).clamp(-1.0, 1.0);
            frame.1 = (frame.1 * gain).clamp(-1.0, 1.0);
        }
    }
    Ok(wav_bytes(&frames, sample_rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn sample() -> Vec<u8> {
        encode(&[(0.8, -0.6); 64], 8_000)
    }

    #[test]
    fn zero_depth_is_an_identity() {
        assert_eq!(apply_tremolo(&sample(), 5.0, 0.0).unwrap(), sample());
    }

    #[test]
    fn modulates_amplitude_without_exceeding_the_source() {
        let out = apply_tremolo(&sample(), 100.0, 1.0).unwrap();
        let (_, frames) = decode(&out).unwrap();
        let (_, source) = decode(&sample()).unwrap();
        let peak = source[0].0;
        assert_eq!(frames.len(), 64);
        assert!(frames.iter().all(|f| f.0.abs() <= peak + 1e-9));
        assert!(frames.iter().any(|f| f.0 < 0.7), "should dip below source");
        assert!((frames[0].0 - peak).abs() < 1e-9, "starts at unity gain");
    }

    #[test]
    fn is_deterministic() {
        assert_eq!(
            apply_tremolo(&sample(), 3.5, 0.7).unwrap(),
            apply_tremolo(&sample(), 3.5, 0.7).unwrap()
        );
    }

    #[test]
    fn rejects_bad_inputs() {
        assert!(apply_tremolo(b"bad", 5.0, 0.5).is_err());
        assert!(apply_tremolo(&sample(), 0.0, 0.5).is_err());
        assert!(apply_tremolo(&sample(), 5.0, 1.5).is_err());
    }
}
