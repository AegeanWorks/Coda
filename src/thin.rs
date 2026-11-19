//! Keep sounding notes that are spaced at least `gap` beats apart.

use crate::score::Score;

pub fn thin(score: &Score, gap: f64) -> Result<Score, String> {
    if !gap.is_finite() || gap <= 0.0 {
        return Err("gap must be a positive finite number".into());
    }
    let mut result = score.clone();
    let mut last_kept: Option<f64> = None;
    result.notes.retain(|note| {
        if note.pitch == "-" {
            return true;
        }
        match last_kept {
            None => {
                last_kept = Some(note.start);
                true
            }
            Some(previous) if note.start - previous + 1e-9 >= gap => {
                last_kept = Some(note.start);
                true
            }
            Some(_) => false,
        }
    });
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    fn sample() -> Score {
        score::parse("note C4 0.25\nat 0.1 E4 0.25\nat 0.5 G4 0.25\nat 1.0 A4 0.25\n").unwrap()
    }

    #[test]
    fn removes_notes_closer_than_the_gap() {
        let result = thin(&sample(), 0.4).unwrap();
        let pitches: Vec<&str> = result.notes.iter().map(|n| n.pitch.as_str()).collect();
        assert_eq!(pitches, vec!["C4", "G4", "A4"]);
    }

    #[test]
    fn a_small_gap_keeps_everything() {
        let source = sample();
        let result = thin(&source, 0.1).unwrap();
        assert_eq!(result, source);
    }

    #[test]
    fn preserves_rests_and_kept_fields_and_roundtrips() {
        let source =
            score::parse("note C4 0.25\nrest 0.25\nat 0.1 E4 0.25\nat 1 G4 0.25\n").unwrap();
        let result = thin(&source, 0.4).unwrap();
        assert_eq!(result.notes[1].pitch, "-");
        assert_eq!(result.notes[0], source.notes[0]);
        assert_eq!(score::parse(&score::serialize(&result)).unwrap(), result);
    }

    #[test]
    fn rejects_bad_gaps() {
        let s = sample();
        assert!(thin(&s, 0.0).is_err());
        assert!(thin(&s, -1.0).is_err());
    }
}
