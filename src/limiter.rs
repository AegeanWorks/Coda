//! Brick-wall limiter for 16-bit stereo Coda WAVs.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be limited".into());
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

/// Clamp every sample to `[-limit, limit]`, keeping frame count and rate.
pub fn apply_limit(wav: &[u8], limit: f64) -> Result<Vec<u8>, String> {
    if !limit.is_finite() || !(0.0..=1.0).contains(&limit) {
        return Err("limit must be between 0 and 1".into());
    }
    let (rate, frames) = decode(wav)?;
    let out: Vec<Frame> = frames
        .iter()
        .map(|&(l, r)| (l.clamp(-limit, limit), r.clamp(-limit, limit)))
        .collect();
    Ok(wav_bytes(&out, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn sample() -> Vec<u8> {
        encode(&[(0.2, -0.2), (0.8, 0.5), (-0.9, 0.1)], 8_000)
    }

    #[test]
    fn full_scale_is_an_identity() {
        assert_eq!(apply_limit(&sample(), 1.0).unwrap(), sample());
    }

    #[test]
    fn caps_every_sample() {
        let out = apply_limit(&sample(), 0.5).unwrap();
        let (_, frames) = decode(&out).unwrap();
        let (_, source) = decode(&sample()).unwrap();
        assert!(frames
            .iter()
            .all(|f| f.0.abs() <= 0.5 + 1e-4 && f.1.abs() <= 0.5 + 1e-4));
        assert!(
            (frames[0].0 - source[0].0).abs() < 1e-4,
            "quiet sample unchanged"
        );
        assert!((frames[1].0 - 0.5).abs() < 1e-4, "loud sample clipped");
        assert!((frames[2].0 + 0.5).abs() < 1e-4, "negative side clipped");
    }

    #[test]
    fn is_deterministic_and_rejects_bad_inputs() {
        assert_eq!(
            apply_limit(&sample(), 0.3).unwrap(),
            apply_limit(&sample(), 0.3).unwrap()
        );
        assert!(apply_limit(b"bad", 0.5).is_err());
        assert!(apply_limit(&sample(), 1.5).is_err());
    }
}
