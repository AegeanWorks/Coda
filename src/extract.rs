//! Extract a beat range from a score, clipping overlapping notes.

use crate::score::Score;

pub fn extract(score: &Score, start: f64, end: f64) -> Result<Score, String> {
    if !start.is_finite() || !end.is_finite() || start < 0.0 || end <= start {
        return Err("range must satisfy 0 <= start < end".into());
    }
    let mut result = score.clone();
    result.notes.clear();
    for note in &score.notes {
        let note_end = note.start + note.beats;
        if note.start >= end || note_end <= start {
            continue;
        }
        let from = note.start.max(start);
        let to = note_end.min(end);
        let mut clipped = note.clone();
        clipped.start = from - start;
        clipped.beats = to - from;
        result.notes.push(clipped);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    fn sample() -> Score {
        score::parse("tempo 120\nwave saw\nnote C4 1\nnote E4 1\nnote G4 1\n").unwrap()
    }

    #[test]
    fn clips_overlapping_notes() {
        let result = extract(&sample(), 0.5, 2.5).unwrap();
        let pairs: Vec<(f64, f64)> = result.notes.iter().map(|n| (n.start, n.beats)).collect();
        assert_eq!(pairs, vec![(0.0, 0.5), (0.5, 1.0), (1.5, 0.5)]);
    }

    #[test]
    fn keeps_full_notes_inside_range() {
        let result = extract(&sample(), 0.0, 3.0).unwrap();
        assert_eq!(result, sample());
    }

    #[test]
    fn preserves_timbre_and_roundtrips() {
        let result = extract(&sample(), 0.5, 2.0).unwrap();
        assert_eq!(result.wave, sample().wave);
        assert_eq!(result.tempo, sample().tempo);
        assert_eq!(result.notes[0].pitch, "C4");
        assert_eq!(result.notes[0].velocity, sample().notes[0].velocity);
        assert_eq!(score::parse(&score::serialize(&result)).unwrap(), result);
    }

    #[test]
    fn rejects_bad_ranges() {
        let s = sample();
        assert!(extract(&s, 1.0, 1.0).is_err());
        assert!(extract(&s, 2.0, 1.0).is_err());
        assert!(extract(&s, -1.0, 2.0).is_err());
    }
}
