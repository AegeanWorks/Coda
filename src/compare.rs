//! Read-only comparison of two scores: structural differences plus whether
//! their rendered audio is byte-identical.

use crate::render::{render, wav_bytes};
use crate::score::Score;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comparison {
    pub identical: bool,
    pub audio_identical: bool,
    pub differences: Vec<String>,
}

fn envelope_text(envelope: &crate::envelope::Envelope) -> String {
    format!(
        "{}/{}/{}/{}",
        envelope.attack, envelope.decay, envelope.sustain, envelope.release
    )
}

fn vibrato_text(vibrato: &crate::score::Vibrato) -> String {
    format!("{}/{}", vibrato.rate, vibrato.cents)
}

pub fn compare(a: &Score, b: &Score, sample_rate: u32) -> Comparison {
    let mut differences = Vec::new();

    if a.tempo != b.tempo {
        differences.push(format!("tempo {} -> {}", a.tempo, b.tempo));
    }
    if a.wave != b.wave {
        differences.push(format!("wave {} -> {}", a.wave.name(), b.wave.name()));
    }
    if a.gain != b.gain {
        differences.push(format!("gain {} -> {}", a.gain, b.gain));
    }
    if a.envelope != b.envelope {
        differences.push(format!(
            "envelope {} -> {}",
            envelope_text(&a.envelope),
            envelope_text(&b.envelope)
        ));
    }
    if a.vibrato != b.vibrato {
        differences.push(format!(
            "vibrato {} -> {}",
            vibrato_text(&a.vibrato),
            vibrato_text(&b.vibrato)
        ));
    }
    if a.notes.len() != b.notes.len() {
        differences.push(format!("note count {} -> {}", a.notes.len(), b.notes.len()));
    }

    for index in 0..a.notes.len().min(b.notes.len()) {
        let left = &a.notes[index];
        let right = &b.notes[index];
        if left.pitch != right.pitch {
            differences.push(format!(
                "note {index} pitch {} -> {}",
                left.pitch, right.pitch
            ));
        }
        if left.start != right.start {
            differences.push(format!(
                "note {index} start {} -> {}",
                left.start, right.start
            ));
        }
        if left.beats != right.beats {
            differences.push(format!(
                "note {index} beats {} -> {}",
                left.beats, right.beats
            ));
        }
        if left.velocity != right.velocity {
            differences.push(format!(
                "note {index} velocity {} -> {}",
                left.velocity, right.velocity
            ));
        }
        if left.pan != right.pan {
            differences.push(format!("note {index} pan {} -> {}", left.pan, right.pan));
        }
        if left.envelope != right.envelope {
            differences.push(format!(
                "note {index} envelope {} -> {}",
                envelope_text(&left.envelope),
                envelope_text(&right.envelope)
            ));
        }
        if left.vibrato != right.vibrato {
            differences.push(format!(
                "note {index} vibrato {} -> {}",
                vibrato_text(&left.vibrato),
                vibrato_text(&right.vibrato)
            ));
        }
        if left.wave != right.wave {
            differences.push(format!(
                "note {index} wave {} -> {}",
                left.wave.name(),
                right.wave.name()
            ));
        }
    }

    let audio_identical = wav_bytes(&render(a, sample_rate), sample_rate)
        == wav_bytes(&render(b, sample_rate), sample_rate);
    Comparison {
        identical: differences.is_empty() && audio_identical,
        audio_identical,
        differences,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    #[test]
    fn identical_scores_have_no_differences() {
        let a = score::parse("tempo 120\nnote C4 1\n").unwrap();
        let result = compare(&a, &a, 8_000);
        assert!(result.identical);
        assert!(result.audio_identical);
        assert!(result.differences.is_empty());
    }

    #[test]
    fn differences_are_ordered_and_exact() {
        let a = score::parse("tempo 120\nwave sine\nnote C4 1\n").unwrap();
        let b = score::parse("tempo 96\nwave sine\nnote D4 1\n").unwrap();
        let result = compare(&a, &b, 8_000);
        assert_eq!(
            result.differences,
            vec!["tempo 120 -> 96", "note 0 pitch C4 -> D4"]
        );
        assert!(!result.audio_identical);
        assert!(!result.identical);
    }

    #[test]
    fn note_count_is_reported() {
        let a = score::parse("note C4 1\n").unwrap();
        let b = score::parse("note C4 1\nnote E4 1\n").unwrap();
        assert_eq!(
            compare(&a, &b, 8_000).differences,
            vec!["note count 1 -> 2"]
        );
    }

    #[test]
    fn equivalent_scores_are_audio_identical() {
        let a = score::parse("tempo 120\nnote C4 1\n").unwrap();
        let b = score::parse("note C4 1\n").unwrap();
        let result = compare(&a, &b, 8_000);
        assert!(result.identical);
        assert!(result.audio_identical);
    }
}
