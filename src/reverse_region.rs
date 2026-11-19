//! Reverse an exact time window of a 16-bit stereo Coda WAV.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be reversed".into());
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

/// Reverse frames between `start` and `end` seconds; head and tail stay unchanged.
pub fn reverse_region(wav: &[u8], start: f64, end: f64) -> Result<Vec<u8>, String> {
    if !start.is_finite() || !end.is_finite() || start < 0.0 || end <= start {
        return Err("range must satisfy 0 <= start < end".into());
    }
    let (rate, mut frames) = decode(wav)?;
    let from = ((start * rate as f64).round() as usize).min(frames.len());
    let to = ((end * rate as f64).round() as usize).min(frames.len());
    if from < to {
        frames[from..to].reverse();
    }
    Ok(wav_bytes(&frames, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn sample() -> Vec<u8> {
        encode(&[(0.1, 0.1), (0.2, 0.2), (0.3, 0.3), (0.4, 0.4)], 8_000)
    }

    #[test]
    fn reverses_only_the_window() {
        let out = reverse_region(&sample(), 0.000125, 0.000375).unwrap(); // frames 1..3
        let (_, frames) = decode(&out).unwrap();
        let values: Vec<f64> = frames.iter().map(|f| f.0).collect();
        assert!((values[0] - 0.1).abs() < 1e-4);
        assert!((values[1] - 0.3).abs() < 1e-4);
        assert!((values[2] - 0.2).abs() < 1e-4);
        assert!((values[3] - 0.4).abs() < 1e-4);
    }

    #[test]
    fn full_window_is_byte_identical_to_a_whole_reverse() {
        let out = reverse_region(&sample(), 0.0, 0.001).unwrap();
        let (_, frames) = decode(&out).unwrap();
        let values: Vec<f64> = frames.iter().map(|f| f.0).collect();
        assert!((values[0] - 0.4).abs() < 1e-4);
        assert!((values[3] - 0.1).abs() < 1e-4);
    }

    #[test]
    fn windows_outside_the_file_are_no_ops() {
        assert_eq!(reverse_region(&sample(), 1.0, 2.0).unwrap(), sample());
    }

    #[test]
    fn rejects_bad_ranges_and_input() {
        let s = sample();
        assert!(reverse_region(b"bad", 0.0, 0.1).is_err());
        assert!(reverse_region(&s, 0.1, 0.1).is_err());
        assert!(reverse_region(&s, -1.0, 0.1).is_err());
    }
}
