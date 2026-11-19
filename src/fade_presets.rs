//! Named fade envelopes applied to an existing Coda WAV.

use crate::fade;
use crate::render::{wav_bytes, Frame};

pub const PRESETS: [(&str, f64, f64); 5] = [
    ("short", 0.01, 0.01),
    ("gentle", 0.05, 0.10),
    ("long", 0.20, 0.40),
    ("tail", 0.0, 0.50),
    ("attack", 0.30, 0.0),
];

pub fn names() -> Vec<&'static str> {
    PRESETS.iter().map(|(name, _, _)| *name).collect()
}

pub fn lookup(name: &str) -> Option<(f64, f64)> {
    PRESETS
        .iter()
        .find(|(candidate, _, _)| *candidate == name)
        .map(|(_, attack, release)| (*attack, *release))
}

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be faded".into());
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

pub fn apply(wav: &[u8], preset: &str) -> Result<Vec<u8>, String> {
    let (attack, release) =
        lookup(preset).ok_or_else(|| format!("unknown fade preset '{preset}'"))?;
    let (rate, mut frames) = decode(wav)?;
    fade::apply_fades(&mut frames, rate, attack, release);
    Ok(wav_bytes(&frames, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn sample() -> Vec<u8> {
        encode(&[(0.5, 0.5); 100], 8_000)
    }

    #[test]
    fn preset_names_are_stable() {
        assert_eq!(names(), vec!["short", "gentle", "long", "tail", "attack"]);
        assert_eq!(lookup("tail"), Some((0.0, 0.5)));
        assert_eq!(lookup("nope"), None);
    }

    #[test]
    fn short_fade_softens_the_head() {
        let faded = apply(&sample(), "short").unwrap();
        assert_eq!(faded.len(), sample().len());
        let (_, frames) = decode(&faded).unwrap();
        assert!(frames[0].0 < 0.1, "head should be quiet: {}", frames[0].0);
        assert!(frames[99].0 < 0.1, "tail should be quiet");
    }

    #[test]
    fn tail_preserves_the_head() {
        let faded = apply(&sample(), "tail").unwrap();
        let (_, frames) = decode(&faded).unwrap();
        assert!((frames[0].0 - 0.5).abs() < 1e-4);
        assert!(frames[99].0 < 0.1);
    }

    #[test]
    fn unknown_preset_and_bad_wav_fail() {
        assert!(apply(&sample(), "nope").is_err());
        assert!(apply(b"bad", "short").is_err());
    }
}
