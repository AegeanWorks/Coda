//! Static equal-power stereo panning for 16-bit stereo Coda WAVs.

use crate::render::{wav_bytes, Frame};

const CENTRE: f64 = std::f64::consts::FRAC_1_SQRT_2;

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be panned".into());
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

/// Place the stereo image at a fixed `pan` (-1 left, 0 centre, 1 right).
/// Gains follow the equal-power law, so centre is byte-identical.
pub fn apply_pan(wav: &[u8], pan: f64) -> Result<Vec<u8>, String> {
    if !pan.is_finite() || !(-1.0..=1.0).contains(&pan) {
        return Err("pan must be between -1 and 1".into());
    }
    let (rate, frames) = decode(wav)?;
    let angle = (pan + 1.0) * std::f64::consts::FRAC_PI_4;
    let left = (angle.cos() / CENTRE).min(2.0_f64.sqrt());
    let right = (angle.sin() / CENTRE).min(2.0_f64.sqrt());
    let out: Vec<Frame> = frames
        .iter()
        .map(|&(l, r)| ((l * left).clamp(-1.0, 1.0), (r * right).clamp(-1.0, 1.0)))
        .collect();
    Ok(wav_bytes(&out, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn sample() -> Vec<u8> {
        encode(&[(0.5, -0.25), (0.1, 0.9)], 8_000)
    }

    #[test]
    fn centre_is_an_identity() {
        assert_eq!(apply_pan(&sample(), 0.0).unwrap(), sample());
    }

    #[test]
    fn hard_sides_isolate_one_channel() {
        let left = apply_pan(&sample(), -1.0).unwrap();
        let right = apply_pan(&sample(), 1.0).unwrap();
        let (_, lf) = decode(&left).unwrap();
        let (_, rf) = decode(&right).unwrap();
        assert!(lf.iter().all(|f| f.1 == 0.0), "left pan silences right");
        assert!(rf.iter().all(|f| f.0 == 0.0), "right pan silences left");
    }

    #[test]
    fn partial_pan_keeps_both_channels_bounded() {
        let out = apply_pan(&sample(), 0.5).unwrap();
        let (_, frames) = decode(&out).unwrap();
        let (_, source) = decode(&sample()).unwrap();
        assert!(frames.iter().all(|f| f.0.abs() <= 1.0 && f.1.abs() <= 1.0));
        assert!(frames[1].1.abs() > source[1].1.abs(), "right grows");
        assert!(frames[1].0.abs() < source[1].0.abs(), "left shrinks");
    }

    #[test]
    fn is_deterministic_and_rejects_bad_inputs() {
        assert_eq!(
            apply_pan(&sample(), 0.3).unwrap(),
            apply_pan(&sample(), 0.3).unwrap()
        );
        assert!(apply_pan(b"bad", 0.0).is_err());
        assert!(apply_pan(&sample(), 1.5).is_err());
    }
}
