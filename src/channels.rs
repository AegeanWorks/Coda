//! Deterministic stereo channel operations on 16-bit Coda WAVs.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be processed".into());
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

pub fn modes() -> Vec<&'static str> {
    vec!["mono", "swap", "left", "right"]
}

pub fn transform(wav: &[u8], mode: &str) -> Result<Vec<u8>, String> {
    if !modes().contains(&mode) {
        return Err(format!("unknown channel mode '{mode}'"));
    }
    let (rate, mut frames) = decode(wav)?;
    for frame in &mut frames {
        let (left, right) = *frame;
        *frame = match mode {
            "mono" => {
                let middle = (left + right) / 2.0;
                (middle, middle)
            }
            "swap" => (right, left),
            "left" => (left, left),
            _ => (right, right),
        };
    }
    Ok(wav_bytes(&frames, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn sample() -> Vec<u8> {
        encode(&[(0.8, 0.2), (-0.4, 0.6)], 8_000)
    }

    fn frames_of(wav: &[u8]) -> Vec<Frame> {
        decode(wav).unwrap().1
    }

    #[test]
    fn modes_are_stable() {
        assert_eq!(modes(), vec!["mono", "swap", "left", "right"]);
        assert!(transform(&sample(), "nope").is_err());
    }

    #[test]
    fn mono_averages_both_sides() {
        let frames = frames_of(&transform(&sample(), "mono").unwrap());
        assert!((frames[0].0 - 0.5).abs() < 1e-4);
        assert_eq!(frames[0].0, frames[0].1);
    }

    #[test]
    fn swap_exchanges_sides() {
        let frames = frames_of(&transform(&sample(), "swap").unwrap());
        assert!((frames[0].0 - 0.2).abs() < 1e-4);
        assert!((frames[0].1 - 0.8).abs() < 1e-4);
    }

    #[test]
    fn left_and_right_duplicate_one_side() {
        let left = frames_of(&transform(&sample(), "left").unwrap());
        let right = frames_of(&transform(&sample(), "right").unwrap());
        assert!((left[0].0 - 0.8).abs() < 1e-4);
        assert_eq!(left[0].0, left[0].1);
        assert!((right[0].0 - 0.2).abs() < 1e-4);
        assert_eq!(right[0].0, right[0].1);
    }

    #[test]
    fn length_and_rate_are_preserved() {
        let input = sample();
        let output = transform(&input, "swap").unwrap();
        assert_eq!(output.len(), input.len());
        assert_eq!(decode(&output).unwrap().0, 8_000);
    }
}
