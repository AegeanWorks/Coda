//! Read-only measurements of a 16-bit stereo Coda WAV.

fn read_frames(wav: &[u8]) -> Result<(u32, Vec<(f64, f64)>), String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be analyzed".into());
    }
    let length = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]) as usize;
    if wav.len() != 44 + length || !length.is_multiple_of(4) {
        return Err("WAV data length does not match the file size".into());
    }
    let rate = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]);
    let frames: Vec<(f64, f64)> = wav[44..]
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

fn dbfs(value: f64) -> String {
    if value <= 0.0 {
        "-inf dBFS".to_string()
    } else {
        format!("{:.2} dBFS", 20.0 * value.log10())
    }
}

pub fn analyze(wav: &[u8]) -> Result<String, String> {
    let (rate, frames) = read_frames(wav)?;
    let sum: f64 = frames.iter().map(|f| f.0 * f.0 + f.1 * f.1).sum();
    let rms = if frames.is_empty() {
        0.0
    } else {
        (sum / (frames.len() * 2) as f64).sqrt()
    };
    let peak = frames
        .iter()
        .map(|f| f.0.abs().max(f.1.abs()))
        .fold(0.0_f64, f64::max);
    let seconds = frames.len() as f64 / rate as f64;

    Ok(format!(
        "frames      {}\n\
         duration    {seconds:.3}s\n\
         sample rate {rate}\n\
         peak        {peak:.6} ({})\n\
         rms         {rms:.6} ({})\n",
        frames.len(),
        dbfs(peak),
        dbfs(rms)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes;

    #[test]
    fn known_constant_amplitude() {
        let wav = wav_bytes(&[(0.5, 0.5); 8], 8_000);
        assert_eq!(
            analyze(&wav).unwrap(),
            concat!(
                "frames      8\n",
                "duration    0.001s\n",
                "sample rate 8000\n",
                "peak        0.500015 (-6.02 dBFS)\n",
                "rms         0.500015 (-6.02 dBFS)\n",
            )
        );
    }

    #[test]
    fn silence_is_finite() {
        let wav = wav_bytes(&[(0.0, 0.0); 4], 8_000);
        let report = analyze(&wav).unwrap();
        assert!(report.contains("peak        0.000000 (-inf dBFS)"));
        assert!(report.contains("rms         0.000000 (-inf dBFS)"));
    }

    #[test]
    fn rejects_bad_input() {
        assert!(analyze(b"not a wav").is_err());
        assert!(analyze(&[]).is_err());
    }
}
