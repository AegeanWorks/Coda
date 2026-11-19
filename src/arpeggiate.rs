//! Roll simultaneous notes into a sequence on a fixed grid.

use crate::score::Score;
use std::collections::BTreeMap;

pub fn arpeggiate(score: &Score, grid: f64) -> Result<Score, String> {
    if !grid.is_finite() || grid <= 0.0 {
        return Err("grid must be a positive finite number".into());
    }
    let mut groups: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
    for (index, note) in score.notes.iter().enumerate() {
        let key = (note.start / grid).round() as i64;
        groups.entry(key).or_default().push(index);
    }

    let mut result = score.clone();
    for indices in groups.values() {
        for (position, &index) in indices.iter().enumerate() {
            let base = score.notes[indices[0]].start;
            result.notes[index].start = base + position as f64 * grid;
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    fn chord() -> Score {
        score::parse("note C4 1\nat 0 E4 1\nat 0 G4 1\nnote A4 0.5\n").unwrap()
    }

    #[test]
    fn spreads_a_chord_on_the_grid() {
        let result = arpeggiate(&chord(), 0.25).unwrap();
        let starts: Vec<f64> = result.notes.iter().map(|n| n.start).collect();
        assert_eq!(starts, vec![0.0, 0.25, 0.5, 1.0]);
        assert_eq!(result.notes.len(), 4);
    }

    #[test]
    fn preserves_every_note_field() {
        let source = score::parse("wave saw\nnote C4 0.5 0.7 -0.4\n").unwrap();
        let result = arpeggiate(&source, 0.25).unwrap();
        assert_eq!(result.notes[0].pitch, source.notes[0].pitch);
        assert_eq!(result.notes[0].beats, source.notes[0].beats);
        assert_eq!(result.notes[0].velocity, source.notes[0].velocity);
        assert_eq!(result.notes[0].pan, source.notes[0].pan);
        assert_eq!(result.notes[0].wave, source.notes[0].wave);
    }

    #[test]
    fn roundtrips_and_rejects_bad_grids() {
        let result = arpeggiate(&chord(), 0.25).unwrap();
        assert_eq!(score::parse(&score::serialize(&result)).unwrap(), result);
        assert!(arpeggiate(&chord(), 0.0).is_err());
        assert!(arpeggiate(&chord(), -1.0).is_err());
    }
}
