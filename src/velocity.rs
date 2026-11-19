//! Rescale a score's velocities so the loudest note reaches a target peak.

use crate::score::Score;

pub fn normalize_velocity(score: &Score, peak: f64) -> Result<Score, String> {
    if !peak.is_finite() || !(0.0..=1.0).contains(&peak) {
        return Err("peak must be between 0 and 1".into());
    }
    let loudest = score
        .notes
        .iter()
        .filter(|n| n.pitch != "-")
        .map(|n| n.velocity)
        .fold(0.0_f64, f64::max);
    let mut result = score.clone();
    if loudest <= 0.0 {
        return Ok(result);
    }
    let factor = peak / loudest;
    for note in &mut result.notes {
        if note.pitch == "-" {
            continue;
        }
        note.velocity = (note.velocity * factor).clamp(0.0, 1.0);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    fn sample() -> Score {
        score::parse("note C4 1 0.8\nat 1 E4 0.5 0.4\nrest 0.5\nat 2 G4 0.5 0.2\n").unwrap()
    }

    #[test]
    fn scales_to_the_target_peak_and_keeps_ratios() {
        let result = normalize_velocity(&sample(), 1.0).unwrap();
        let velocities: Vec<f64> = result.notes.iter().map(|n| n.velocity).collect();
        assert_eq!(velocities[0], 1.0);
        assert!((velocities[1] - 0.5).abs() < 1e-9);
        assert_eq!(velocities[3], 0.25);
    }

    #[test]
    fn a_lower_peak_scales_everything_down() {
        let result = normalize_velocity(&sample(), 0.5).unwrap();
        assert_eq!(result.notes[0].velocity, 0.5);
        assert!((result.notes[1].velocity - 0.25).abs() < 1e-9);
    }

    #[test]
    fn keeps_rests_and_other_fields_and_roundtrips() {
        let source = sample();
        let result = normalize_velocity(&source, 0.9).unwrap();
        assert_eq!(result.notes[2].pitch, "-");
        assert_eq!(result.notes[2].velocity, source.notes[2].velocity);
        assert_eq!(result.notes[0].start, source.notes[0].start);
        assert_eq!(result.notes[0].beats, source.notes[0].beats);
        assert_eq!(result.notes[0].wave, source.notes[0].wave);
        assert_eq!(score::parse(&score::serialize(&result)).unwrap(), result);
    }

    #[test]
    fn silent_scores_are_unchanged_and_bad_peaks_fail() {
        let silent = score::parse("note C4 1 0\n").unwrap();
        assert_eq!(normalize_velocity(&silent, 0.8).unwrap(), silent);
        let s = sample();
        assert!(normalize_velocity(&s, 1.5).is_err());
        assert!(normalize_velocity(&s, -0.1).is_err());
    }
}
