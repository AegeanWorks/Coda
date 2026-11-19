//! Apply the shared feedback delay to an existing 16-bit stereo Coda WAV.

use crate::echo;
use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be echoed".into());
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

/// Add a decaying delayed copy of the signal using the shared comb echo.
pub fn apply_echo_wav(wav: &[u8], delay: f64, feedback: f64) -> Result<Vec<u8>, String> {
    let (rate, mut frames) = decode(wav)?;
    echo::apply_echo(&mut frames, rate, delay, feedback);
    Ok(wav_bytes(&frames, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn impulse() -> Vec<u8> {
        let mut frames = vec![(0.0_f64, 0.0_f64); 12];
        frames[0] = (1.0, 0.5);
        encode(&frames, 8_000)
    }

    #[test]
    fn zero_delay_is_an_identity() {
        assert_eq!(apply_echo_wav(&impulse(), 0.0, 0.5).unwrap(), impulse());
    }

    #[test]
    fn creates_a_decaying_delayed_tail() {
        let out = apply_echo_wav(&impulse(), 0.001, 0.5).unwrap();
        let (rate, frames) = decode(&out).unwrap();
        assert_eq!(rate, 8_000);
        assert_eq!(frames.len(), 12);
        assert!((frames[8].0 - 0.5).abs() < 0.01, "{}", frames[8].0);
        assert!((frames[8].1 - 0.25).abs() < 0.01, "{}", frames[8].1);
        assert!(frames[4].0 == 0.0 && frames[5].0 == 0.0, "no early tail");
        assert!(frames.iter().all(|f| f.0.abs() <= 1.0 && f.1.abs() <= 1.0));
    }

    #[test]
    fn is_deterministic() {
        assert_eq!(
            apply_echo_wav(&impulse(), 0.0005, 0.4).unwrap(),
            apply_echo_wav(&impulse(), 0.0005, 0.4).unwrap()
        );
    }

    #[test]
    fn rejects_bad_input() {
        assert!(apply_echo_wav(b"bad", 0.01, 0.5).is_err());
    }
}
