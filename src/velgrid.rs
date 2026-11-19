//! Snap sounding-note velocities to an evenly spaced level grid.

use crate::score::Score;

pub fn velgrid(score: &Score, levels: usize) -> Result<Score, String> {
    if levels < 2 {
        return Err("levels must be at least 2".into());
    }
    let step = 1.0 / (levels - 1) as f64;
    let mut result = score.clone();
    for note in &mut result.notes {
        if note.pitch == "-" {
            continue;
        }
        // Round half down so exact ties choose the lower level.
        let index = (note.velocity / step - 0.5).ceil().max(0.0) as usize;
        note.velocity = (index as f64 * step).clamp(0.0, 1.0);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    fn sample() -> Score {
        score::parse("note C4 1 0.1\nat 1 E4 0.5 0.55\nrest 0.5\nat 2 G4 0.5 0.9\n").unwrap()
    }

    #[test]
    fn snaps_to_the_nearest_level() {
        let result = velgrid(&sample(), 3).unwrap(); // levels 0, 0.5, 1
        let velocities: Vec<f64> = result.notes.iter().map(|n| n.velocity).collect();
        assert_eq!(velocities[0], 0.0);
        assert_eq!(velocities[1], 0.5);
        assert_eq!(velocities[3], 1.0);
    }

    #[test]
    fn breaks_ties_toward_the_lower_level() {
        let source = score::parse("note C4 1 0.25\n").unwrap();
        let result = velgrid(&source, 3).unwrap();
        assert_eq!(result.notes[0].velocity, 0.0, "0.25 ties 0 and 0.5");
    }

    #[test]
    fn keeps_rests_and_other_fields_and_roundtrips() {
        let source = sample();
        let result = velgrid(&source, 4).unwrap();
        assert_eq!(result.notes[2].pitch, "-");
        assert_eq!(result.notes[2].velocity, source.notes[2].velocity);
        assert_eq!(result.notes[0].start, source.notes[0].start);
        assert_eq!(result.notes[0].beats, source.notes[0].beats);
        assert_eq!(result.notes[0].wave, source.notes[0].wave);
        assert_eq!(score::parse(&score::serialize(&result)).unwrap(), result);
    }

    #[test]
    fn rejects_bad_level_counts() {
        assert!(velgrid(&sample(), 0).is_err());
        assert!(velgrid(&sample(), 1).is_err());
    }
}
