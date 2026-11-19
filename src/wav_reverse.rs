//! Reverse the frame order of a 16-bit stereo WAV produced by Coda.

use crate::render::{wav_bytes, Frame};

fn read_frames(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be reversed".into());
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

pub fn reverse_wav(wav: &[u8]) -> Result<Vec<u8>, String> {
    let (rate, mut frames) = read_frames(wav)?;
    frames.reverse();
    Ok(wav_bytes(&frames, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode;

    fn sample_wav() -> Vec<u8> {
        encode(&[(0.0, 0.1), (0.5, 0.2), (-0.5, -0.2)], 8_000)
    }

    #[test]
    fn reverses_frame_order() {
        let reversed = reverse_wav(&sample_wav()).unwrap();
        let (_, frames) = read_frames(&reversed).unwrap();
        assert_eq!(frames.len(), 3);
        assert!((frames[0].0 + 0.5).abs() < 1e-4);
        assert!((frames[2].1 - 0.1).abs() < 1e-4);
    }

    #[test]
    fn double_reverse_is_byte_identical() {
        let original = sample_wav();
        assert_eq!(
            reverse_wav(&reverse_wav(&original).unwrap()).unwrap(),
            original
        );
    }

    #[test]
    fn rejects_bad_input() {
        assert!(reverse_wav(b"not a wav").is_err());
        assert!(reverse_wav(&[]).is_err());
    }
}
