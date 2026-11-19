//! A small provenance chunk appended to Coda WAV files.

fn validate(wav: &[u8]) -> Result<usize, String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    let data = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]) as usize;
    if wav.len() < 44 + data {
        return Err("WAV data length exceeds the file size".into());
    }
    Ok(44 + data)
}

pub fn stamp(wav: &[u8], text: &str) -> Result<Vec<u8>, String> {
    let audio_end = validate(wav)?;
    let padded = text.len() + (text.len() % 2);
    let mut out = Vec::with_capacity(audio_end + 8 + padded);
    out.extend_from_slice(&wav[..audio_end]);
    out.extend_from_slice(b"CODA");
    out.extend_from_slice(&(text.len() as u32).to_le_bytes());
    out.extend_from_slice(text.as_bytes());
    if text.len() % 2 == 1 {
        out.push(0);
    }
    let riff_size = (out.len() - 8) as u32;
    out[4..8].copy_from_slice(&riff_size.to_le_bytes());
    Ok(out)
}

pub fn read(wav: &[u8]) -> Result<Option<String>, String> {
    validate(wav)?;
    let mut pos = 12_usize;
    while pos + 8 <= wav.len() {
        let id = &wav[pos..pos + 4];
        let size =
            u32::from_le_bytes([wav[pos + 4], wav[pos + 5], wav[pos + 6], wav[pos + 7]]) as usize;
        let start = pos + 8;
        let end = start.saturating_add(size);
        if end > wav.len() {
            return Err("WAV chunk exceeds the file size".into());
        }
        if id == b"CODA" {
            let text = String::from_utf8_lossy(&wav[start..end]);
            return Ok(Some(text.trim_end_matches('\0').to_string()));
        }
        pos = start + size + (size % 2);
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes;

    fn sample() -> Vec<u8> {
        wav_bytes(&[(0.5, 0.5); 8], 8_000)
    }

    #[test]
    fn roundtrips_a_payload() {
        let stamped = stamp(&sample(), "digest:abc\nnotes:5\n").unwrap();
        assert_eq!(
            read(&stamped).unwrap().as_deref(),
            Some("digest:abc\nnotes:5\n")
        );
    }

    #[test]
    fn keeps_audio_bytes_and_updates_riff_size() {
        let original = sample();
        let stamped = stamp(&original, "x").unwrap();
        // Only the RIFF size field changes; everything from offset 8 is identical.
        assert_eq!(&stamped[8..original.len()], &original[8..]);
        assert_eq!(&stamped[original.len()..original.len() + 4], b"CODA");
        assert_eq!(
            u32::from_le_bytes([stamped[4], stamped[5], stamped[6], stamped[7]]) as usize,
            stamped.len() - 8
        );
        assert_eq!(stamped.len(), original.len() + 10);
    }

    #[test]
    fn missing_chunk_is_none() {
        assert_eq!(read(&sample()).unwrap(), None);
    }

    #[test]
    fn rejects_bad_input() {
        assert!(stamp(b"bad", "x").is_err());
        assert!(read(b"bad").is_err());
    }
}
