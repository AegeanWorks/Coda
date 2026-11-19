//! Deterministic pitch vibrato for 16-bit stereo Coda WAVs.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can receive vibrato".into());
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

/// Warp the sample timeline with a sine LFO of `rate` Hz and `depth` frames.
/// Depth 0 is byte-identical; frame count and rate are preserved.
pub fn apply_vibrato(wav: &[u8], rate: f64, depth: f64) -> Result<Vec<u8>, String> {
    if !rate.is_finite() || rate <= 0.0 {
        return Err("rate must be a positive number".into());
    }
    if !depth.is_finite() || depth < 0.0 {
        return Err("depth must be a non-negative number".into());
    }
    let (sample_rate, frames) = decode(wav)?;
    if frames.is_empty() || depth == 0.0 {
        return Ok(wav_bytes(&frames, sample_rate));
    }
    let last = frames.len() - 1;
    let step = std::f64::consts::TAU * rate / sample_rate as f64;
    let mut out = Vec::with_capacity(frames.len());
    for index in 0..frames.len() {
        let offset = depth * (step * index as f64).sin();
        let position = (index as f64 + offset).clamp(0.0, last as f64);
        let lower = position as usize;
        let fraction = position - lower as f64;
        let a = frames[lower];
        let b = frames[(lower + 1).min(last)];
        out.push((
            a.0 * (1.0 - fraction) + b.0 * fraction,
            a.1 * (1.0 - fraction) + b.1 * fraction,
        ));
    }
    Ok(wav_bytes(&out, sample_rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn sample() -> Vec<u8> {
        encode(&[(0.0, 0.0), (0.5, 0.5), (1.0, 1.0), (0.5, 0.5)], 8_000)
    }

    #[test]
    fn zero_depth_is_an_identity() {
        assert_eq!(apply_vibrato(&sample(), 5.0, 0.0).unwrap(), sample());
    }

    #[test]
    fn preserves_length_and_is_deterministic() {
        let out = apply_vibrato(&sample(), 100.0, 0.5).unwrap();
        let (rate, frames) = decode(&out).unwrap();
        assert_eq!(rate, 8_000);
        assert_eq!(frames.len(), 4);
        assert!(frames.iter().all(|f| f.0.abs() <= 1.0 && f.1.abs() <= 1.0));
        assert_eq!(out, apply_vibrato(&sample(), 100.0, 0.5).unwrap());
    }

    #[test]
    fn changes_samples_when_depth_is_positive() {
        let out = apply_vibrato(&sample(), 1000.0, 1.5).unwrap();
        assert_ne!(out, sample());
    }

    #[test]
    fn rejects_bad_inputs() {
        assert!(apply_vibrato(b"bad", 5.0, 0.5).is_err());
        assert!(apply_vibrato(&sample(), 0.0, 0.5).is_err());
        assert!(apply_vibrato(&sample(), 5.0, -1.0).is_err());
    }
}
