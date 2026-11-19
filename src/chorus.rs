//! Chorus: a dry signal mixed with an LFO-modulated delayed copy.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be chorused".into());
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

fn sample_at(frames: &[Frame], position: f64) -> Frame {
    let last = frames.len() - 1;
    let lower = position.floor().max(0.0) as usize;
    let upper = (lower + 1).min(last);
    let fraction = (position - lower as f64).clamp(0.0, 1.0);
    let a = frames[lower.min(last)];
    let b = frames[upper];
    (
        a.0 * (1.0 - fraction) + b.0 * fraction,
        a.1 * (1.0 - fraction) + b.1 * fraction,
    )
}

/// Mix the dry signal with a delayed copy whose delay follows a sine LFO.
/// `mix = 0` is byte-identical; length and rate are preserved.
pub fn apply_chorus(
    wav: &[u8],
    rate: f64,
    depth: f64,
    base_delay: f64,
    mix: f64,
) -> Result<Vec<u8>, String> {
    if !rate.is_finite() || rate <= 0.0 {
        return Err("rate must be a positive number".into());
    }
    if !depth.is_finite() || depth < 0.0 {
        return Err("depth must be a non-negative number".into());
    }
    if !base_delay.is_finite() || base_delay < 0.0 {
        return Err("delay must be a non-negative number".into());
    }
    if !mix.is_finite() || !(0.0..=1.0).contains(&mix) {
        return Err("mix must be between 0 and 1".into());
    }
    let (sample_rate, frames) = decode(wav)?;
    if frames.is_empty() || mix == 0.0 {
        return Ok(wav_bytes(&frames, sample_rate));
    }
    let last = frames.len() - 1;
    let step = std::f64::consts::TAU * rate / sample_rate as f64;
    let out: Vec<Frame> = frames
        .iter()
        .enumerate()
        .map(|(index, &dry)| {
            let delay = base_delay + depth * (step * index as f64).sin();
            let source = (index as f64 - delay).clamp(0.0, last as f64);
            let wet = sample_at(&frames, source);
            (
                (dry.0 * (1.0 - mix) + wet.0 * mix).clamp(-1.0, 1.0),
                (dry.1 * (1.0 - mix) + wet.1 * mix).clamp(-1.0, 1.0),
            )
        })
        .collect();
    Ok(wav_bytes(&out, sample_rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn sample() -> Vec<u8> {
        encode(
            &[(0.0, 0.0), (0.5, 0.5), (1.0, 1.0), (0.5, 0.5), (0.0, 0.0)],
            8_000,
        )
    }

    #[test]
    fn zero_mix_is_an_identity() {
        assert_eq!(
            apply_chorus(&sample(), 5.0, 1.0, 2.0, 0.0).unwrap(),
            sample()
        );
    }

    #[test]
    fn preserves_length_and_is_deterministic_and_bounded() {
        let out = apply_chorus(&sample(), 50.0, 1.5, 2.0, 0.5).unwrap();
        let (rate, frames) = decode(&out).unwrap();
        assert_eq!(rate, 8_000);
        assert_eq!(frames.len(), 5);
        assert!(frames.iter().all(|f| f.0.abs() <= 1.0 && f.1.abs() <= 1.0));
        assert_eq!(out, apply_chorus(&sample(), 50.0, 1.5, 2.0, 0.5).unwrap());
    }

    #[test]
    fn modulated_delay_changes_samples() {
        let out = apply_chorus(&sample(), 1000.0, 2.0, 2.0, 0.7).unwrap();
        assert_ne!(out, sample());
    }

    #[test]
    fn rejects_bad_inputs() {
        let s = sample();
        assert!(apply_chorus(b"bad", 5.0, 1.0, 2.0, 0.5).is_err());
        assert!(apply_chorus(&s, 0.0, 1.0, 2.0, 0.5).is_err());
        assert!(apply_chorus(&s, 5.0, -1.0, 2.0, 0.5).is_err());
        assert!(apply_chorus(&s, 5.0, 1.0, -1.0, 0.5).is_err());
        assert!(apply_chorus(&s, 5.0, 1.0, 2.0, 1.5).is_err());
    }
}
