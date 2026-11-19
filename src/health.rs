//! Health diagnostics for a 16-bit stereo Coda WAV.

use crate::render::Frame;

fn read_frames(wav: &[u8]) -> Result<Vec<Frame>, String> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data"
    {
        return Err("not a 16-bit PCM WAV produced by Coda".into());
    }
    if u16::from_le_bytes([wav[22], wav[23]]) != 2 || u16::from_le_bytes([wav[34], wav[35]]) != 16 {
        return Err("only 16-bit stereo WAV can be diagnosed".into());
    }
    let length = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]) as usize;
    if wav.len() != 44 + length || !length.is_multiple_of(4) {
        return Err("WAV data length does not match the file size".into());
    }
    Ok(wav[44..]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| {
            (
                i16::from_le_bytes([c[0], c[1]]) as f64 / 32767.0,
                i16::from_le_bytes([c[2], c[3]]) as f64 / 32767.0,
            )
        })
        .collect())
}

fn crossings(frames: &[Frame], channel: usize) -> usize {
    let mut last = 0.0_f64;
    let mut count = 0;
    for frame in frames {
        let value = if channel == 0 { frame.0 } else { frame.1 };
        if value == 0.0 {
            continue;
        }
        if last != 0.0 && (value > 0.0) != (last > 0.0) {
            count += 1;
        }
        last = value;
    }
    count
}

fn crest(peak: f64, rms: f64) -> String {
    if peak <= 0.0 || rms <= 0.0 {
        "-inf dB".to_string()
    } else {
        format!("{:.2} dB", 20.0 * (peak / rms).log10())
    }
}

/// Print a deterministic health report: silence, clipping, DC, crossings, crest.
pub fn health(wav: &[u8]) -> Result<String, String> {
    let frames = read_frames(wav)?;
    let count = frames.len();
    let silent = frames.iter().filter(|f| f.0 == 0.0 && f.1 == 0.0).count();
    let clipped = frames
        .iter()
        .map(|f| (f.0.abs() >= 1.0) as usize + (f.1.abs() >= 1.0) as usize)
        .sum::<usize>();
    let (left_sum, right_sum) = frames
        .iter()
        .fold((0.0_f64, 0.0_f64), |(l, r), f| (l + f.0, r + f.1));
    let (left_dc, right_dc) = if count == 0 {
        (0.0, 0.0)
    } else {
        (left_sum / count as f64, right_sum / count as f64)
    };
    let energy = frames.iter().map(|f| f.0 * f.0 + f.1 * f.1).sum::<f64>();
    let rms = if count == 0 {
        0.0
    } else {
        (energy / (2 * count) as f64).sqrt()
    };
    let peak = frames
        .iter()
        .map(|f| f.0.abs().max(f.1.abs()))
        .fold(0.0_f64, f64::max);

    Ok(format!(
        "frames          {count}\n\
         silent frames   {silent}\n\
         clipped samples {clipped}\n\
         zero crossings  {}/{}\n\
         dc offset       {left_dc:.6}/{right_dc:.6}\n\
         crest factor    {}\n",
        crossings(&frames, 0),
        crossings(&frames, 1),
        crest(peak, rms)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes;

    #[test]
    fn pins_the_full_report_for_a_small_signal() {
        let wav = wav_bytes(&[(0.5, 0.5), (-0.5, -0.5), (0.5, 0.5), (0.0, 0.0)], 8_000);
        let report = health(&wav).unwrap();
        assert!(report.contains("frames          4"), "{report}");
        assert!(report.contains("silent frames   1"), "{report}");
        assert!(report.contains("clipped samples 0"), "{report}");
        assert!(report.contains("zero crossings  2/2"), "{report}");
        assert!(
            report.contains("dc offset       0.125004/0.125004"),
            "{report}"
        );
        assert!(report.contains("crest factor    1.25 dB"), "{report}");
    }

    #[test]
    fn counts_clipping_and_silence() {
        let wav = wav_bytes(&[(1.0, -1.0), (0.0, 0.0)], 8_000);
        let report = health(&wav).unwrap();
        assert!(report.contains("clipped samples 2"), "{report}");
        assert!(report.contains("silent frames   1"), "{report}");
        assert!(report.contains("crest factor    3.01 dB"), "{report}");
    }

    #[test]
    fn silence_has_finite_crest() {
        let wav = wav_bytes(&[(0.0, 0.0); 4], 8_000);
        let report = health(&wav).unwrap();
        assert!(report.contains("silent frames   4"), "{report}");
        assert!(report.contains("crest factor    -inf dB"), "{report}");
    }

    #[test]
    fn rejects_bad_input() {
        assert!(health(b"bad").is_err());
    }
}
