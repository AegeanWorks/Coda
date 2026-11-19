//! Deterministic, read-only score statistics.

use crate::notes;
use crate::score::Score;
use std::collections::BTreeSet;

pub fn report(score: &Score) -> String {
    let sounding: Vec<_> = score.notes.iter().filter(|n| n.pitch != "-").collect();
    let rests = score.notes.len() - sounding.len();
    let beats: f64 = score
        .notes
        .iter()
        .map(|n| n.start + n.beats)
        .fold(0.0_f64, f64::max);
    let seconds = beats * 60.0 / score.tempo;
    let density = if beats > 0.0 {
        sounding.len() as f64 / beats
    } else {
        0.0
    };

    let pitches: BTreeSet<&str> = sounding.iter().map(|n| n.pitch.as_str()).collect();
    let numbers: Vec<i32> = sounding
        .iter()
        .filter_map(|n| notes::midi(&n.pitch))
        .collect();
    let range = if let (Some(low), Some(high)) = (numbers.iter().min(), numbers.iter().max()) {
        format!(
            "{}..{} (MIDI {}..{})",
            notes::name(*low),
            notes::name(*high),
            low,
            high
        )
    } else {
        "none".to_string()
    };

    let waves: BTreeSet<&str> = sounding.iter().map(|n| n.wave.name()).collect();
    let pan_min = sounding.iter().map(|n| n.pan).fold(f64::INFINITY, f64::min);
    let pan_max = sounding
        .iter()
        .map(|n| n.pan)
        .fold(f64::NEG_INFINITY, f64::max);
    let pan = if sounding.is_empty() {
        "none".to_string()
    } else {
        format!("{pan_min:.3}..{pan_max:.3}")
    };

    format!(
        "notes       {} sounding, {} rests\n\
         duration    {beats} beats / {seconds:.3}s\n\
         pitch range {range}\n\
         distinct    {}\n\
         density     {density:.3} notes/beat\n\
         waves       {}\n\
         pan         {pan}\n",
        sounding.len(),
        rests,
        pitches.len(),
        waves.into_iter().collect::<Vec<_>>().join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    #[test]
    fn exact_report() {
        let score =
            score::parse("tempo 120\nnote C4 0.5 1 -0.5\nat 1 A4 0.5 1 0.5\nrest 1.5\n").unwrap();
        assert_eq!(
            report(&score),
            concat!(
                "notes       2 sounding, 1 rests\n",
                "duration    2 beats / 1.000s\n",
                "pitch range C4..A4 (MIDI 60..69)\n",
                "distinct    2\n",
                "density     1.000 notes/beat\n",
                "waves       triangle\n",
                "pan         -0.500..0.500\n",
            )
        );
    }

    #[test]
    fn empty_score_is_safe() {
        let score = score::parse("tempo 120\n").unwrap();
        let text = report(&score);
        assert!(text.contains("notes       0 sounding, 0 rests"));
        assert!(text.contains("pitch range none"));
        assert!(text.contains("density     0.000 notes/beat"));
        assert!(text.contains("pan         none"));
    }

    #[test]
    fn mixed_waves_are_sorted() {
        let score = score::parse("wave saw\nnote C4 1\ninst sine\nnote E4 1\n").unwrap();
        assert!(report(&score).contains("waves       saw, sine"));
    }
}
