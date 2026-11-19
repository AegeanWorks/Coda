//! Decibel-based gain for 16-bit stereo Coda WAVs.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be gained".into());
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

pub fn change_gain(wav: &[u8], db: f64) -> Result<Vec<u8>, String> {
    if !db.is_finite() || !(-100.0..=6.0).contains(&db) {
        return Err("gain must be between -100 and +6 dB".into());
    }
    let (rate, mut frames) = decode(wav)?;
    let factor = 10.0_f64.powf(db / 20.0);
    for frame in &mut frames {
        frame.0 = (frame.0 * factor).clamp(-1.0, 1.0);
        frame.1 = (frame.1 * factor).clamp(-1.0, 1.0);
    }
    Ok(wav_bytes(&frames, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn sample() -> Vec<u8> {
        encode(&[(0.8, -0.4); 16], 8_000)
    }

    #[test]
    fn zero_db_is_identity() {
        assert_eq!(change_gain(&sample(), 0.0).unwrap(), sample());
    }

    #[test]
    fn minus_six_db_halves_amplitude() {
        let (_, frames) = decode(&change_gain(&sample(), -6.0206).unwrap()).unwrap();
        assert!((frames[0].0 - 0.4).abs() < 0.01, "{}", frames[0].0);
        assert!((frames[0].1 + 0.2).abs() < 0.01, "{}", frames[0].1);
    }

    #[test]
    fn positive_gain_clamps() {
        let (_, frames) = decode(&change_gain(&sample(), 6.0).unwrap()).unwrap();
        assert!(frames.iter().all(|f| f.0.abs() <= 1.0 && f.1.abs() <= 1.0));
    }

    #[test]
    fn rejects_bad_inputs() {
        assert!(change_gain(b"bad", 0.0).is_err());
        assert!(change_gain(&sample(), 20.0).is_err());
    }
}
