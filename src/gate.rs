//! Downward noise gate: silence quiet frames of a 16-bit stereo Coda WAV.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be gated".into());
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

/// Silence any frame whose peak amplitude is below `threshold_db` (dBFS).
/// Frames at or above the threshold pass through unchanged.
pub fn apply_gate(wav: &[u8], threshold_db: f64) -> Result<Vec<u8>, String> {
    if !threshold_db.is_finite() || !(-100.0..=0.0).contains(&threshold_db) {
        return Err("threshold must be between -100 and 0 dBFS".into());
    }
    let (rate, mut frames) = decode(wav)?;
    let threshold = 10.0_f64.powf(threshold_db / 20.0);
    for frame in &mut frames {
        if frame.0.abs().max(frame.1.abs()) < threshold {
            frame.0 = 0.0;
            frame.1 = 0.0;
        }
    }
    Ok(wav_bytes(&frames, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn mixed() -> Vec<u8> {
        encode(&[(0.8, -0.6), (0.001, 0.0005), (0.5, 0.4)], 8_000)
    }

    #[test]
    fn silences_quiet_frames_only() {
        let out = apply_gate(&mixed(), -40.0).unwrap();
        let (_, frames) = decode(&out).unwrap();
        assert_eq!(frames.len(), 3);
        let (_, source) = decode(&mixed()).unwrap();
        assert_eq!(frames[0], source[0]);
        assert_eq!(frames[2], source[2]);
        assert_eq!(frames[1], (0.0, 0.0));
    }

    #[test]
    fn faint_threshold_is_an_identity() {
        assert_eq!(apply_gate(&mixed(), -100.0).unwrap(), mixed());
    }

    #[test]
    fn zero_threshold_silences_all_non_full_scale_frames() {
        let out = apply_gate(&mixed(), 0.0).unwrap();
        let (_, frames) = decode(&out).unwrap();
        assert!(frames.iter().all(|f| f.0 == 0.0 && f.1 == 0.0));
    }

    #[test]
    fn rejects_bad_inputs() {
        assert!(apply_gate(b"bad", -40.0).is_err());
        assert!(apply_gate(&mixed(), 1.0).is_err());
        assert!(apply_gate(&mixed(), -101.0).is_err());
    }
}
