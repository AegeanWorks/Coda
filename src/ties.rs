//! Merge consecutive touching notes of the same pitch into one.

use crate::score::Score;

pub fn ties(score: &Score) -> Result<Score, String> {
    if score.notes.is_empty() {
        return Err("score has no notes to tie".into());
    }
    let mut result = score.clone();
    let mut merged: Vec<crate::score::Note> = Vec::with_capacity(result.notes.len());
    for note in result.notes.drain(..) {
        if let Some(last) = merged.last_mut() {
            if last.pitch != "-"
                && last.pitch == note.pitch
                && (last.start + last.beats - note.start).abs() < 1e-9
            {
                last.beats += note.beats;
                continue;
            }
        }
        merged.push(note);
    }
    result.notes = merged;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    #[test]
    fn merges_touching_notes_of_the_same_pitch() {
        let source = score::parse("note C4 1\nnote C4 0.5\nnote E4 1\n").unwrap();
        let result = ties(&source).unwrap();
        assert_eq!(result.notes.len(), 2);
        assert_eq!(result.notes[0].pitch, "C4");
        assert_eq!(result.notes[0].beats, 1.5);
        assert_eq!(result.notes[1].pitch, "E4");
    }

    #[test]
    fn leaves_gaps_and_rests_untouched() {
        let source = score::parse("note C4 1\nat 2 C4 0.5\nrest 0.5\n").unwrap();
        let result = ties(&source).unwrap();
        assert_eq!(result.notes.len(), 3, "gap and rest keep notes separate");
        assert_eq!(result.notes[0].beats, 1.0);
    }

    #[test]
    fn keeps_the_first_notes_fields() {
        let source = score::parse("note C4 1 0.7 -0.3\nnote C4 0.5 0.2 0.4\n").unwrap();
        let result = ties(&source).unwrap();
        assert_eq!(result.notes[0].velocity, 0.7);
        assert_eq!(result.notes[0].pan, -0.3);
        assert_eq!(result.notes[0].beats, 1.5);
    }

    #[test]
    fn roundtrips_and_rejects_empty() {
        let source = score::parse("note C4 1\nnote C4 0.5\n").unwrap();
        let result = ties(&source).unwrap();
        assert_eq!(score::parse(&score::serialize(&result)).unwrap(), result);
        assert!(ties(&score::parse("tempo 120\n").unwrap()).is_err());
    }
}
