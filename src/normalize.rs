//! Peak- and RMS-targeted normalization for 16-bit stereo Coda WAVs.

use crate::render::wav_bytes;

type Frames = Vec<(f64, f64)>;

fn decode(wav: &[u8]) -> Result<(u32, Frames), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be normalized".into());
    }
    let length = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]) as usize;
    if wav.len() != 44 + length || !length.is_multiple_of(4) {
        return Err("WAV data length does not match the file size".into());
    }
    let rate = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]);
    let frames: Frames = wav[44..]
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

fn scale(frames: &mut Frames, gain: f64) {
    for frame in frames {
        frame.0 = (frame.0 * gain).clamp(-1.0, 1.0);
        frame.1 = (frame.1 * gain).clamp(-1.0, 1.0);
    }
}

pub fn normalize(wav: &[u8], peak: f64) -> Result<Vec<u8>, String> {
    if !peak.is_finite() || !(0.0..=1.0).contains(&peak) || peak == 0.0 {
        return Err("peak must be a finite number between 0 and 1".into());
    }
    let (rate, mut frames) = decode(wav)?;
    let current = frames
        .iter()
        .map(|f| f.0.abs().max(f.1.abs()))
        .fold(0.0_f64, f64::max);
    if current > 0.0 {
        scale(&mut frames, peak / current);
    }
    Ok(wav_bytes(&frames, rate))
}

/// Scale the signal so its RMS matches `target_dbfs`. Silence is unchanged.
pub fn normalize_rms(wav: &[u8], target_dbfs: f64) -> Result<Vec<u8>, String> {
    if !target_dbfs.is_finite() || target_dbfs > 0.0 || target_dbfs < -100.0 {
        return Err("RMS target must be between -100 and 0 dBFS".into());
    }
    let (rate, mut frames) = decode(wav)?;
    let sum: f64 = frames.iter().map(|f| f.0 * f.0 + f.1 * f.1).sum();
    let rms = if frames.is_empty() {
        0.0
    } else {
        (sum / (frames.len() * 2) as f64).sqrt()
    };
    if rms > 0.0 {
        let target = 10.0_f64.powf(target_dbfs / 20.0);
        scale(&mut frames, target / rms);
    }
    Ok(wav_bytes(&frames, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{render, wav_bytes as encode};
    use crate::score;

    fn quiet_wav() -> Vec<u8> {
        let score = score::parse("gain 0.05\nnote A4 1\n").unwrap();
        encode(&render(&score, 8_000), 8_000)
    }

    fn peak_of(wav: &[u8]) -> f64 {
        wav[44..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| (i16::from_le_bytes([c[0], c[1]]) as f64 / 32767.0).abs())
            .fold(0.0_f64, f64::max)
    }

    fn rms_of(wav: &[u8]) -> f64 {
        let (_, frames) = decode(wav).unwrap();
        let sum: f64 = frames.iter().map(|f| f.0 * f.0 + f.1 * f.1).sum();
        (sum / (frames.len() * 2) as f64).sqrt()
    }

    #[test]
    fn raises_the_peak_to_the_target() {
        let input = quiet_wav();
        assert!(peak_of(&input) < 0.1);
        let output = normalize(&input, 0.9).unwrap();
        assert!((peak_of(&output) - 0.9).abs() < 0.001);
    }

    #[test]
    fn rms_target_is_reached() {
        let output = normalize_rms(&encode(&[(0.5, 0.5); 8], 8_000), -12.0).unwrap();
        let dbfs = 20.0 * rms_of(&output).log10();
        assert!((dbfs + 12.0).abs() < 0.05, "{dbfs}");
    }

    #[test]
    fn rms_is_idempotent_and_silence_is_unchanged() {
        let once = normalize_rms(&quiet_wav(), -18.0).unwrap();
        assert_eq!(normalize_rms(&once, -18.0).unwrap(), once);
        let silent = encode(&[(0.0, 0.0); 4], 8_000);
        assert_eq!(normalize_rms(&silent, -18.0).unwrap(), silent);
    }

    #[test]
    fn rejects_bad_inputs() {
        assert!(normalize(b"not a wav", 0.9).is_err());
        assert!(normalize(&quiet_wav(), 1.5).is_err());
        assert!(normalize_rms(&quiet_wav(), 2.0).is_err());
    }
}
