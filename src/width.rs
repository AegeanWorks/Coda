//! Stereo width via mid/side scaling.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be widened".into());
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

pub fn change_width(wav: &[u8], amount: f64) -> Result<Vec<u8>, String> {
    if !amount.is_finite() || !(0.0..=2.0).contains(&amount) {
        return Err("width amount must be between 0 and 2".into());
    }
    let (rate, mut frames) = decode(wav)?;
    for frame in &mut frames {
        let mid = (frame.0 + frame.1) / 2.0;
        let side = (frame.0 - frame.1) / 2.0;
        frame.0 = (mid + side * amount).clamp(-1.0, 1.0);
        frame.1 = (mid - side * amount).clamp(-1.0, 1.0);
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

    #[test]
    fn unity_is_byte_identical() {
        assert_eq!(change_width(&sample(), 1.0).unwrap(), sample());
    }

    #[test]
    fn zero_folds_to_mono() {
        let (_, frames) = decode(&change_width(&sample(), 0.0).unwrap()).unwrap();
        assert!(frames.iter().all(|f| (f.0 - f.1).abs() < 1e-4));
        assert!((frames[0].0 - 0.5).abs() < 1e-4);
    }

    #[test]
    fn widening_expands_the_difference() {
        let (_, frames) = decode(&change_width(&sample(), 2.0).unwrap()).unwrap();
        assert!(
            (frames[0].0 - 1.0).abs() < 0.01,
            "clamped left: {}",
            frames[0].0
        );
        assert!((frames[0].1 + 0.1).abs() < 0.01, "{}", frames[0].1);
    }

    #[test]
    fn rejects_bad_amounts_and_input() {
        assert!(change_width(&sample(), -0.1).is_err());
        assert!(change_width(&sample(), 2.5).is_err());
        assert!(change_width(b"bad", 1.0).is_err());
    }
}
