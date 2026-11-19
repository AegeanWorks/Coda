//! Change WAV playback speed while keeping the declared sample rate.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be sped up".into());
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

/// Resample the timeline by `factor` but write the same sample rate:
/// `factor > 1` shortens, `factor < 1` lengthens. Factor 1 is byte-identical.
pub fn apply_speed(wav: &[u8], factor: f64) -> Result<Vec<u8>, String> {
    if !factor.is_finite() || factor <= 0.0 {
        return Err("factor must be a positive finite number".into());
    }
    let (rate, frames) = decode(wav)?;
    if frames.is_empty() || factor == 1.0 {
        return Ok(wav_bytes(&frames, rate));
    }
    let output_len = (frames.len() as f64 / factor).round() as usize;
    let mut output = Vec::with_capacity(output_len);
    for index in 0..output_len {
        let position = index as f64 * factor;
        let lower = position as usize;
        let fraction = position - lower as f64;
        let a = frames[lower.min(frames.len() - 1)];
        let b = frames[(lower + 1).min(frames.len() - 1)];
        output.push((
            a.0 * (1.0 - fraction) + b.0 * fraction,
            a.1 * (1.0 - fraction) + b.1 * fraction,
        ));
    }
    Ok(wav_bytes(&output, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn sample() -> Vec<u8> {
        encode(&[(0.0, 0.0), (0.5, 0.5), (1.0, 1.0), (1.0, 1.0)], 8_000)
    }

    #[test]
    fn unit_factor_is_an_identity() {
        assert_eq!(apply_speed(&sample(), 1.0).unwrap(), sample());
    }

    #[test]
    fn doubling_halves_the_frames_and_keeps_the_rate() {
        let out = apply_speed(&sample(), 2.0).unwrap();
        let (rate, frames) = decode(&out).unwrap();
        assert_eq!(rate, 8_000);
        assert_eq!(frames.len(), 2);
        assert!((frames[0].0 - 0.0).abs() < 1e-4);
        assert!((frames[1].0 - 1.0).abs() < 1e-4);
    }

    #[test]
    fn halving_doubles_the_frames_and_interpolates() {
        let out = apply_speed(&sample(), 0.5).unwrap();
        let (rate, frames) = decode(&out).unwrap();
        assert_eq!(rate, 8_000);
        assert_eq!(frames.len(), 8);
        assert!((frames[1].0 - 0.25).abs() < 1e-4, "{}", frames[1].0);
    }

    #[test]
    fn is_deterministic_and_rejects_bad_factors() {
        assert_eq!(
            apply_speed(&sample(), 1.5).unwrap(),
            apply_speed(&sample(), 1.5).unwrap()
        );
        assert!(apply_speed(b"bad", 2.0).is_err());
        assert!(apply_speed(&sample(), 0.0).is_err());
    }
}
