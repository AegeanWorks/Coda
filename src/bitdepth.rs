//! 8-bit unsigned PCM export from 16-bit Coda WAVs.

use crate::render::Frame;

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be converted".into());
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

fn encode_8bit(frames: &[Frame], rate: u32) -> Vec<u8> {
    let total = frames.len() * 2;
    let mut out = Vec::with_capacity(44 + total);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36_u32 + total as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16_u32.to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes()); // PCM
    out.extend_from_slice(&2_u16.to_le_bytes()); // stereo
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes()); // byte rate
    out.extend_from_slice(&2_u16.to_le_bytes()); // block align
    out.extend_from_slice(&8_u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(total as u32).to_le_bytes());
    for (left, right) in frames {
        for sample in [*left, *right] {
            let value = ((sample + 1.0) * 127.5).round().clamp(0.0, 255.0) as u8;
            out.push(value);
        }
    }
    out
}

pub fn convert(wav: &[u8], bits: u16) -> Result<Vec<u8>, String> {
    match bits {
        16 => {
            decode(wav)?;
            Ok(wav.to_vec())
        }
        8 => {
            let (rate, frames) = decode(wav)?;
            Ok(encode_8bit(&frames, rate))
        }
        _ => Err("bits must be 8 or 16".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes;

    fn sample() -> Vec<u8> {
        wav_bytes(&[(-1.0, 0.0), (1.0, 0.5)], 8_000)
    }

    #[test]
    fn sixteen_bits_is_an_identity() {
        assert_eq!(convert(&sample(), 16).unwrap(), sample());
    }

    #[test]
    fn header_describes_8bit_stereo() {
        let out = convert(&sample(), 8).unwrap();
        assert_eq!(&out[0..4], b"RIFF");
        assert_eq!(u16::from_le_bytes([out[22], out[23]]), 2);
        assert_eq!(u16::from_le_bytes([out[34], out[35]]), 8);
        assert_eq!(u16::from_le_bytes([out[32], out[33]]), 2);
        assert_eq!(
            u32::from_le_bytes([out[28], out[29], out[30], out[31]]),
            16_000
        );
        assert_eq!(u32::from_le_bytes([out[40], out[41], out[42], out[43]]), 4);
    }

    #[test]
    fn sample_mapping_is_exact() {
        let out = convert(&sample(), 8).unwrap();
        assert_eq!(&out[44..48], &[0, 128, 255, 191]);
    }

    #[test]
    fn rejects_bad_inputs() {
        assert!(convert(b"bad", 8).is_err());
        assert!(convert(&sample(), 24).is_err());
    }
}
