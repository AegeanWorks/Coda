//! Linear velocity ramp (crescendo / diminuendo) across sounding notes.

use crate::score::Score;

pub fn ramp(score: &Score, from: f64, to: f64) -> Result<Score, String> {
    for value in [from, to] {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err("velocity endpoints must be between 0 and 1".into());
        }
    }
    let indices: Vec<usize> = score
        .notes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.pitch != "-")
        .map(|(i, _)| i)
        .collect();
    let mut result = score.clone();
    if indices.len() == 1 {
        result.notes[indices[0]].velocity = to;
        return Ok(result);
    }
    for (position, &index) in indices.iter().enumerate() {
        let fraction = position as f64 / (indices.len() - 1) as f64;
        result.notes[index].velocity = from + (to - from) * fraction;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    fn sample() -> Score {
        score::parse("note C4 1\nrest 1\nat 2 E4 1\nat 3 G4 1\nat 4 A4 1\n").unwrap()
    }

    #[test]
    fn ramps_across_sounding_notes() {
        let result = ramp(&sample(), 0.2, 1.0).unwrap();
        let velocities: Vec<f64> = result.notes.iter().map(|n| n.velocity).collect();
        assert!((velocities[0] - 0.2).abs() < 1e-9);
        assert!((velocities[2] - 0.466666).abs() < 1e-5, "{}", velocities[2]);
        assert!((velocities[3] - 0.733333).abs() < 1e-5, "{}", velocities[3]);
        assert!((velocities[4] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn skips_rests() {
        let result = ramp(&sample(), 0.2, 1.0).unwrap();
        assert_eq!(result.notes[1].pitch, "-");
        assert_eq!(result.notes[1].velocity, sample().notes[1].velocity);
    }

    #[test]
    fn a_single_note_takes_the_end_value() {
        let source = score::parse("note C4 1 0.5\n").unwrap();
        let result = ramp(&source, 0.1, 0.9).unwrap();
        assert!((result.notes[0].velocity - 0.9).abs() < 1e-9);
    }

    #[test]
    fn preserves_other_fields_and_roundtrips() {
        let source = sample();
        let result = ramp(&source, 0.3, 0.8).unwrap();
        assert_eq!(result.notes[0].start, source.notes[0].start);
        assert_eq!(result.notes[0].beats, source.notes[0].beats);
        assert_eq!(result.notes[0].pitch, source.notes[0].pitch);
        assert_eq!(score::parse(&score::serialize(&result)).unwrap(), result);
    }

    #[test]
    fn rejects_bad_endpoints() {
        let s = sample();
        assert!(ramp(&s, -0.1, 1.0).is_err());
        assert!(ramp(&s, 0.0, 1.5).is_err());
    }
}
