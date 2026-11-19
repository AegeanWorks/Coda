//! Snap note starts to a rhythmic grid without changing any other field.

use crate::score::Score;

pub fn quantize(score: &Score, grid: f64) -> Result<Score, String> {
    if !grid.is_finite() || grid <= 0.0 {
        return Err("grid must be a positive finite number".into());
    }
    let mut result = score.clone();
    for note in &mut result.notes {
        note.start = (note.start / grid).round() * grid;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    #[test]
    fn snaps_to_nearest_grid_point() {
        let source = score::parse("at 0.11 C4 1\nat 0.49 E4 1\nat 0.62 G4 1\n").unwrap();
        let result = quantize(&source, 0.25).unwrap();
        let starts: Vec<f64> = result.notes.iter().map(|n| n.start).collect();
        assert_eq!(starts, vec![0.0, 0.5, 0.5]);
    }

    #[test]
    fn already_quantized_is_an_identity() {
        let source = score::parse("at 0.5 C4 0.25\nat 1.0 E4 0.5\n").unwrap();
        assert_eq!(quantize(&source, 0.25).unwrap(), source);
    }

    #[test]
    fn preserves_every_other_field() {
        let source = score::parse("wave saw\nnote C4 0.3 0.7 -0.6\n").unwrap();
        let result = quantize(&source, 0.25).unwrap();
        assert_eq!(result.wave, source.wave);
        assert_eq!(result.notes[0].beats, source.notes[0].beats);
        assert_eq!(result.notes[0].velocity, source.notes[0].velocity);
        assert_eq!(result.notes[0].pan, source.notes[0].pan);
        assert_eq!(result.notes[0].envelope, source.notes[0].envelope);
        assert_eq!(score::parse(&score::serialize(&result)).unwrap(), result);
    }

    #[test]
    fn rejects_bad_grids() {
        let source = score::parse("note C4 1\n").unwrap();
        assert!(quantize(&source, 0.0).is_err());
        assert!(quantize(&source, -0.25).is_err());
        assert!(quantize(&source, f64::NAN).is_err());
    }
}
