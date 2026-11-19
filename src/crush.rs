//! Deterministic bitcrush and sample-and-hold decimation for Coda WAVs.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be crushed".into());
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

fn quantize(value: f64, mask: i16) -> f64 {
    let sample = (value * 32767.0).round() as i16;
    (sample & mask) as f64 / 32767.0
}

/// Reduce effective resolution to `bits` and repeat each sample `hold` times.
/// `bits = 16` with `hold = 1` is byte-identical; frame count and rate are preserved.
pub fn apply_crush(wav: &[u8], bits: u8, hold: usize) -> Result<Vec<u8>, String> {
    if !(1..=16).contains(&bits) {
        return Err("bits must be between 1 and 16".into());
    }
    if hold == 0 {
        return Err("hold must be at least 1".into());
    }
    let (rate, frames) = decode(wav)?;
    let mask = (-1_i16) << (16 - bits);
    let mut out = Vec::with_capacity(frames.len());
    let mut current = (0.0_f64, 0.0_f64);
    for (index, frame) in frames.iter().enumerate() {
        if index % hold == 0 {
            current = (quantize(frame.0, mask), quantize(frame.1, mask));
        }
        out.push(current);
    }
    Ok(wav_bytes(&out, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn sample() -> Vec<u8> {
        encode(&[(0.5, -0.25), (0.1, 0.9), (-0.8, 0.3), (0.6, -0.6)], 8_000)
    }

    fn samples(wav: &[u8]) -> Vec<(i16, i16)> {
        wav[44..]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| {
                (
                    i16::from_le_bytes([c[0], c[1]]),
                    i16::from_le_bytes([c[2], c[3]]),
                )
            })
            .collect()
    }

    #[test]
    fn full_resolution_no_hold_is_an_identity() {
        assert_eq!(apply_crush(&sample(), 16, 1).unwrap(), sample());
    }

    #[test]
    fn holds_each_sample_for_the_factor() {
        let out = apply_crush(&sample(), 16, 2).unwrap();
        let frames = samples(&out);
        assert_eq!(frames.len(), 4);
        assert_eq!(frames[0], frames[1]);
        assert_eq!(frames[2], frames[3]);
        assert_eq!(frames[0], samples(&sample())[0]);
        assert_eq!(frames[2], samples(&sample())[2]);
    }

    #[test]
    fn quantizes_to_the_requested_grid() {
        let out = apply_crush(&sample(), 8, 1).unwrap();
        for (left, right) in samples(&out) {
            assert_eq!(left & 0x00FF, 0);
            assert_eq!(right & 0x00FF, 0);
        }
    }

    #[test]
    fn rejects_bad_inputs() {
        assert!(apply_crush(b"bad", 8, 1).is_err());
        assert!(apply_crush(&sample(), 0, 1).is_err());
        assert!(apply_crush(&sample(), 17, 1).is_err());
        assert!(apply_crush(&sample(), 8, 0).is_err());
    }
}
