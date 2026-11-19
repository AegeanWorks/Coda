//! Append transposed copies of a phrase at successive offsets.

use crate::notes;
use crate::score::Score;

pub fn sequence(score: &Score, shifts: &[i32]) -> Result<Score, String> {
    if shifts.is_empty() {
        return Err("at least one shift is required".into());
    }
    if shifts.iter().any(|s| !(-24..=24).contains(s)) {
        return Err("shifts must be between -24 and 24 semitones".into());
    }
    let span = score
        .notes
        .iter()
        .map(|n| n.start + n.beats)
        .fold(0.0_f64, f64::max);
    if span <= 0.0 {
        return Err("score has no positive span to sequence".into());
    }

    let mut result = score.clone();
    for (index, &shift) in shifts.iter().enumerate() {
        let offset = span * (index + 1) as f64;
        for note in &score.notes {
            let mut copy = note.clone();
            copy.start += offset;
            if note.pitch != "-" {
                if let Some(number) = notes::midi(&note.pitch) {
                    let target = number + shift;
                    if (0..=127).contains(&target) {
                        copy.pitch = notes::name(target);
                    }
                }
            }
            result.notes.push(copy);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    fn sample() -> Score {
        score::parse("tempo 120\nwave saw\nnote C4 1\nnote E4 1\n").unwrap()
    }

    #[test]
    fn appends_a_transposed_copy_per_shift() {
        let result = sequence(&sample(), &[0, 12]).unwrap();
        let pairs: Vec<(f64, &str)> = result
            .notes
            .iter()
            .map(|n| (n.start, n.pitch.as_str()))
            .collect();
        assert_eq!(
            pairs,
            vec![
                (0.0, "C4"),
                (1.0, "E4"),
                (2.0, "C4"),
                (3.0, "E4"),
                (4.0, "C5"),
                (5.0, "E5"),
            ]
        );
    }

    #[test]
    fn zero_shift_repeats_the_phrase_exactly() {
        let result = sequence(&sample(), &[0]).unwrap();
        assert_eq!(result.notes.len(), 4);
        for (original, copy) in result.notes[..2].iter().zip(result.notes[2..].iter()) {
            assert_eq!(original.pitch, copy.pitch);
            assert_eq!(original.beats, copy.beats);
            assert_eq!(original.velocity, copy.velocity);
            assert_eq!(original.wave, copy.wave);
            assert_eq!(copy.start, original.start + 2.0);
        }
        assert_eq!(result.notes[2].start, 2.0);
    }

    #[test]
    fn preserves_timbre_and_roundtrips() {
        let result = sequence(&sample(), &[0, 7]).unwrap();
        assert_eq!(result.tempo, sample().tempo);
        assert_eq!(result.wave, sample().wave);
        assert_eq!(result.notes[2].velocity, sample().notes[0].velocity);
        assert_eq!(score::parse(&score::serialize(&result)).unwrap(), result);
    }

    #[test]
    fn rejects_bad_shifts_and_empty_scores() {
        let s = sample();
        assert!(sequence(&s, &[]).is_err());
        assert!(sequence(&s, &[99]).is_err());
        assert!(sequence(&score::parse("tempo 120\n").unwrap(), &[0]).is_err());
    }
}
