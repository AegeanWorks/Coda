//! Render the rolling peak amplitude envelope of a Coda WAV as audio.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be enveloped".into());
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

/// Replace each frame with the peak magnitude over the trailing `window` frames.
/// A window of 1 is byte-identical; length and rate are preserved.
pub fn envelope_wav(wav: &[u8], window: usize) -> Result<Vec<u8>, String> {
    if window == 0 {
        return Err("window must be at least 1".into());
    }
    let (rate, frames) = decode(wav)?;
    if window == 1 || frames.is_empty() {
        return Ok(wav_bytes(&frames, rate));
    }
    let out: Vec<Frame> = (0..frames.len())
        .map(|index| {
            let from = index.saturating_sub(window - 1);
            let mut peak = (0.0_f64, 0.0_f64);
            for frame in &frames[from..=index] {
                peak.0 = peak.0.max(frame.0.abs());
                peak.1 = peak.1.max(frame.1.abs());
            }
            peak
        })
        .collect();
    Ok(wav_bytes(&out, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    #[test]
    fn window_one_is_an_identity() {
        let wav = encode(&[(0.2, -0.3), (0.8, 0.1)], 8_000);
        assert_eq!(envelope_wav(&wav, 1).unwrap(), wav);
    }

    #[test]
    fn builds_a_causal_peak_contour() {
        let wav = encode(&[(0.0, 0.0), (0.5, 0.5), (0.2, 0.2), (1.0, 1.0)], 8_000);
        let (_, frames) = decode(&envelope_wav(&wav, 2).unwrap()).unwrap();
        let values: Vec<f64> = frames.iter().map(|f| f.0).collect();
        assert!(values[0].abs() < 1e-4);
        assert!((values[1] - 0.5).abs() < 1e-4, "{}", values[1]);
        assert!((values[2] - 0.5).abs() < 1e-4, "{}", values[2]);
        assert!((values[3] - 1.0).abs() < 1e-4, "{}", values[3]);
    }

    #[test]
    fn silence_stays_silent_and_output_is_bounded() {
        let wav = encode(&[(0.0, 0.0); 4], 8_000);
        let (_, frames) = decode(&envelope_wav(&wav, 4).unwrap()).unwrap();
        assert!(frames.iter().all(|f| f.0 == 0.0 && f.1 == 0.0));
        let loud = encode(&[(1.0, -1.0); 4], 8_000);
        let (_, frames) = decode(&envelope_wav(&loud, 4).unwrap()).unwrap();
        assert!(frames.iter().all(|f| f.0.abs() <= 1.0 && f.1.abs() <= 1.0));
    }

    #[test]
    fn rejects_bad_inputs() {
        let wav = encode(&[(0.5, 0.5)], 8_000);
        assert!(envelope_wav(b"bad", 2).is_err());
        assert!(envelope_wav(&wav, 0).is_err());
    }
}
