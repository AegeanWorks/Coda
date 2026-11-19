//! Linear-interpolation sample-rate conversion for 16-bit stereo Coda WAVs.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be resampled".into());
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

pub fn resample(wav: &[u8], target_rate: u32) -> Result<Vec<u8>, String> {
    if target_rate == 0 || target_rate > 384_000 {
        return Err("target rate must be between 1 and 384000 Hz".into());
    }
    let (source_rate, frames) = decode(wav)?;
    if frames.is_empty() || source_rate == target_rate {
        return Ok(wav_bytes(&frames, target_rate));
    }

    let output_len =
        (frames.len() as f64 * target_rate as f64 / source_rate as f64).round() as usize;
    let ratio = source_rate as f64 / target_rate as f64;
    let mut output = Vec::with_capacity(output_len);
    for index in 0..output_len {
        let position = index as f64 * ratio;
        let lower = position as usize;
        let fraction = position - lower as f64;
        let a = frames[lower.min(frames.len() - 1)];
        let b = frames[(lower + 1).min(frames.len() - 1)];
        output.push((
            a.0 * (1.0 - fraction) + b.0 * fraction,
            a.1 * (1.0 - fraction) + b.1 * fraction,
        ));
    }
    Ok(wav_bytes(&output, target_rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn sample() -> Vec<u8> {
        encode(&[(0.0, 0.0), (1.0, 1.0), (0.0, 0.0), (1.0, 1.0)], 8_000)
    }

    #[test]
    fn same_rate_is_byte_identical() {
        assert_eq!(resample(&sample(), 8_000).unwrap(), sample());
    }

    #[test]
    fn halving_the_rate_halves_the_frame_count() {
        let input = encode(&[(0.0, 0.0), (0.5, 0.5), (1.0, 1.0), (1.0, 1.0)], 8_000);
        let result = resample(&input, 4_000).unwrap();
        let (rate, frames) = decode(&result).unwrap();
        assert_eq!(rate, 4_000);
        assert_eq!(frames.len(), 2);
        assert!((frames[0].0 - 0.0).abs() < 0.01, "{}", frames[0].0);
        assert!((frames[1].0 - 1.0).abs() < 0.01, "{}", frames[1].0);
    }

    #[test]
    fn interpolates_between_frames() {
        let input = encode(&[(0.0, 0.0), (1.0, 1.0)], 1_000);
        let (_, frames) = decode(&resample(&input, 2_000).unwrap()).unwrap();
        assert_eq!(frames.len(), 4);
        assert!((frames[1].0 - 0.5).abs() < 0.01, "{}", frames[1].0);
    }

    #[test]
    fn rejects_bad_rates() {
        assert!(resample(&sample(), 0).is_err());
        assert!(resample(&sample(), 500_000).is_err());
    }
}
