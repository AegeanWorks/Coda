//! Add harmony voices at chosen semitone intervals.

use crate::notes;
use crate::score::Score;

pub fn harmonize(score: &Score, intervals: &[i32]) -> Result<Score, String> {
    if intervals.is_empty() {
        return Err("at least one interval is required".into());
    }
    if intervals.iter().any(|i| !(-24..=24).contains(i)) {
        return Err("intervals must be between -24 and 24 semitones".into());
    }

    let mut result = score.clone();
    result.notes.clear();
    for note in &score.notes {
        result.notes.push(note.clone());
        if note.pitch == "-" {
            continue;
        }
        let Some(number) = notes::midi(&note.pitch) else {
            continue;
        };
        for &interval in intervals {
            let target = number + interval;
            if !(0..=127).contains(&target) {
                continue;
            }
            let mut voice = note.clone();
            voice.pitch = notes::name(target);
            voice.velocity = (note.velocity * 0.7).clamp(0.0, 1.0);
            result.notes.push(voice);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    fn sample() -> Score {
        score::parse("tempo 120\nwave sine\nnote C4 1 0.8\n").unwrap()
    }

    #[test]
    fn adds_the_requested_intervals() {
        let result = harmonize(&sample(), &[4, 7]).unwrap();
        let pitches: Vec<&str> = result.notes.iter().map(|n| n.pitch.as_str()).collect();
        assert_eq!(pitches, vec!["C4", "E4", "G4"]);
        assert_eq!(result.notes[0].velocity, 0.8);
        assert!((result.notes[1].velocity - 0.56).abs() < 1e-9);
    }

    #[test]
    fn preserves_original_timing_and_timbre() {
        let result = harmonize(&sample(), &[7]).unwrap();
        assert_eq!(result.notes[1].start, result.notes[0].start);
        assert_eq!(result.notes[1].beats, result.notes[0].beats);
        assert_eq!(result.notes[1].wave, result.notes[0].wave);
        assert_eq!(result.notes[1].envelope, result.notes[0].envelope);
    }

    #[test]
    fn skips_rests() {
        let source = score::parse("note C4 1\nrest 1\n").unwrap();
        let result = harmonize(&source, &[7]).unwrap();
        assert_eq!(result.notes.len(), 3); // C4 + G4 + rest
        assert_eq!(result.notes[2].pitch, "-");
    }

    #[test]
    fn roundtrips_and_rejects_bad_intervals() {
        let result = harmonize(&sample(), &[3, 7]).unwrap();
        assert_eq!(score::parse(&score::serialize(&result)).unwrap(), result);
        assert!(harmonize(&sample(), &[]).is_err());
        assert!(harmonize(&sample(), &[99]).is_err());
    }
}
