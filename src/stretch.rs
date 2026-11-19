//! Scale a score's event timeline by a positive factor.

use crate::score::Score;

pub fn stretch(score: &Score, factor: f64) -> Result<Score, String> {
    if !factor.is_finite() || factor <= 0.0 {
        return Err("factor must be a positive finite number".into());
    }
    let mut result = score.clone();
    if factor == 1.0 {
        return Ok(result);
    }
    for note in &mut result.notes {
        note.start *= factor;
        note.beats *= factor;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    fn sample() -> Score {
        score::parse("tempo 120\nnote C4 1 0.8 -0.5\nat 3 E4 0.5 0.6\nrest 0.5\n").unwrap()
    }

    #[test]
    fn unit_factor_is_an_identity() {
        assert_eq!(stretch(&sample(), 1.0).unwrap(), sample());
    }

    #[test]
    fn scales_starts_and_durations() {
        let result = stretch(&sample(), 2.0).unwrap();
        let pairs: Vec<(f64, f64)> = result.notes.iter().map(|n| (n.start, n.beats)).collect();
        assert_eq!(pairs, vec![(0.0, 2.0), (6.0, 1.0), (2.0, 1.0)]);
    }

    #[test]
    fn preserves_pitch_and_timbre() {
        let source = score::parse("wave saw\nnote C4 0.5 0.7 -0.4\n").unwrap();
        let result = stretch(&source, 0.5).unwrap();
        assert_eq!(result.wave, source.wave);
        assert_eq!(result.notes[0].pitch, source.notes[0].pitch);
        assert_eq!(result.notes[0].velocity, source.notes[0].velocity);
        assert_eq!(result.notes[0].pan, source.notes[0].pan);
        assert_eq!(result.notes[0].envelope, source.notes[0].envelope);
        assert_eq!(score::parse(&score::serialize(&result)).unwrap(), result);
    }

    #[test]
    fn rejects_bad_factors() {
        let source = score::parse("note C4 1\n").unwrap();
        assert!(stretch(&source, 0.0).is_err());
        assert!(stretch(&source, -2.0).is_err());
        assert!(stretch(&source, f64::NAN).is_err());
    }
}
