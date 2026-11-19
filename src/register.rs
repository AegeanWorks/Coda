//! Fit sounding pitches into a target register by octave displacement.

use crate::notes;
use crate::score::Score;

pub fn register(score: &Score, low: &str, high: &str) -> Result<Score, String> {
    let lower = notes::midi(low).ok_or_else(|| format!("invalid low note '{low}'"))?;
    let upper = notes::midi(high).ok_or_else(|| format!("invalid high note '{high}'"))?;
    if lower > upper {
        return Err("low must not exceed high".into());
    }

    let mut result = score.clone();
    for note in &mut result.notes {
        if note.pitch == "-" {
            continue;
        }
        let Some(number) = notes::midi(&note.pitch) else {
            continue;
        };
        let fitted = if number < lower {
            let shifted = number + 12 * ((lower - number + 11) / 12);
            if shifted <= upper {
                shifted
            } else {
                number
            }
        } else if number > upper {
            let shifted = number - 12 * ((number - upper + 11) / 12);
            if shifted >= lower {
                shifted
            } else {
                number
            }
        } else {
            number
        };
        if (0..=127).contains(&fitted) {
            note.pitch = notes::name(fitted);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    fn sample() -> Score {
        score::parse("tempo 120\nwave sine\nnote C2 1 0.8\nnote C4 1 0.6\nnote C6 1 0.4\n").unwrap()
    }

    #[test]
    fn folds_pitches_into_the_range() {
        let result = register(&sample(), "C4", "C5").unwrap();
        let pitches: Vec<&str> = result.notes.iter().map(|n| n.pitch.as_str()).collect();
        assert_eq!(pitches, vec!["C4", "C4", "C5"]);
    }

    #[test]
    fn leaves_notes_when_no_octave_fits() {
        let result = register(&sample(), "C4", "C4").unwrap();
        let pitches: Vec<&str> = result.notes.iter().map(|n| n.pitch.as_str()).collect();
        assert_eq!(pitches, vec!["C4", "C4", "C4"]);
    }

    #[test]
    fn preserves_other_fields_and_roundtrips() {
        let result = register(&sample(), "C3", "C5").unwrap();
        assert_eq!(result.tempo, sample().tempo);
        assert_eq!(result.wave, sample().wave);
        assert_eq!(result.notes[0].beats, sample().notes[0].beats);
        assert_eq!(result.notes[0].velocity, sample().notes[0].velocity);
        assert_eq!(score::parse(&score::serialize(&result)).unwrap(), result);
    }

    #[test]
    fn rejects_bad_bounds() {
        let s = sample();
        assert!(register(&s, "H9", "C5").is_err());
        assert!(register(&s, "C4", "H9").is_err());
        assert!(register(&s, "C5", "C4").is_err());
    }
}
