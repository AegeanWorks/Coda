//! Peak normalization for 16-bit stereo WAV buffers produced by Coda.

use crate::render::wav_bytes;

fn data_length(wav: &[u8]) -> Result<usize, String> {
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
    Ok(length)
}

pub fn normalize(wav: &[u8], peak: f64) -> Result<Vec<u8>, String> {
    if !peak.is_finite() || !(0.0..=1.0).contains(&peak) || peak == 0.0 {
        return Err("peak must be a finite number between 0 and 1".into());
    }
    let length = data_length(wav)?;
    let rate = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]);
    let mut frames = Vec::with_capacity(length / 4);
    let mut current_peak = 0.0_f64;
    for chunk in wav[44..].as_chunks::<4>().0 {
        let left = i16::from_le_bytes([chunk[0], chunk[1]]) as f64 / 32767.0;
        let right = i16::from_le_bytes([chunk[2], chunk[3]]) as f64 / 32767.0;
        current_peak = current_peak.max(left.abs()).max(right.abs());
        frames.push((left, right));
    }

    if current_peak > 0.0 {
        let gain = peak / current_peak;
        for frame in &mut frames {
            frame.0 = (frame.0 * gain).clamp(-1.0, 1.0);
            frame.1 = (frame.1 * gain).clamp(-1.0, 1.0);
        }
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

    #[test]
    fn raises_the_peak_to_the_target() {
        let input = quiet_wav();
        assert!(peak_of(&input) < 0.1);
        let output = normalize(&input, 0.9).unwrap();
        assert!(
            (peak_of(&output) - 0.9).abs() < 0.001,
            "{}",
            peak_of(&output)
        );
    }

    #[test]
    fn is_idempotent() {
        let once = normalize(&quiet_wav(), 0.8).unwrap();
        let twice = normalize(&once, 0.8).unwrap();
        assert_eq!(once, twice);
    }

    #[test]
    fn silence_is_unchanged() {
        let silent = encode(&[(0.0, 0.0), (0.0, 0.0)], 8_000);
        assert_eq!(normalize(&silent, 0.9).unwrap(), silent);
    }

    #[test]
    fn rejects_bad_inputs() {
        assert!(normalize(b"not a wav", 0.9).is_err());
        assert!(normalize(&quiet_wav(), 0.0).is_err());
        assert!(normalize(&quiet_wav(), 1.5).is_err());
    }
}
