//! Scale note durations in place without moving their starts.

use crate::score::Score;

pub fn augment(score: &Score, factor: f64) -> Result<Score, String> {
    if !factor.is_finite() || factor <= 0.0 {
        return Err("factor must be a positive finite number".into());
    }
    let mut result = score.clone();
    if factor == 1.0 {
        return Ok(result);
    }
    for note in &mut result.notes {
        note.beats *= factor;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    fn sample() -> Score {
        score::parse("tempo 120\nwave saw\nnote C4 1 0.8 -0.5\nat 3 E4 0.5 0.6\nrest 0.5\n")
            .unwrap()
    }

    #[test]
    fn scales_only_durations() {
        let result = augment(&sample(), 2.0).unwrap();
        let pairs: Vec<(f64, f64)> = result.notes.iter().map(|n| (n.start, n.beats)).collect();
        assert_eq!(pairs, vec![(0.0, 2.0), (3.0, 1.0), (1.0, 1.0)]);
    }

    #[test]
    fn unit_factor_is_an_identity() {
        assert_eq!(augment(&sample(), 1.0).unwrap(), sample());
    }

    #[test]
    fn preserves_pitch_and_timbre_and_roundtrips() {
        let result = augment(&sample(), 0.5).unwrap();
        assert_eq!(result.tempo, sample().tempo);
        assert_eq!(result.wave, sample().wave);
        assert_eq!(result.notes[0].pitch, sample().notes[0].pitch);
        assert_eq!(result.notes[0].velocity, sample().notes[0].velocity);
        assert_eq!(result.notes[0].pan, sample().notes[0].pan);
        assert_eq!(score::parse(&score::serialize(&result)).unwrap(), result);
    }

    #[test]
    fn rejects_bad_factors() {
        let s = sample();
        assert!(augment(&s, 0.0).is_err());
        assert!(augment(&s, -1.0).is_err());
        assert!(augment(&s, f64::NAN).is_err());
    }
}
