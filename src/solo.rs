//! Keep only the sounding notes within a pitch range.

use crate::notes;
use crate::score::Score;

pub fn solo(score: &Score, low: &str, high: &str) -> Result<Score, String> {
    let lower = notes::midi(low).ok_or_else(|| format!("invalid low note '{low}'"))?;
    let upper = notes::midi(high).ok_or_else(|| format!("invalid high note '{high}'"))?;
    if lower > upper {
        return Err("low must not exceed high".into());
    }
    let mut result = score.clone();
    result.notes.retain(|note| {
        if note.pitch == "-" {
            return false;
        }
        notes::midi(&note.pitch)
            .map(|number| (lower..=upper).contains(&number))
            .unwrap_or(false)
    });
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    fn sample() -> Score {
        score::parse("tempo 120\nwave saw\nnote C3 1\nrest 1\nat 2 C4 1\nat 3 C5 1\nat 4 C6 1\n")
            .unwrap()
    }

    #[test]
    fn keeps_only_notes_in_the_range() {
        let result = solo(&sample(), "C4", "C5").unwrap();
        let pitches: Vec<&str> = result.notes.iter().map(|n| n.pitch.as_str()).collect();
        assert_eq!(pitches, vec!["C4", "C5"]);
    }

    #[test]
    fn drops_rests_and_out_of_range_notes() {
        let result = solo(&sample(), "C3", "C3").unwrap();
        assert_eq!(result.notes.len(), 1);
        assert_eq!(result.notes[0].pitch, "C3");
    }

    #[test]
    fn keeps_kept_notes_unchanged_and_roundtrips() {
        let source = sample();
        let result = solo(&source, "C4", "C5").unwrap();
        assert_eq!(result.notes[0], source.notes[2]);
        assert_eq!(result.tempo, source.tempo);
        assert_eq!(result.wave, source.wave);
        assert_eq!(score::parse(&score::serialize(&result)).unwrap(), result);
    }

    #[test]
    fn rejects_bad_bounds() {
        let s = sample();
        assert!(solo(&s, "H9", "C5").is_err());
        assert!(solo(&s, "C4", "H9").is_err());
        assert!(solo(&s, "C5", "C4").is_err());
    }
}
