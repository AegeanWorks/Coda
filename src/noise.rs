//! Deterministic seeded noise overlay for 16-bit stereo Coda WAVs.

use crate::render::{wav_bytes, Frame};

fn decode(wav: &[u8]) -> Result<(u32, Vec<Frame>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can receive noise".into());
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

/// xorshift64* producing a deterministic sample in `[-1, 1]`.
fn next(state: &mut u64) -> f64 {
    let mut x = *state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *state = x;
    let bits = x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11;
    bits as f64 / (1_u64 << 53) as f64 * 2.0 - 1.0
}

/// Add seeded pseudo-random noise scaled by `amount`; amount 0 is byte-identical.
pub fn add_noise(wav: &[u8], seed: u64, amount: f64) -> Result<Vec<u8>, String> {
    if !amount.is_finite() || !(0.0..=1.0).contains(&amount) {
        return Err("amount must be between 0 and 1".into());
    }
    let (rate, frames) = decode(wav)?;
    let mut state = seed ^ 0x9E37_79B9_7F4A_7C15;
    if state == 0 {
        state = 1;
    }
    let out: Vec<Frame> = frames
        .iter()
        .map(|&(l, r)| {
            (
                (l + amount * next(&mut state)).clamp(-1.0, 1.0),
                (r + amount * next(&mut state)).clamp(-1.0, 1.0),
            )
        })
        .collect();
    Ok(wav_bytes(&out, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes as encode_wav;

    fn sample() -> Vec<u8> {
        encode_wav(&[(0.2, -0.2), (0.1, 0.3), (-0.5, 0.4)], 8_000)
    }

    #[test]
    fn zero_amount_is_an_identity() {
        assert_eq!(add_noise(&sample(), 42, 0.0).unwrap(), sample());
    }

    #[test]
    fn same_seed_is_deterministic_and_other_seeds_differ() {
        let a = add_noise(&sample(), 7, 0.3).unwrap();
        assert_eq!(a, add_noise(&sample(), 7, 0.3).unwrap());
        assert_ne!(a, add_noise(&sample(), 8, 0.3).unwrap());
    }

    #[test]
    fn stays_bounded() {
        let out = add_noise(&sample(), 1, 1.0).unwrap();
        let (_, frames) = decode(&out).unwrap();
        assert!(frames.iter().all(|f| f.0.abs() <= 1.0 && f.1.abs() <= 1.0));
    }

    #[test]
    fn rejects_bad_inputs() {
        assert!(add_noise(b"bad", 1, 0.5).is_err());
        assert!(add_noise(&sample(), 1, 1.5).is_err());
    }
}
