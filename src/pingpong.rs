//! Ping-pong delay: echoes bouncing alternately between stereo channels.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be delayed".into());
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

/// Feed each channel's delay line into the opposite channel so echoes bounce.
/// Zero delay is byte-identical; feedback is clamped below 1 to stay bounded.
pub fn apply_pingpong(wav: &[u8], delay: f64, feedback: f64) -> Result<Vec<u8>, String> {
    if !delay.is_finite() || delay < 0.0 {
        return Err("delay must be a non-negative number".into());
    }
    if !feedback.is_finite() || !(0.0..=1.0).contains(&feedback) {
        return Err("feedback must be between 0 and 1".into());
    }
    let (rate, frames) = decode(wav)?;
    let d = (delay * rate as f64).round() as usize;
    if frames.is_empty() || d == 0 {
        return Ok(wav_bytes(&frames, rate));
    }
    let gain = feedback.min(0.95);
    let mut left_line = vec![0.0_f64; d];
    let mut right_line = vec![0.0_f64; d];
    let out: Vec<Frame> = frames
        .iter()
        .enumerate()
        .map(|(index, &(l, r))| {
            let slot = index % d;
            let from_right = right_line[slot];
            let from_left = left_line[slot];
            let echo_l = (l + gain * from_right).clamp(-1.0, 1.0);
            let echo_r = (r + gain * from_left).clamp(-1.0, 1.0);
            left_line[slot] = echo_l;
            right_line[slot] = echo_r;
            (echo_l, echo_r)
        })
        .collect();
    Ok(wav_bytes(&out, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn impulse(channel: usize) -> Vec<u8> {
        let mut frames = vec![(0.0_f64, 0.0_f64); 16];
        frames[0] = if channel == 0 { (1.0, 0.0) } else { (0.0, 1.0) };
        encode(&frames, 8_000)
    }

    #[test]
    fn zero_delay_is_an_identity() {
        let wav = impulse(0);
        assert_eq!(apply_pingpong(&wav, 0.0, 0.5).unwrap(), wav);
    }

    #[test]
    fn bounces_echoes_across_channels() {
        let out = apply_pingpong(&impulse(0), 0.000375, 0.5).unwrap(); // D = 3
        let (_, frames) = decode(&out).unwrap();
        assert!(frames[0].0 > 0.9, "original on left");
        assert!(frames[3].1 > 0.4, "first echo on right");
        assert!(frames[6].0 > 0.2, "second echo back on left");
        assert!(frames[9].1 > 0.1, "third echo on right");
    }

    #[test]
    fn is_bounded_and_deterministic() {
        let out = apply_pingpong(&impulse(0), 0.000125, 1.0).unwrap();
        let (_, frames) = decode(&out).unwrap();
        assert!(frames.iter().all(|f| f.0.abs() <= 1.0 && f.1.abs() <= 1.0));
        assert_eq!(out, apply_pingpong(&impulse(0), 0.000125, 1.0).unwrap());
    }

    #[test]
    fn rejects_bad_inputs() {
        let wav = impulse(0);
        assert!(apply_pingpong(b"bad", 0.001, 0.5).is_err());
        assert!(apply_pingpong(&wav, -1.0, 0.5).is_err());
        assert!(apply_pingpong(&wav, 0.001, 1.5).is_err());
    }
}
