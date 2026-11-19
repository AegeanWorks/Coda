//! Adjust stereo width by scaling the side component of a mid/side image.

use crate::render::{wav_bytes, Frame};

const ROOT2: f64 = std::f64::consts::SQRT_2;

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

/// Scale the side component: 0 is mono, 1 is byte-identical, above 1 widens.
pub fn mswidth(wav: &[u8], side: f64) -> Result<Vec<u8>, String> {
    if !side.is_finite() || !(0.0..=2.0).contains(&side) {
        return Err("side must be between 0 and 2".into());
    }
    let (rate, frames) = decode(wav)?;
    if side == 1.0 {
        return Ok(wav_bytes(&frames, rate));
    }
    let out: Vec<Frame> = frames
        .iter()
        .map(|&(l, r)| {
            let mid = (l + r) / ROOT2;
            let difference = ((l - r) / ROOT2) * side;
            (
                ((mid + difference) / ROOT2).clamp(-1.0, 1.0),
                ((mid - difference) / ROOT2).clamp(-1.0, 1.0),
            )
        })
        .collect();
    Ok(wav_bytes(&out, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn sample() -> Vec<u8> {
        encode(&[(0.6, 0.2), (-0.3, 0.8), (0.5, 0.5)], 8_000)
    }

    #[test]
    fn unit_side_is_an_identity() {
        assert_eq!(mswidth(&sample(), 1.0).unwrap(), sample());
    }

    #[test]
    fn zero_side_is_dual_mono() {
        let out = mswidth(&sample(), 0.0).unwrap();
        let (_, frames) = decode(&out).unwrap();
        assert!(
            frames.iter().all(|f| (f.0 - f.1).abs() < 1e-4),
            "{:?}",
            frames
        );
    }

    #[test]
    fn wider_side_increases_the_difference() {
        let out = mswidth(&sample(), 2.0).unwrap();
        let (_, frames) = decode(&out).unwrap();
        let (_, source) = decode(&sample()).unwrap();
        for (before, after) in source.iter().zip(frames.iter()) {
            assert!((after.0 - after.1).abs() >= (before.0 - before.1).abs() - 1e-4);
            assert!(after.0.abs() <= 1.0 && after.1.abs() <= 1.0);
        }
    }

    #[test]
    fn rejects_bad_inputs() {
        assert!(mswidth(b"bad", 1.0).is_err());
        assert!(mswidth(&sample(), -0.1).is_err());
        assert!(mswidth(&sample(), 2.5).is_err());
    }
}
