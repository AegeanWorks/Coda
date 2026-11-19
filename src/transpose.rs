//! Chromatic transposition: shift every pitched note by a number of semitones.
//!
//! Only `pitch` changes. Timing, velocity, pan, envelopes, vibrato, waveform
//! and score-level settings are copied unchanged.

use crate::notes;
use crate::score::{Note, Score};

pub fn transpose(score: &Score, semitones: i32) -> Score {
    let mut result = score.clone();
    for note in &mut result.notes {
        note.pitch = transpose_note(note, semitones);
    }
    result
}

fn transpose_note(note: &Note, semitones: i32) -> String {
    if note.pitch == "-" {
        return note.pitch.clone();
    }
    match notes::midi(&note.pitch) {
        Some(number) => notes::name(number + semitones),
        None => note.pitch.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    #[test]
    fn shifts_pitches_by_semitones() {
        let score = score::parse("note C4 1\nat 2 E4 1\n").unwrap();
        let up = transpose(&score, 12);
        assert_eq!(up.notes[0].pitch, "C5");
        assert_eq!(up.notes[1].pitch, "E5");
        let down = transpose(&score, -1);
        assert_eq!(down.notes[0].pitch, "B3");
        assert_eq!(down.notes[1].pitch, "D#4");
    }

    #[test]
    fn preserves_everything_except_pitch() {
        let score = score::parse(
            "tempo 96\ngain 0.4\npreset pad\nvib 7 40\npan -0.5\nnote C4 1 0.8\nrest 1\n",
        )
        .unwrap();
        let shifted = transpose(&score, 3);
        assert_eq!(shifted.tempo, score.tempo);
        assert_eq!(shifted.gain, score.gain);
        assert_eq!(shifted.wave, score.wave);
        assert_eq!(shifted.envelope, score.envelope);
        assert_eq!(shifted.vibrato, score.vibrato);
        for (before, after) in score.notes.iter().zip(shifted.notes.iter()) {
            assert_eq!(before.start, after.start);
            assert_eq!(before.beats, after.beats);
            assert_eq!(before.velocity, after.velocity);
            assert_eq!(before.pan, after.pan);
            assert_eq!(before.envelope, after.envelope);
            assert_eq!(before.vibrato, after.vibrato);
            assert_eq!(before.wave, after.wave);
        }
        assert_eq!(shifted.notes[1].pitch, "-");
    }

    #[test]
    fn inverse_transpose_is_identity() {
        let score = score::parse("wave saw\nnote C4 1\nat 2 G4 1\n").unwrap();
        assert_eq!(transpose(&transpose(&score, 19), -19), score);
    }

    #[test]
    fn zero_semitones_changes_nothing() {
        let score = score::parse("note C4 1\n").unwrap();
        assert_eq!(transpose(&score, 0), score);
    }
}
