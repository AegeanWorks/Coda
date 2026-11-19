//! Deterministic score linting: legal but questionable values get a warning
//! before any audio or MIDI is written. Linting never changes a score.

use crate::notes;
use crate::score::Score;
use std::collections::BTreeMap;

/// A single lint message, generated in a stable order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Warning {
    pub message: String,
}

fn warn(out: &mut Vec<Warning>, message: String) {
    out.push(Warning { message });
}

pub fn lint(score: &Score) -> Vec<Warning> {
    let mut out = Vec::new();

    if score.tempo < 40.0 || score.tempo > 200.0 {
        warn(
            &mut out,
            format!("score: tempo {} is outside 40..200", score.tempo),
        );
    }
    if score.gain > 0.8 {
        warn(
            &mut out,
            format!("score: gain {} risks clipping", score.gain),
        );
    }
    if score.vibrato.rate > 20.0 {
        warn(
            &mut out,
            format!("score: vibrato rate {} Hz is extreme", score.vibrato.rate),
        );
    }
    if score.vibrato.cents > 100.0 {
        warn(
            &mut out,
            format!(
                "score: vibrato depth {} cents is extreme",
                score.vibrato.cents
            ),
        );
    }
    if score.envelope.release > 2.0 {
        warn(
            &mut out,
            format!("score: release {} s is very long", score.envelope.release),
        );
    }

    let mut seen: BTreeMap<(i64, String), usize> = BTreeMap::new();
    for (index, note) in score.notes.iter().enumerate() {
        let at = (note.start * 480.0).round() as i64;
        if note.pitch == "-" {
            if note.beats > 8.0 {
                warn(
                    &mut out,
                    format!(
                        "note {index} @ beat {}: rest of {} beats is very long",
                        note.start, note.beats
                    ),
                );
            }
            continue;
        }

        if let Some(number) = notes::midi(&note.pitch) {
            if !(36..=96).contains(&number) {
                warn(
                    &mut out,
                    format!(
                        "note {index} @ beat {}: pitch {} (MIDI {number}) is outside C2..C7",
                        note.start, note.pitch
                    ),
                );
            }
        }
        if note.beats > 8.0 {
            warn(
                &mut out,
                format!(
                    "note {index} @ beat {}: duration {} beats is very long",
                    note.start, note.beats
                ),
            );
        }
        if note.velocity == 0.0 {
            warn(
                &mut out,
                format!("note {index} @ beat {}: velocity 0 is silent", note.start),
            );
        }
        if note.pan.abs() > 0.95 {
            warn(
                &mut out,
                format!(
                    "note {index} @ beat {}: pan {} is hard-panned",
                    note.start, note.pan
                ),
            );
        }
        if note.envelope.attack > 1.0 {
            warn(
                &mut out,
                format!(
                    "note {index} @ beat {}: attack {} s is very slow",
                    note.start, note.envelope.attack
                ),
            );
        }
        if note.vibrato.rate > 20.0 || note.vibrato.cents > 100.0 {
            warn(
                &mut out,
                format!(
                    "note {index} @ beat {}: vibrato {} Hz / {} cents is extreme",
                    note.start, note.vibrato.rate, note.vibrato.cents
                ),
            );
        }

        if let Some(first) = seen.get(&(at, note.pitch.clone())) {
            warn(
                &mut out,
                format!(
                    "note {index} @ beat {}: duplicates note {first} pitch {} at the same beat",
                    note.start, note.pitch
                ),
            );
        } else {
            seen.insert((at, note.pitch.clone()), index);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    #[test]
    fn clean_score_has_no_warnings() {
        let score = score::parse("tempo 120\ngain 0.4\nnote C4 1\nat 2 E4 1\n").unwrap();
        assert!(lint(&score).is_empty());
    }

    #[test]
    fn score_level_warnings() {
        let score = score::parse("tempo 300\ngain 0.95\nvibrato 30 200\nrelease 3\n").unwrap();
        let messages: Vec<String> = lint(&score).into_iter().map(|w| w.message).collect();
        assert_eq!(
            messages,
            vec![
                "score: tempo 300 is outside 40..200",
                "score: gain 0.95 risks clipping",
                "score: vibrato rate 30 Hz is extreme",
                "score: vibrato depth 200 cents is extreme",
                "score: release 3 s is very long",
            ]
        );
    }

    #[test]
    fn note_level_warnings_are_ordered() {
        let score = score::parse("note C9 9 0 -1\nat 0 C9 1\nrest 9\n").unwrap();
        let messages: Vec<String> = lint(&score).into_iter().map(|w| w.message).collect();
        assert_eq!(
            messages,
            vec![
                "note 0 @ beat 0: pitch C9 (MIDI 120) is outside C2..C7",
                "note 0 @ beat 0: duration 9 beats is very long",
                "note 0 @ beat 0: velocity 0 is silent",
                "note 0 @ beat 0: pan -1 is hard-panned",
                "note 1 @ beat 0: pitch C9 (MIDI 120) is outside C2..C7",
                "note 1 @ beat 0: duplicates note 0 pitch C9 at the same beat",
                "note 2 @ beat 9: rest of 9 beats is very long",
            ]
        );
    }
}
