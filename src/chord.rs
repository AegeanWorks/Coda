//! Insert a named-quality triad at a chosen beat.

use crate::notes;
use crate::score::{Note, Score};

const QUALITIES: [(&str, [i32; 2]); 4] = [
    ("major", [4, 7]),
    ("minor", [3, 7]),
    ("dim", [3, 6]),
    ("aug", [4, 8]),
];

pub fn add_chord(
    score: &Score,
    at: f64,
    root: &str,
    quality: &str,
    beats: f64,
) -> Result<Score, String> {
    if !at.is_finite() || at < 0.0 {
        return Err("at must be a non-negative number".into());
    }
    if !beats.is_finite() || beats <= 0.0 {
        return Err("beats must be a positive number".into());
    }
    let base = notes::midi(root).ok_or_else(|| format!("invalid chord root '{root}'"))?;
    let intervals = QUALITIES
        .iter()
        .find(|(name, _)| *name == quality)
        .map(|(_, steps)| *steps)
        .ok_or_else(|| format!("unknown chord quality '{quality}'"))?;

    let mut result = score.clone();
    for interval in std::iter::once(0).chain(intervals.iter().copied()) {
        let pitch = base + interval;
        if !(0..=127).contains(&pitch) {
            return Err("chord pitch out of MIDI range".into());
        }
        result.notes.push(Note {
            pitch: notes::name(pitch),
            start: at,
            beats,
            velocity: 0.7,
            pan: 0.0,
            envelope: score.envelope,
            vibrato: score.vibrato,
            wave: score.wave,
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    fn sample() -> Score {
        score::parse("tempo 120\nwave saw\nnote C4 1\n").unwrap()
    }

    #[test]
    fn builds_each_quality_from_the_root() {
        for (quality, expected) in [
            ("major", vec!["C4", "E4", "G4"]),
            ("minor", vec!["C4", "D#4", "G4"]),
            ("dim", vec!["C4", "D#4", "F#4"]),
            ("aug", vec!["C4", "E4", "G#4"]),
        ] {
            let result = add_chord(&sample(), 1.0, "C4", quality, 2.0).unwrap();
            let pitches: Vec<&str> = result.notes[1..].iter().map(|n| n.pitch.as_str()).collect();
            assert_eq!(pitches, expected, "quality {quality}");
            assert!(result.notes[1..]
                .iter()
                .all(|n| n.start == 1.0 && n.beats == 2.0));
        }
    }

    #[test]
    fn keeps_existing_notes_and_roundtrips() {
        let source = sample();
        let result = add_chord(&source, 0.0, "C3", "major", 1.0).unwrap();
        assert_eq!(result.notes[0], source.notes[0]);
        assert_eq!(result.notes.len(), 4);
        assert_eq!(score::parse(&score::serialize(&result)).unwrap(), result);
    }

    #[test]
    fn rejects_bad_arguments() {
        let s = sample();
        assert!(add_chord(&s, -1.0, "C4", "major", 1.0).is_err());
        assert!(add_chord(&s, 0.0, "C4", "major", 0.0).is_err());
        assert!(add_chord(&s, 0.0, "H9", "major", 1.0).is_err());
        assert!(add_chord(&s, 0.0, "C4", "seventh", 1.0).is_err());
    }
}
