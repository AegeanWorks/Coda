//! Melodic inversion around a chosen center pitch.

use crate::notes;
use crate::score::Score;

pub fn invert(score: &Score, center: &str) -> Result<Score, String> {
    let pivot = notes::midi(center).ok_or_else(|| format!("invalid center note '{center}'"))?;
    let mut result = score.clone();
    for note in &mut result.notes {
        if note.pitch == "-" {
            continue;
        }
        let Some(number) = notes::midi(&note.pitch) else {
            continue;
        };
        let target = 2 * pivot - number;
        if (0..=127).contains(&target) {
            note.pitch = notes::name(target);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    fn sample() -> Score {
        score::parse("tempo 120\nwave sine\nnote C4 1 0.8\nat 1 E4 0.5 0.6\n").unwrap()
    }

    #[test]
    fn reflects_pitches_around_the_center() {
        let result = invert(&sample(), "C4").unwrap();
        let pitches: Vec<&str> = result.notes.iter().map(|n| n.pitch.as_str()).collect();
        assert_eq!(pitches, vec!["C4", "G#3"]);
    }

    #[test]
    fn center_at_the_first_pitch_keeps_it() {
        let result = invert(&sample(), "C4").unwrap();
        assert_eq!(result.notes[0].pitch, "C4");
    }

    #[test]
    fn preserves_other_fields_and_roundtrips() {
        let result = invert(&sample(), "D4").unwrap();
        assert_eq!(result.notes[0].beats, sample().notes[0].beats);
        assert_eq!(result.notes[0].velocity, sample().notes[0].velocity);
        assert_eq!(result.notes[0].wave, sample().notes[0].wave);
        assert_eq!(score::parse(&score::serialize(&result)).unwrap(), result);
    }

    #[test]
    fn rejects_bad_center() {
        assert!(invert(&sample(), "H9").is_err());
    }
}
