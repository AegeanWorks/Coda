//! Classic long-short eighth-note swing, applied deterministically.

use crate::score::Score;

const EIGHTH: f64 = 0.5;
const TOLERANCE: f64 = 0.05;

pub fn swing(score: &Score, ratio: f64) -> Result<Score, String> {
    if !ratio.is_finite() || !(0.0..=1.0).contains(&ratio) {
        return Err("ratio must be between 0 and 1".into());
    }
    let mut result = score.clone();
    if ratio == 0.0 {
        return Ok(result);
    }
    let delay = ratio * 0.25;
    for note in &mut result.notes {
        let eighth = (note.start / EIGHTH).round();
        let on_grid = (note.start - eighth * EIGHTH).abs() <= TOLERANCE;
        if on_grid && eighth.rem_euclid(2.0) == 1.0 {
            note.start += delay;
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    fn sample() -> Score {
        score::parse("note C4 0.5\nnote E4 0.5\nnote G4 0.5\nnote B4 0.5\n").unwrap()
    }

    #[test]
    fn zero_ratio_is_an_identity() {
        assert_eq!(swing(&sample(), 0.0).unwrap(), sample());
    }

    #[test]
    fn delays_only_off_beats() {
        let result = swing(&sample(), 0.8).unwrap();
        let starts: Vec<f64> = result.notes.iter().map(|n| n.start).collect();
        assert_eq!(starts, vec![0.0, 0.7, 1.0, 1.7]);
    }

    #[test]
    fn full_ratio_reaches_a_quarter_of_a_beat() {
        let result = swing(&sample(), 1.0).unwrap();
        assert_eq!(result.notes[1].start, 0.75);
    }

    #[test]
    fn preserves_other_fields_and_roundtrips() {
        let source = score::parse("wave saw\ninst organ\nnote C4 0.5 0.7 -0.4\n").unwrap();
        let result = swing(&source, 0.5).unwrap();
        assert_eq!(result.notes[0].wave, source.notes[0].wave);
        assert_eq!(result.notes[0].beats, source.notes[0].beats);
        assert_eq!(result.notes[0].velocity, source.notes[0].velocity);
        assert_eq!(score::parse(&score::serialize(&result)).unwrap(), result);
    }

    #[test]
    fn rejects_bad_ratios() {
        assert!(swing(&sample(), -0.1).is_err());
        assert!(swing(&sample(), 1.2).is_err());
    }
}
