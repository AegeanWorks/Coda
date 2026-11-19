//! Read-only inspection of the MIDI subset Coda exports and imports.

use crate::midi_import;
use crate::notes;

pub fn midi_info(bytes: &[u8]) -> Result<String, String> {
    if bytes.len() < 14 || &bytes[0..4] != b"MThd" {
        return Err("not a Standard MIDI File".into());
    }
    let header_length = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    if header_length < 6 || bytes.len() < 8 + header_length as usize {
        return Err("truncated MIDI header".into());
    }
    let format = u16::from_be_bytes([bytes[8], bytes[9]]);
    let tracks = u16::from_be_bytes([bytes[10], bytes[11]]);
    let division = u16::from_be_bytes([bytes[12], bytes[13]]);
    let score = midi_import::import_midi(bytes)?;

    let sounding: Vec<&crate::score::Note> =
        score.notes.iter().filter(|n| n.pitch != "-").collect();
    let duration = score
        .notes
        .iter()
        .map(|n| n.start + n.beats)
        .fold(0.0_f64, f64::max);
    let numbers: Vec<i32> = sounding
        .iter()
        .filter_map(|n| notes::midi(&n.pitch))
        .collect();
    let low = numbers
        .iter()
        .min()
        .copied()
        .map(notes::name)
        .unwrap_or_else(|| "-".into());
    let high = numbers
        .iter()
        .max()
        .copied()
        .map(notes::name)
        .unwrap_or_else(|| "-".into());

    Ok(format!(
        "format      {format}\n\
         tracks      {tracks}\n\
         division    {division}\n\
         tempo       {:.2}\n\
         notes       {}\n\
         rests       {}\n\
         duration    {duration:.3} beats\n\
         pitch range {low}..{high}\n",
        score.tempo,
        sounding.len(),
        score.notes.len() - sounding.len(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi::midi_bytes;
    use crate::score;

    #[test]
    fn describes_a_generated_midi_file() {
        let source = score::parse("tempo 120\nnote C4 1\nnote E4 1\nnote G4 2\n").unwrap();
        let bytes = midi_bytes(&source);
        let report = midi_info(&bytes).unwrap();
        assert!(report.contains("format      0"), "{report}");
        assert!(report.contains("tracks      1"), "{report}");
        assert!(report.contains("notes       3"), "{report}");
        assert!(report.contains("rests       0"), "{report}");
        assert!(report.contains("duration    4.000 beats"), "{report}");
        assert!(report.contains("pitch range C4..G4"), "{report}");
    }

    #[test]
    fn rejects_bad_input() {
        assert!(midi_info(b"bad").is_err());
        assert!(midi_info(&[]).is_err());
    }
}
