//! A single fixed delayed copy mixed into the dry signal (slapback).

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be slapped back".into());
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

/// Mix in a single copy delayed by `delay` seconds at wet level `mix`.
/// There is no feedback; zero delay or mix is byte-identical.
pub fn apply_slapback(wav: &[u8], delay: f64, mix: f64) -> Result<Vec<u8>, String> {
    if !delay.is_finite() || delay < 0.0 {
        return Err("delay must be a non-negative number".into());
    }
    if !mix.is_finite() || !(0.0..=1.0).contains(&mix) {
        return Err("mix must be between 0 and 1".into());
    }
    let (rate, frames) = decode(wav)?;
    let d = (delay * rate as f64).round() as usize;
    if frames.is_empty() || d == 0 || mix == 0.0 || d >= frames.len() {
        return Ok(wav_bytes(&frames, rate));
    }
    let out: Vec<Frame> = frames
        .iter()
        .enumerate()
        .map(|(index, &dry)| {
            if index < d {
                dry
            } else {
                let tap = frames[index - d];
                (
                    (dry.0 + tap.0 * mix).clamp(-1.0, 1.0),
                    (dry.1 + tap.1 * mix).clamp(-1.0, 1.0),
                )
            }
        })
        .collect();
    Ok(wav_bytes(&out, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn impulse() -> Vec<u8> {
        let mut frames = vec![(0.0_f64, 0.0_f64); 8];
        frames[0] = (1.0, 0.5);
        encode(&frames, 8_000)
    }

    #[test]
    fn zero_delay_or_mix_is_an_identity() {
        let wav = impulse();
        assert_eq!(apply_slapback(&wav, 0.0, 0.5).unwrap(), wav);
        assert_eq!(apply_slapback(&wav, 0.001, 0.0).unwrap(), wav);
    }

    #[test]
    fn adds_exactly_one_delayed_tap() {
        let out = apply_slapback(&impulse(), 0.000375, 0.5).unwrap(); // D = 3
        let (_, frames) = decode(&out).unwrap();
        assert!(frames[0].0 > 0.9, "original impulse kept");
        assert!(
            frames[1].0.abs() < 1e-4 && frames[2].0.abs() < 1e-4,
            "gap before tap"
        );
        assert!((frames[3].0 - 0.5).abs() < 1e-4, "tap at D, scaled by mix");
        assert!((frames[3].1 - 0.25).abs() < 1e-4, "right-channel tap");
        assert!(
            frames[4].0.abs() < 1e-4 && frames[5].0.abs() < 1e-4,
            "no feedback tail"
        );
    }

    #[test]
    fn tap_level_follows_the_wet_mix() {
        let mut frames = vec![(0.0_f64, 0.0_f64); 8];
        frames[0] = (1.0, 1.0);
        let wav = encode(&frames, 8_000);
        let out = apply_slapback(&wav, 0.000125, 0.5).unwrap(); // D = 1
        let (_, decoded) = decode(&out).unwrap();
        assert!((decoded[1].0 - 0.5).abs() < 1e-4, "{}", decoded[1].0);
    }

    #[test]
    fn is_bounded_deterministic_and_rejects_bad_inputs() {
        let out = apply_slapback(&impulse(), 0.000125, 1.0).unwrap();
        let (_, frames) = decode(&out).unwrap();
        assert!(frames.iter().all(|f| f.0.abs() <= 1.0 && f.1.abs() <= 1.0));
        assert_eq!(out, apply_slapback(&impulse(), 0.000125, 1.0).unwrap());
        let wav = impulse();
        assert!(apply_slapback(b"bad", 0.001, 0.5).is_err());
        assert!(apply_slapback(&wav, -1.0, 0.5).is_err());
        assert!(apply_slapback(&wav, 0.001, 1.5).is_err());
    }
}
