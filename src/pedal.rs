//! Add a sustained pedal-point drone spanning an entire score.

use crate::notes;
use crate::score::{Note, Score};

pub fn pedal(score: &Score, pitch: &str, velocity: f64) -> Result<Score, String> {
    let number = notes::midi(pitch).ok_or_else(|| format!("invalid pedal note '{pitch}'"))?;
    if !velocity.is_finite() || !(0.0..=1.0).contains(&velocity) {
        return Err("velocity must be between 0 and 1".into());
    }
    let span = score
        .notes
        .iter()
        .map(|n| n.start + n.beats)
        .fold(0.0_f64, f64::max);
    if span <= 0.0 {
        return Err("score has no sounding span to pedal".into());
    }

    let mut result = score.clone();
    result.notes.push(Note {
        pitch: notes::name(number),
        start: 0.0,
        beats: span,
        velocity,
        pan: 0.0,
        envelope: score.envelope,
        vibrato: score.vibrato,
        wave: score.wave,
    });
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    fn sample() -> Score {
        score::parse("tempo 120\nwave saw\nnote C4 1\nat 2 E4 0.5\n").unwrap()
    }

    #[test]
    fn spans_the_whole_score_at_the_first_beat() {
        let result = pedal(&sample(), "C2", 0.4).unwrap();
        let drone = result.notes.last().unwrap();
        assert_eq!(drone.pitch, "C2");
        assert_eq!(drone.start, 0.0);
        assert_eq!(drone.beats, 2.5);
        assert_eq!(drone.velocity, 0.4);
    }

    #[test]
    fn leaves_existing_notes_untouched() {
        let source = sample();
        let result = pedal(&source, "C2", 0.5).unwrap();
        assert_eq!(&result.notes[..source.notes.len()], &source.notes[..]);
        assert_eq!(result.notes.len(), source.notes.len() + 1);
    }

    #[test]
    fn roundtrips_through_the_serializer() {
        let result = pedal(&sample(), "G2", 0.3).unwrap();
        assert_eq!(score::parse(&score::serialize(&result)).unwrap(), result);
    }

    #[test]
    fn rejects_bad_arguments() {
        let s = sample();
        assert!(pedal(&s, "H9", 0.5).is_err());
        assert!(pedal(&s, "C2", 1.5).is_err());
        assert!(pedal(&score::parse("tempo 120\n").unwrap(), "C2", 0.5).is_err());
    }
}
