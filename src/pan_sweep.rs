//! Apply the shared equal-power auto-pan LFO to an existing Coda WAV.

use crate::autopan;
use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be auto-panned".into());
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

/// Sweep the stereo image with a deterministic sine LFO.
pub fn pan_sweep(wav: &[u8], rate: f64, depth: f64) -> Result<Vec<u8>, String> {
    let (sample_rate, mut frames) = decode(wav)?;
    autopan::apply_autopan(&mut frames, sample_rate, rate, depth);
    Ok(wav_bytes(&frames, sample_rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn sample() -> Vec<u8> {
        encode(&[(0.5, -0.5); 64], 8_000)
    }

    #[test]
    fn zero_depth_is_an_identity() {
        assert_eq!(pan_sweep(&sample(), 2.0, 0.0).unwrap(), sample());
    }

    #[test]
    fn sweeps_the_balance_and_stays_bounded() {
        let out = pan_sweep(&sample(), 100.0, 1.0).unwrap();
        let (_, frames) = decode(&out).unwrap();
        assert_eq!(frames.len(), 64);
        assert!(frames.iter().any(|f| f.0 > f.1.abs()), "sweeps left");
        assert!(frames.iter().any(|f| f.1.abs() > f.0), "sweeps right");
        assert!(frames.iter().all(|f| f.0.abs() <= 1.0 && f.1.abs() <= 1.0));
    }

    #[test]
    fn is_deterministic() {
        assert_eq!(
            pan_sweep(&sample(), 3.5, 0.7).unwrap(),
            pan_sweep(&sample(), 3.5, 0.7).unwrap()
        );
    }

    #[test]
    fn rejects_bad_input() {
        assert!(pan_sweep(b"bad", 2.0, 0.5).is_err());
    }
}
