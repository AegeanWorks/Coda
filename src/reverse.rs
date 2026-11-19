//! Time reversal of a score: mirror every event around the score's end.

use crate::score::Score;

pub fn reverse(score: &Score) -> Score {
    let total = score
        .notes
        .iter()
        .map(|n| n.start + n.beats)
        .fold(0.0_f64, f64::max);
    let mut result = score.clone();
    for note in &mut result.notes {
        note.start = total - (note.start + note.beats);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    #[test]
    fn mirrors_starts_around_the_end() {
        let source = score::parse("note C4 1\nnote E4 0.5\nat 3 G4 1\n").unwrap();
        let reversed = reverse(&source);
        let starts: Vec<f64> = reversed.notes.iter().map(|n| n.start).collect();
        assert_eq!(starts, vec![3.0, 2.5, 0.0]);
    }

    #[test]
    fn double_reverse_is_an_exact_identity() {
        for source in [
            "note C4 1\nat 2 E4 0.5\nat 2 G4 0.5\nrest 1\n",
            "preset pad\nnote C4 0.25\nnote E4 0.25\nnote G4 0.25\n",
        ] {
            let score = score::parse(source).unwrap();
            assert_eq!(reverse(&reverse(&score)), score, "{source}");
            assert_eq!(
                score::serialize(&reverse(&reverse(&score))),
                score::serialize(&score)
            );
        }
    }

    #[test]
    fn only_start_changes() {
        let source = score::parse("wave saw\nnote C4 1 0.7 -0.4\nnote E4 1\n").unwrap();
        let reversed = reverse(&source);
        for (before, after) in source.notes.iter().zip(reversed.notes.iter()) {
            assert_eq!(before.pitch, after.pitch);
            assert_eq!(before.beats, after.beats);
            assert_eq!(before.velocity, after.velocity);
            assert_eq!(before.pan, after.pan);
            assert_eq!(before.envelope, after.envelope);
            assert_eq!(before.vibrato, after.vibrato);
            assert_eq!(before.wave, after.wave);
        }
    }

    #[test]
    fn empty_score_is_safe() {
        let score = score::parse("tempo 120\n").unwrap();
        assert_eq!(reverse(&score), score);
    }
}
