//! Deterministic soft-clip saturation for 16-bit stereo Coda WAVs.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be distorted".into());
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

fn saturate(value: f64, drive: f64) -> f64 {
    ((1.0 + drive) * value / (1.0 + drive * value.abs())).clamp(-1.0, 1.0)
}

/// Apply a bounded soft-saturation curve. `drive = 0` is byte-identical.
pub fn apply_distort(wav: &[u8], drive: f64) -> Result<Vec<u8>, String> {
    if !drive.is_finite() || !(0.0..=1.0).contains(&drive) {
        return Err("drive must be between 0 and 1".into());
    }
    let (rate, frames) = decode(wav)?;
    let out: Vec<Frame> = frames
        .iter()
        .map(|&(l, r)| (saturate(l, drive), saturate(r, drive)))
        .collect();
    Ok(wav_bytes(&out, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn sample() -> Vec<u8> {
        encode(&[(0.2, -0.2), (0.5, 0.5), (1.0, -1.0)], 8_000)
    }

    #[test]
    fn zero_drive_is_an_identity() {
        assert_eq!(apply_distort(&sample(), 0.0).unwrap(), sample());
    }

    #[test]
    fn stays_bounded_and_is_monotonic() {
        let out = apply_distort(&sample(), 1.0).unwrap();
        let (_, frames) = decode(&out).unwrap();
        let (_, source) = decode(&sample()).unwrap();
        assert!(frames.iter().all(|f| f.0.abs() <= 1.0 && f.1.abs() <= 1.0));
        for (before, after) in source.iter().zip(frames.iter()) {
            assert!(
                after.0.abs() >= before.0.abs() - 1e-9,
                "left magnitude grows"
            );
            assert!(
                after.1.abs() >= before.1.abs() - 1e-9,
                "right magnitude grows"
            );
            assert!((after.0 >= 0.0) == (before.0 >= 0.0), "left sign preserved");
            assert!(
                (after.1 >= 0.0) == (before.1 >= 0.0),
                "right sign preserved"
            );
        }
    }

    #[test]
    fn boosts_quiet_signals_more_than_loud_ones() {
        let wav = encode(&[(0.2, 0.2), (1.0, 1.0)], 8_000);
        let out = apply_distort(&wav, 1.0).unwrap();
        let (_, frames) = decode(&out).unwrap();
        let (_, source) = decode(&wav).unwrap();
        assert!(frames[0].0 > source[0].0, "quiet sample boosted");
        assert!((frames[1].0 - 1.0).abs() < 1e-4, "full scale stays bounded");
    }

    #[test]
    fn is_deterministic_and_rejects_bad_inputs() {
        assert_eq!(
            apply_distort(&sample(), 0.5).unwrap(),
            apply_distort(&sample(), 0.5).unwrap()
        );
        assert!(apply_distort(b"bad", 0.5).is_err());
        assert!(apply_distort(&sample(), 1.5).is_err());
    }
}
