//! Repeat a Coda WAV a whole number of times.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be repeated".into());
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

/// Append the source `times` times. `times = 1` is byte-identical.
pub fn repeat_wav(wav: &[u8], times: usize) -> Result<Vec<u8>, String> {
    if times == 0 || times > 1_000 {
        return Err("times must be between 1 and 1000".into());
    }
    let (rate, frames) = decode(wav)?;
    let mut out = Vec::with_capacity(frames.len() * times);
    for _ in 0..times {
        out.extend_from_slice(&frames);
    }
    Ok(wav_bytes(&out, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn sample() -> Vec<u8> {
        encode(&[(0.2, -0.2), (0.5, 0.5)], 8_000)
    }

    #[test]
    fn one_repeat_is_an_identity() {
        assert_eq!(repeat_wav(&sample(), 1).unwrap(), sample());
    }

    #[test]
    fn tiles_exactly_without_changing_bytes() {
        let out = repeat_wav(&sample(), 3).unwrap();
        let (rate, frames) = decode(&out).unwrap();
        assert_eq!(rate, 8_000);
        assert_eq!(frames.len(), 6);
        let (_, source) = decode(&sample()).unwrap();
        assert_eq!(&frames[0..2], &source[..]);
        assert_eq!(&frames[2..4], &source[..]);
        assert_eq!(&frames[4..6], &source[..]);
    }

    #[test]
    fn rejects_bad_counts_and_input() {
        assert!(repeat_wav(b"bad", 2).is_err());
        assert!(repeat_wav(&sample(), 0).is_err());
        assert!(repeat_wav(&sample(), 1_001).is_err());
    }
}
