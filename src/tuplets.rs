//! Compress or expand a beat window into a tuplet.

use crate::score::Score;

fn remap(time: f64, start: f64, end: f64, ratio: f64) -> f64 {
    if time <= start {
        time
    } else if time >= end {
        start + (end - start) * ratio + (time - end)
    } else {
        start + (time - start) * ratio
    }
}

pub fn tuplets(score: &Score, start: f64, end: f64, ratio: f64) -> Result<Score, String> {
    if !start.is_finite() || !end.is_finite() || start < 0.0 || end <= start {
        return Err("range must satisfy 0 <= start < end".into());
    }
    if !ratio.is_finite() || ratio <= 0.0 {
        return Err("ratio must be a positive finite number".into());
    }
    let mut result = score.clone();
    for note in &mut result.notes {
        let from = remap(note.start, start, end, ratio);
        let note_end = note.start + note.beats;
        let to = remap(note_end, start, end, ratio);
        note.start = from;
        note.beats = to - from;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    fn sample() -> Score {
        score::parse("note C4 1\nat 1 E4 1\nat 2 G4 1\nat 3 A4 1\n").unwrap()
    }

    #[test]
    fn compresses_only_the_window() {
        let result = tuplets(&sample(), 1.0, 3.0, 0.5).unwrap();
        let pairs: Vec<(f64, f64)> = result.notes.iter().map(|n| (n.start, n.beats)).collect();
        assert_eq!(pairs, vec![(0.0, 1.0), (1.0, 0.5), (1.5, 0.5), (2.0, 1.0),]);
    }

    #[test]
    fn notes_before_are_untouched_and_after_are_shifted() {
        let source = sample();
        let result = tuplets(&source, 1.0, 3.0, 0.5).unwrap();
        assert_eq!(result.notes[0], source.notes[0], "before the window");
        assert!(
            (result.notes[3].start - 2.0).abs() < 1e-9,
            "shifted by the delta"
        );
        assert_eq!(
            result.notes[3].beats, source.notes[3].beats,
            "duration kept"
        );
    }

    #[test]
    fn preserves_timbre_and_roundtrips() {
        let source = score::parse("wave saw\nnote C4 1 0.7 -0.4\nat 1 E4 1\n").unwrap();
        let result = tuplets(&source, 0.0, 2.0, 0.5).unwrap();
        assert_eq!(result.wave, source.wave);
        assert_eq!(result.notes[0].pitch, "C4");
        assert_eq!(result.notes[0].velocity, source.notes[0].velocity);
        assert_eq!(score::parse(&score::serialize(&result)).unwrap(), result);
    }

    #[test]
    fn rejects_bad_inputs() {
        let s = sample();
        assert!(tuplets(&s, 1.0, 1.0, 0.5).is_err());
        assert!(tuplets(&s, 2.0, 1.0, 0.5).is_err());
        assert!(tuplets(&s, 0.0, 2.0, 0.0).is_err());
    }
}
