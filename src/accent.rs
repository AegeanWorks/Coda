//! Apply a repeating velocity pattern to sounding notes.

use crate::score::Score;

pub fn accent(score: &Score, pattern: &[f64]) -> Result<Score, String> {
    if pattern.is_empty() {
        return Err("at least one velocity is required".into());
    }
    if pattern
        .iter()
        .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
    {
        return Err("velocities must be between 0 and 1".into());
    }

    let mut result = score.clone();
    let mut index = 0_usize;
    for note in &mut result.notes {
        if note.pitch == "-" {
            continue;
        }
        note.velocity = pattern[index % pattern.len()];
        index += 1;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    fn sample() -> Score {
        score::parse("note C4 1\nnote E4 1\nrest 1\nnote G4 1\n").unwrap()
    }

    #[test]
    fn cycles_the_pattern_across_sounding_notes() {
        let result = accent(&sample(), &[1.0, 0.5]).unwrap();
        let velocities: Vec<f64> = result.notes.iter().map(|n| n.velocity).collect();
        assert_eq!(velocities, vec![1.0, 0.5, 1.0, 1.0]);
        assert_eq!(result.notes[2].pitch, "-");
    }

    #[test]
    fn preserves_timing_and_pitch() {
        let source = sample();
        let result = accent(&source, &[0.6]).unwrap();
        for (before, after) in source.notes.iter().zip(result.notes.iter()) {
            assert_eq!(before.start, after.start);
            assert_eq!(before.beats, after.beats);
            assert_eq!(before.pitch, after.pitch);
        }
    }

    #[test]
    fn roundtrips_and_rejects_bad_patterns() {
        let result = accent(&sample(), &[0.8, 0.4, 0.6]).unwrap();
        assert_eq!(score::parse(&score::serialize(&result)).unwrap(), result);
        assert!(accent(&sample(), &[]).is_err());
        assert!(accent(&sample(), &[1.5]).is_err());
    }
}
