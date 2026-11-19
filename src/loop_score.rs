//! Repeat a score's phrase N times into one canonical score.

use crate::score::Score;

pub fn repeat_score(score: &Score, times: usize) -> Result<Score, String> {
    if times == 0 {
        return Err("times must be at least 1".into());
    }
    if times == 1 || score.notes.is_empty() {
        return Ok(score.clone());
    }
    let span = score
        .notes
        .iter()
        .map(|n| n.start + n.beats)
        .fold(0.0_f64, f64::max);
    let mut result = score.clone();
    result.notes.clear();
    for repetition in 0..times {
        for note in &score.notes {
            let mut shifted = note.clone();
            shifted.start += repetition as f64 * span;
            result.notes.push(shifted);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    fn sample() -> Score {
        score::parse("tempo 120\nwave sine\nnote C4 1\nnote E4 0.5\nrest 0.5\n").unwrap()
    }

    #[test]
    fn one_repetition_is_an_identity() {
        assert_eq!(repeat_score(&sample(), 1).unwrap(), sample());
    }

    #[test]
    fn shifts_notes_by_the_phrase_length() {
        let result = repeat_score(&sample(), 2).unwrap();
        assert_eq!(result.notes.len(), 6);
        let starts: Vec<f64> = result.notes.iter().map(|n| n.start).collect();
        assert_eq!(starts, vec![0.0, 1.0, 1.5, 2.0, 3.0, 3.5]);
    }

    #[test]
    fn preserves_every_note_field_and_roundtrips() {
        let source = score::parse("wave saw\nnote C4 0.5 0.7 -0.4\n").unwrap();
        let result = repeat_score(&source, 3).unwrap();
        assert_eq!(result.wave, source.wave);
        assert_eq!(result.tempo, source.tempo);
        for note in &result.notes {
            assert_eq!(note.pitch, "C4");
            assert_eq!(note.beats, 0.5);
            assert_eq!(note.velocity, 0.7);
            assert_eq!(note.pan, -0.4);
        }
        assert_eq!(score::parse(&score::serialize(&result)).unwrap(), result);
    }

    #[test]
    fn rejects_zero() {
        assert!(repeat_score(&sample(), 0).is_err());
    }
}
