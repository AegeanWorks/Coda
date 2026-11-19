//! Transpose notes by scale degrees rather than by semitones.

use crate::gen::Scale;
use crate::notes;
use crate::score::Score;

pub fn diatonic(score: &Score, steps: i32, scale: Scale) -> Score {
    let degrees = scale.degrees();
    let span = degrees.len() as i32;
    let mut result = score.clone();
    for note in &mut result.notes {
        if note.pitch == "-" {
            continue;
        }
        let Some(number) = notes::midi(&note.pitch) else {
            continue;
        };
        let pitch_class = number.rem_euclid(12);
        let octave = number.div_euclid(12) - 1;
        let index = degrees
            .iter()
            .position(|degree| *degree == pitch_class)
            .unwrap_or_else(|| {
                degrees
                    .iter()
                    .rposition(|degree| *degree <= pitch_class)
                    .unwrap_or(0)
            });
        let shifted = index as i32 + steps;
        let octave_shift = shifted.div_euclid(span);
        let degree = degrees[shifted.rem_euclid(span) as usize];
        note.pitch = notes::name((octave + octave_shift + 1) * 12 + degree);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    fn sample() -> Score {
        score::parse("note C4 1\nnote E4 0.5\nat 2 A4 1\n").unwrap()
    }

    #[test]
    fn zero_steps_is_an_identity() {
        assert_eq!(diatonic(&sample(), 0, Scale::Major), sample());
    }

    #[test]
    fn steps_move_by_degrees_not_semitones() {
        let result = diatonic(&sample(), 1, Scale::Major);
        let pitches: Vec<&str> = result.notes.iter().map(|n| n.pitch.as_str()).collect();
        assert_eq!(pitches, vec!["D4", "F4", "B4"]);
    }

    #[test]
    fn a_full_scale_walks_an_octave() {
        let result = diatonic(&sample(), 7, Scale::Major);
        let pitches: Vec<&str> = result.notes.iter().map(|n| n.pitch.as_str()).collect();
        assert_eq!(pitches, vec!["C5", "E5", "A5"]);
    }

    #[test]
    fn notes_stay_in_the_chosen_scale() {
        let result = diatonic(&sample(), 3, Scale::Minor);
        let degrees = Scale::Minor.degrees();
        assert!(result.notes.iter().all(|n| {
            notes::midi(&n.pitch)
                .map(|m| degrees.contains(&(m.rem_euclid(12))))
                .unwrap_or(true)
        }));
    }

    #[test]
    fn preserves_every_other_field() {
        let source = score::parse("wave saw\nnote C4 0.5 0.7 -0.4\n").unwrap();
        let result = diatonic(&source, 2, Scale::Pentatonic);
        assert_eq!(result.notes[0].beats, source.notes[0].beats);
        assert_eq!(result.notes[0].velocity, source.notes[0].velocity);
        assert_eq!(result.notes[0].pan, source.notes[0].pan);
        assert_eq!(score::parse(&score::serialize(&result)).unwrap(), result);
    }
}
