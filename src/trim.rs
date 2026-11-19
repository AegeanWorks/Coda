//! Time-range and silence-trimming for 16-bit stereo Coda WAVs.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be trimmed".into());
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

pub fn trim_wav(
    wav: &[u8],
    start: f64,
    end: Option<f64>,
    silence_threshold_dbfs: Option<f64>,
) -> Result<Vec<u8>, String> {
    if !start.is_finite() || start < 0.0 {
        return Err("start must be a non-negative number".into());
    }
    if let Some(end) = end {
        if !end.is_finite() || end < 0.0 || end <= start {
            return Err("end must be greater than start".into());
        }
    }
    if let Some(threshold) = silence_threshold_dbfs {
        if !threshold.is_finite() || threshold > 0.0 || threshold < -100.0 {
            return Err("silence threshold must be between -100 and 0 dBFS".into());
        }
    }

    let (rate, frames) = decode(wav)?;
    let mut from = (start * rate as f64).round() as usize;
    let mut to = end
        .map(|value| (value * rate as f64).round() as usize)
        .unwrap_or(frames.len());

    if let Some(threshold) = silence_threshold_dbfs {
        let linear = 10.0_f64.powf(threshold / 20.0);
        let loud: Vec<usize> = frames
            .iter()
            .enumerate()
            .filter(|(_, f)| f.0.abs().max(f.1.abs()) >= linear)
            .map(|(i, _)| i)
            .collect();
        if let (Some(first), Some(last)) = (loud.first(), loud.last()) {
            from = from.max(*first);
            to = to.min(last + 1);
        } else {
            // Entirely silent: return a single valid silent frame.
            return Ok(wav_bytes(&[(0.0, 0.0)], rate));
        }
    }

    if from >= frames.len() || to > frames.len() || from >= to {
        return Err("trim range is outside the audio".into());
    }
    Ok(wav_bytes(&frames[from..to], rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn sample() -> Vec<u8> {
        encode(&[(0.0, 0.0), (0.5, 0.5), (0.5, 0.5), (0.0, 0.0)], 8_000)
    }

    #[test]
    fn trims_an_exact_time_range() {
        let result = trim_wav(&sample(), 0.000125, Some(0.000375), None).unwrap();
        let (_, frames) = decode(&result).unwrap();
        assert_eq!(frames.len(), 2);
        assert!((frames[0].0 - 0.5).abs() < 1e-4);
    }

    #[test]
    fn trims_leading_and_trailing_silence() {
        let result = trim_wav(&sample(), 0.0, None, Some(-60.0)).unwrap();
        let (_, frames) = decode(&result).unwrap();
        assert_eq!(frames.len(), 2);
        assert!(frames.iter().all(|f| (f.0 - 0.5).abs() < 1e-4));
    }

    #[test]
    fn no_op_range_is_byte_identical() {
        let input = sample();
        assert_eq!(trim_wav(&input, 0.0, None, None).unwrap(), input);
    }

    #[test]
    fn all_silent_stays_valid() {
        let silent = encode(&[(0.0, 0.0); 4], 8_000);
        let result = trim_wav(&silent, 0.0, None, Some(-60.0)).unwrap();
        let (_, frames) = decode(&result).unwrap();
        assert_eq!(frames.len(), 1);
    }

    #[test]
    fn rejects_bad_ranges() {
        let input = sample();
        assert!(trim_wav(&input, 1.0, None, None).is_err());
        assert!(trim_wav(&input, 0.0, Some(0.0), None).is_err());
        assert!(trim_wav(&input, 0.0, None, Some(2.0)).is_err());
    }
}
