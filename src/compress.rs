//! Deterministic downward dynamics compression for Coda WAVs.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be compressed".into());
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

fn compress_sample(value: f64, threshold: f64, ratio: f64) -> f64 {
    let magnitude = value.abs();
    if magnitude <= threshold {
        return value;
    }
    let reduced = threshold + (magnitude - threshold) / ratio;
    reduced.copysign(value)
}

/// Attenuate samples above `threshold` by `ratio`:1, leaving quiet audio untouched.
/// `threshold = 1` or `ratio = 1` is byte-identical.
pub fn compress(wav: &[u8], threshold: f64, ratio: f64) -> Result<Vec<u8>, String> {
    if !threshold.is_finite() || !(0.0..=1.0).contains(&threshold) {
        return Err("threshold must be between 0 and 1".into());
    }
    if !ratio.is_finite() || !(1.0..=20.0).contains(&ratio) {
        return Err("ratio must be between 1 and 20".into());
    }
    let (rate, frames) = decode(wav)?;
    let out: Vec<Frame> = frames
        .iter()
        .map(|&(l, r)| {
            (
                compress_sample(l, threshold, ratio),
                compress_sample(r, threshold, ratio),
            )
        })
        .collect();
    Ok(wav_bytes(&out, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn sample() -> Vec<u8> {
        encode(&[(0.2, -0.2), (0.8, -0.8), (1.0, 0.5)], 8_000)
    }

    #[test]
    fn identity_at_threshold_one_or_ratio_one() {
        assert_eq!(compress(&sample(), 1.0, 4.0).unwrap(), sample());
        assert_eq!(compress(&sample(), 0.5, 1.0).unwrap(), sample());
    }

    #[test]
    fn leaves_quiet_audio_and_reduces_loud_audio() {
        let out = compress(&sample(), 0.5, 4.0).unwrap();
        let (_, frames) = decode(&out).unwrap();
        let (_, source) = decode(&sample()).unwrap();
        assert!((frames[0].0 - source[0].0).abs() < 1e-4, "quiet unchanged");
        assert!((frames[1].0 - 0.575).abs() < 1e-4, "{}", frames[1].0);
        assert!((frames[1].1 + 0.575).abs() < 1e-4, "sign preserved");
        assert!((frames[2].0 - 0.625).abs() < 1e-4, "peak reduced");
    }

    #[test]
    fn is_bounded_and_deterministic() {
        let out = compress(&sample(), 0.3, 8.0).unwrap();
        let (_, frames) = decode(&out).unwrap();
        assert!(frames.iter().all(|f| f.0.abs() <= 1.0 && f.1.abs() <= 1.0));
        assert_eq!(out, compress(&sample(), 0.3, 8.0).unwrap());
    }

    #[test]
    fn rejects_bad_inputs() {
        let s = sample();
        assert!(compress(b"bad", 0.5, 4.0).is_err());
        assert!(compress(&s, 1.5, 4.0).is_err());
        assert!(compress(&s, 0.5, 0.5).is_err());
    }
}
