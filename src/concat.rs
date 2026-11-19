//! Merge several scores into one canonical arrangement.

use crate::score::Score;

fn span(score: &Score) -> f64 {
    score
        .notes
        .iter()
        .map(|n| n.start + n.beats)
        .fold(0.0_f64, f64::max)
}

pub fn concat_scores(scores: &[Score]) -> Result<Score, String> {
    let first = scores
        .first()
        .ok_or_else(|| "no scores to concatenate".to_string())?;
    let mut result = first.clone();
    let mut cursor = span(first);
    for score in scores.iter().skip(1) {
        for note in &score.notes {
            let mut shifted = note.clone();
            shifted.start += cursor;
            result.notes.push(shifted);
        }
        cursor += span(score);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    #[test]
    fn single_score_is_an_identity() {
        let a = score::parse("tempo 96\nwave saw\nnote C4 1\n").unwrap();
        assert_eq!(concat_scores(std::slice::from_ref(&a)).unwrap(), a);
    }

    #[test]
    fn offsets_following_scores() {
        let a = score::parse("tempo 120\nnote C4 1\n").unwrap();
        let b = score::parse("tempo 200\nnote E4 0.5\n").unwrap();
        let merged = concat_scores(&[a, b]).unwrap();
        assert_eq!(merged.notes.len(), 2);
        assert_eq!(merged.notes[0].start, 0.0);
        assert_eq!(merged.notes[1].start, 1.0);
        assert_eq!(merged.tempo, 120.0, "the first score's settings win");
    }

    #[test]
    fn roundtrips_through_the_serializer() {
        let a = score::parse("preset pad\nnote C4 0.5\n").unwrap();
        let b = score::parse("wave saw\nnote E4 0.5\n").unwrap();
        let merged = concat_scores(&[a, b]).unwrap();
        assert_eq!(score::parse(&score::serialize(&merged)).unwrap(), merged);
    }

    #[test]
    fn rejects_empty_input() {
        assert!(concat_scores(&[]).is_err());
    }
}
