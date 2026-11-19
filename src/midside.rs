//! Mid/side stereo encoding and decoding for Coda WAVs.

use crate::render::{wav_bytes, Frame};

const ROOT2: f64 = std::f64::consts::SQRT_2;

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be transformed".into());
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

/// Encode stereo to mid/side, or decode mid/side back to stereo.
pub fn midside(wav: &[u8], mode: &str) -> Result<Vec<u8>, String> {
    match mode {
        "encode" | "decode" => {}
        _ => return Err(format!("unknown mid/side mode '{mode}'")),
    }
    let (rate, frames) = decode(wav)?;
    // With √2 normalization the transform is its own inverse, so both modes
    // share the same butterfly; `mode` documents the caller's intent.
    let out: Vec<Frame> = frames
        .iter()
        .map(|&(a, b)| {
            (
                ((a + b) / ROOT2).clamp(-1.0, 1.0),
                ((a - b) / ROOT2).clamp(-1.0, 1.0),
            )
        })
        .collect();
    Ok(wav_bytes(&out, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode_wav;

    #[test]
    fn mono_encodes_to_zero_side() {
        let wav = encode_wav(&[(0.5, 0.5), (-0.3, -0.3)], 8_000);
        let (_, frames) = decode(&midside(&wav, "encode").unwrap()).unwrap();
        assert!(frames.iter().all(|f| f.1.abs() < 1e-4), "side must be zero");
    }

    #[test]
    fn mid_only_decodes_to_dual_mono() {
        let wav = encode_wav(&[(0.6, 0.0), (-0.4, 0.0)], 8_000);
        let (_, frames) = decode(&midside(&wav, "decode").unwrap()).unwrap();
        for frame in frames {
            assert!((frame.0 - frame.1).abs() < 1e-4, "{:?}", frame);
        }
    }

    #[test]
    fn roundtrip_is_within_quantization() {
        let wav = encode_wav(&[(0.5, -0.2), (0.1, 0.9), (-0.7, 0.3)], 8_000);
        let encoded = midside(&wav, "encode").unwrap();
        let decoded = midside(&encoded, "decode").unwrap();
        let (_, source) = decode(&wav).unwrap();
        let (_, result) = decode(&decoded).unwrap();
        for (before, after) in source.iter().zip(result.iter()) {
            assert!((before.0 - after.0).abs() < 1e-4);
            assert!((before.1 - after.1).abs() < 1e-4);
        }
    }

    #[test]
    fn rejects_bad_mode_and_input() {
        assert!(midside(b"bad", "encode").is_err());
        assert!(midside(&encode_wav(&[(0.5, 0.5)], 8_000), "sideways").is_err());
    }
}
