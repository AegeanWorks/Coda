//! Snap sounding pitches to the nearest degree of a chosen scale.

use crate::gen::Scale;
use crate::notes;
use crate::score::Score;

pub fn snap_scale(score: &Score, root: &str, scale: Scale) -> Result<Score, String> {
    let pivot = notes::midi(root).ok_or_else(|| format!("invalid root note '{root}'"))?;
    let degrees = scale.degrees();

    let mut result = score.clone();
    for note in &mut result.notes {
        if note.pitch == "-" {
            continue;
        }
        let Some(number) = notes::midi(&note.pitch) else {
            continue;
        };
        let relative = number - pivot;
        let octave = relative.div_euclid(12);
        let class = relative.rem_euclid(12);

        // Degrees are ascending, so strict `<` keeps the lower one on a tie.
        let mut best = class;
        let mut best_distance = 12_i32;
        for &degree in degrees {
            let distance = (class - degree).abs();
            if distance < best_distance {
                best_distance = distance;
                best = degree;
            }
        }
        let target = pivot + octave * 12 + best;
        if (0..=127).contains(&target) {
            note.pitch = notes::name(target);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    fn sample() -> Score {
        score::parse("tempo 120\nwave saw\nnote C4 1 0.8\nnote C#4 1\nnote D#4 1\nnote F#4 1\n")
            .unwrap()
    }

    #[test]
    fn snaps_to_nearest_major_degree_with_lower_on_ties() {
        let result = snap_scale(&sample(), "C4", Scale::Major).unwrap();
        let pitches: Vec<&str> = result.notes.iter().map(|n| n.pitch.as_str()).collect();
        assert_eq!(pitches, vec!["C4", "C4", "D4", "F4"]);
    }

    #[test]
    fn handles_roots_other_than_c() {
        let source = score::parse("note D4 1\nnote D#4 1\n").unwrap();
        let result = snap_scale(&source, "D4", Scale::Minor).unwrap();
        let pitches: Vec<&str> = result.notes.iter().map(|n| n.pitch.as_str()).collect();
        assert_eq!(pitches, vec!["D4", "D4"]);
    }

    #[test]
    fn preserves_timing_and_timbre_and_roundtrips() {
        let source = sample();
        let result = snap_scale(&source, "C4", Scale::Pentatonic).unwrap();
        assert_eq!(result.tempo, source.tempo);
        assert_eq!(result.wave, source.wave);
        assert_eq!(result.notes[0].beats, source.notes[0].beats);
        assert_eq!(result.notes[0].velocity, source.notes[0].velocity);
        assert_eq!(score::parse(&score::serialize(&result)).unwrap(), result);
    }

    #[test]
    fn rejects_bad_root() {
        assert!(snap_scale(&sample(), "H9", Scale::Major).is_err());
    }
}
