//! Concatenate Coda-rendered WAV buffers into one file.
//!
//! Only 16-bit stereo PCM produced by this crate is accepted; the RIFF header
//! is rebuilt with the summed data length and the original sample rate.

fn data_length(wav: &[u8]) -> Result<usize, String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be joined".into());
    }
    let length = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]) as usize;
    if wav.len() != 44 + length {
        return Err("WAV data length does not match the file size".into());
    }
    Ok(length)
}

fn sample_rate(wav: &[u8]) -> u32 {
    u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]])
}

pub fn join_wavs(parts: &[Vec<u8>]) -> Result<Vec<u8>, String> {
    let first = parts
        .first()
        .ok_or_else(|| "no WAV parts to join".to_string())?;
    data_length(first)?;
    let rate = sample_rate(first);
    for part in parts {
        data_length(part)?;
        if sample_rate(part) != rate {
            return Err(format!(
                "sample-rate mismatch: {} Hz vs {} Hz",
                sample_rate(part),
                rate
            ));
        }
    }

    let total: usize = parts.iter().map(|p| data_length(p).unwrap_or(0)).sum();
    let mut out = Vec::with_capacity(44 + total);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36_u32 + total as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16_u32.to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes()); // PCM
    out.extend_from_slice(&2_u16.to_le_bytes()); // stereo
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 4).to_le_bytes());
    out.extend_from_slice(&4_u16.to_le_bytes()); // block align
    out.extend_from_slice(&16_u16.to_le_bytes()); // bits
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(total as u32).to_le_bytes());
    for part in parts {
        out.extend_from_slice(&part[44..]);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{self, wav_bytes};
    use crate::score;

    fn wav(source: &str, rate: u32) -> Vec<u8> {
        let parsed = score::parse(source).unwrap();
        wav_bytes(&render::render(&parsed, rate), rate)
    }

    #[test]
    fn header_and_data_are_concatenated() {
        let a = wav("note C4 1\n", 8_000);
        let b = wav("note E4 1\n", 8_000);
        let joined = join_wavs(&[a.clone(), b.clone()]).unwrap();
        assert_eq!(&joined[0..4], b"RIFF");
        assert_eq!(&joined[36..40], b"data");
        let expected_len = (a.len() - 44) + (b.len() - 44);
        assert_eq!(
            u32::from_le_bytes([joined[40], joined[41], joined[42], joined[43]]) as usize,
            expected_len
        );
        assert_eq!(joined.len(), 44 + expected_len);
        assert_eq!(&joined[44..44 + a.len() - 44], &a[44..]);
        assert_eq!(&joined[44 + a.len() - 44..], &b[44..]);
    }

    #[test]
    fn single_part_is_unchanged() {
        let a = wav("note C4 1\n", 8_000);
        assert_eq!(join_wavs(std::slice::from_ref(&a)).unwrap(), a);
    }

    #[test]
    fn rejects_mixed_sample_rates() {
        let a = wav("note C4 1\n", 8_000);
        let b = wav("note E4 1\n", 16_000);
        assert!(join_wavs(&[a, b])
            .unwrap_err()
            .contains("sample-rate mismatch"));
    }

    #[test]
    fn rejects_non_wav_bytes() {
        assert!(join_wavs(&[b"not a wav".to_vec()]).is_err());
        assert!(join_wavs(&[]).is_err());
    }
}
