//! Extend each note to the next distinct onset for a legato line.

use crate::score::Score;

pub fn sustain(score: &Score) -> Result<Score, String> {
    if score.notes.is_empty() {
        return Err("score has no notes to sustain".into());
    }
    let mut onsets: Vec<f64> = score.notes.iter().map(|n| n.start).collect();
    onsets.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    onsets.dedup_by(|a, b| (*a - *b).abs() < 1e-9);

    let mut result = score.clone();
    for note in &mut result.notes {
        if let Some(next) = onsets.iter().find(|start| **start > note.start + 1e-9) {
            let length = *next - note.start;
            if length > 0.0 {
                note.beats = length;
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    #[test]
    fn extends_notes_to_the_next_onset() {
        let source = score::parse("note C4 1\nat 2 E4 0.5\nat 4 G4 0.5\n").unwrap();
        let result = sustain(&source).unwrap();
        let beats: Vec<f64> = result.notes.iter().map(|n| n.beats).collect();
        assert_eq!(beats, vec![2.0, 2.0, 0.5]);
    }

    #[test]
    fn extends_chord_notes_and_keeps_the_final_note() {
        let source = score::parse("note C4 1\nat 0 E4 0.25\nat 3 G4 0.5\n").unwrap();
        let result = sustain(&source).unwrap();
        assert_eq!(result.notes[0].beats, 3.0);
        assert_eq!(result.notes[1].beats, 3.0, "chord voice also extends");
        assert_eq!(result.notes[2].beats, 0.5, "final note untouched");
    }

    #[test]
    fn preserves_pitch_and_timbre_and_roundtrips() {
        let source = score::parse("wave saw\nnote C4 1 0.7 -0.4\nat 2 E4 0.5\n").unwrap();
        let result = sustain(&source).unwrap();
        assert_eq!(result.wave, source.wave);
        assert_eq!(result.notes[0].pitch, "C4");
        assert_eq!(result.notes[0].velocity, source.notes[0].velocity);
        assert_eq!(result.notes[0].pan, source.notes[0].pan);
        assert_eq!(score::parse(&score::serialize(&result)).unwrap(), result);
    }

    #[test]
    fn rejects_an_empty_score() {
        assert!(sustain(&score::parse("tempo 120\n").unwrap()).is_err());
    }
}
