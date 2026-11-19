//! Per-channel DC-offset removal for 16-bit stereo Coda WAVs.

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

pub fn remove_dc(wav: &[u8]) -> Result<Vec<u8>, String> {
    let (rate, mut frames) = decode(wav)?;
    if frames.is_empty() {
        return Ok(wav_bytes(&frames, rate));
    }
    let (left_sum, right_sum) = frames
        .iter()
        .fold((0.0_f64, 0.0_f64), |(l, r), f| (l + f.0, r + f.1));
    let count = frames.len() as f64;
    let (left_mean, right_mean) = (left_sum / count, right_sum / count);
    for frame in &mut frames {
        frame.0 = (frame.0 - left_mean).clamp(-1.0, 1.0);
        frame.1 = (frame.1 - right_mean).clamp(-1.0, 1.0);
    }
    Ok(wav_bytes(&frames, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    #[test]
    fn zero_mean_is_an_identity() {
        let input = encode(&[(0.5, 0.2), (-0.5, -0.2)], 8_000);
        assert_eq!(remove_dc(&input).unwrap(), input);
    }

    #[test]
    fn removes_a_known_offset() {
        let input = encode(&[(0.6, 0.4), (0.4, 0.2)], 8_000);
        let (_, frames) = decode(&remove_dc(&input).unwrap()).unwrap();
        assert!((frames[0].0 - 0.1).abs() < 0.01, "{}", frames[0].0);
        assert!((frames[0].1 - 0.1).abs() < 0.01, "{}", frames[0].1);
        assert!((frames[1].0 + 0.1).abs() < 0.01, "{}", frames[1].0);
    }

    #[test]
    fn rejects_bad_input() {
        assert!(remove_dc(b"bad").is_err());
    }
}
