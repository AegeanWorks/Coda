//! Note-name to frequency conversion (equal temperament, A4 = 440 Hz).

/// Convert a note name such as `C4`, `A#4` or `Bb3` into a MIDI note number
/// (A4 = 69). Returns `None` for malformed input.
pub fn midi(name: &str) -> Option<i32> {
    let s = name.trim();
    let b = s.as_bytes();
    if b.is_empty() {
        return None;
    }

    let letter = (b[0] as char).to_ascii_uppercase();
    let mut semitone: i32 = match letter {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };

    let mut i = 1;
    if i < b.len() {
        match b[i] {
            b'#' => {
                semitone += 1;
                i += 1;
            }
            b'b' => {
                semitone -= 1;
                i += 1;
            }
            _ => {}
        }
    }

    let octave: i32 = s.get(i..)?.parse().ok()?;
    Some((octave + 1) * 12 + semitone)
}

/// Convert a note name such as `C4`, `A#4` or `Bb3` into Hz.
/// Returns `None` for malformed input.
pub fn frequency(name: &str) -> Option<f64> {
    let number = midi(name)?;
    Some(440.0 * 2.0_f64.powf((number - 69) as f64 / 12.0))
}

/// Convert a MIDI note number into a note name (`60` -> `C4`).
pub fn name(midi: i32) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let pc = midi.rem_euclid(12) as usize;
    let octave = midi.div_euclid(12) - 1;
    format!("{}{}", NAMES[pc], octave)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midi_numbers() {
        assert_eq!(midi("A4"), Some(69));
        assert_eq!(midi("C4"), Some(60));
        assert_eq!(midi("Bb3"), Some(58));
        assert_eq!(midi("-"), None);
    }

    #[test]
    fn reference_pitches() {
        assert!((frequency("A4").unwrap() - 440.0).abs() < 1e-9);
        assert!((frequency("C4").unwrap() - 261.6255653).abs() < 1e-6);
        assert!((frequency("A3").unwrap() - 220.0).abs() < 1e-9);
    }

    #[test]
    fn sharps_and_flats_agree() {
        assert!((frequency("A#4").unwrap() - frequency("Bb4").unwrap()).abs() < 1e-9);
    }

    #[test]
    fn rejects_garbage() {
        assert!(frequency("H4").is_none());
        assert!(frequency("C").is_none());
        assert!(frequency("Cx4").is_none());
    }

    #[test]
    fn names_roundtrip_through_frequency() {
        assert_eq!(name(60), "C4");
        assert_eq!(name(69), "A4");
        assert!((frequency(&name(69)).unwrap() - 440.0).abs() < 1e-9);
        assert!((frequency(&name(60)).unwrap() - frequency("C4").unwrap()).abs() < 1e-9);
    }
}
