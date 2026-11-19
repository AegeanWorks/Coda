//! Superimpose scores at their own absolute beats, without shifting starts.

use crate::score::Score;
use std::cmp::Ordering;

pub fn overlay_scores(scores: &[Score]) -> Result<Score, String> {
    let first = scores
        .first()
        .ok_or_else(|| "no scores to overlay".to_string())?;
    let mut result = first.clone();
    for score in scores.iter().skip(1) {
        result.notes.extend(score.notes.iter().cloned());
    }
    result.notes.sort_by(|a, b| {
        a.start
            .partial_cmp(&b.start)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.pitch.cmp(&b.pitch))
    });
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    #[test]
    fn keeps_absolute_starts_and_sums_notes() {
        let a = score::parse("tempo 90\nwave saw\nnote C4 1\nat 2 E4 0.5\n").unwrap();
        let b = score::parse("tempo 200\nwave sine\nnote G3 2\nat 1 B3 0.5\n").unwrap();
        let merged = overlay_scores(&[a, b]).unwrap();
        let starts: Vec<f64> = merged.notes.iter().map(|n| n.start).collect();
        assert_eq!(starts, vec![0.0, 0.0, 1.0, 2.0]);
        assert_eq!(merged.notes.len(), 4);
        assert_eq!(merged.tempo, 90.0, "the first score's settings win");
    }

    #[test]
    fn orders_notes_deterministically() {
        let a = score::parse("note E4 1\nat 0 C4 1\n").unwrap();
        let merged = overlay_scores(std::slice::from_ref(&a)).unwrap();
        let pitches: Vec<&str> = merged.notes.iter().map(|n| n.pitch.as_str()).collect();
        assert_eq!(pitches, vec!["C4", "E4"]);
    }

    #[test]
    fn roundtrips_through_the_serializer() {
        let a = score::parse("preset pad\nnote C4 0.5\n").unwrap();
        let b = score::parse("wave saw\nnote E4 0.5\n").unwrap();
        let merged = overlay_scores(&[a, b]).unwrap();
        assert_eq!(score::parse(&score::serialize(&merged)).unwrap(), merged);
    }

    #[test]
    fn rejects_empty_input() {
        assert!(overlay_scores(&[]).is_err());
    }
}
