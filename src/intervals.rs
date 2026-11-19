//! Deterministic melodic-interval analysis of a score.

use crate::notes;
use crate::score::Score;

pub fn intervals(score: &Score) -> Result<String, String> {
    let pitches: Vec<i32> = score
        .notes
        .iter()
        .filter(|n| n.pitch != "-")
        .filter_map(|n| notes::midi(&n.pitch))
        .collect();
    if pitches.is_empty() {
        return Err("score has no sounding pitches".into());
    }
    let steps: Vec<i32> = pitches.windows(2).map(|w| w[1] - w[0]).collect();
    let sequence = if steps.is_empty() {
        "-".to_string()
    } else {
        steps
            .iter()
            .map(|s| format!("{s:+}"))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let smallest = steps.iter().copied().min().unwrap_or(0);
    let largest = steps.iter().copied().max().unwrap_or(0);
    let up = steps.iter().filter(|s| **s > 0).count();
    let down = steps.iter().filter(|s| **s < 0).count();
    let level = steps.iter().filter(|s| **s == 0).count();

    Ok(format!(
        "notes       {}\n\
         intervals   {}\n\
         sequence    {sequence}\n\
         smallest    {smallest:+}\n\
         largest     {largest:+}\n\
         direction   up {up} / down {down} / level {level}\n",
        pitches.len(),
        steps.len(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    #[test]
    fn reports_a_known_melody() {
        let source = score::parse("note C4 1\nnote E4 1\nnote G4 1\nnote E4 1\n").unwrap();
        let report = intervals(&source).unwrap();
        assert!(report.contains("notes       4"), "{report}");
        assert!(report.contains("intervals   3"), "{report}");
        assert!(report.contains("sequence    +4 +3 -3"), "{report}");
        assert!(report.contains("smallest    -3"), "{report}");
        assert!(report.contains("largest     +4"), "{report}");
        assert!(
            report.contains("direction   up 2 / down 1 / level 0"),
            "{report}"
        );
    }

    #[test]
    fn skips_rests() {
        let source = score::parse("note C4 1\nrest 1\nnote G4 1\n").unwrap();
        let report = intervals(&source).unwrap();
        assert!(report.contains("notes       2"), "{report}");
        assert!(report.contains("sequence    +7"), "{report}");
    }

    #[test]
    fn handles_a_single_note() {
        let source = score::parse("note C4 1\n").unwrap();
        let report = intervals(&source).unwrap();
        assert!(report.contains("intervals   0"), "{report}");
        assert!(report.contains("sequence    -"), "{report}");
        assert!(
            report.contains("direction   up 0 / down 0 / level 0"),
            "{report}"
        );
    }

    #[test]
    fn rejects_an_empty_score() {
        assert!(intervals(&score::parse("tempo 120\n").unwrap()).is_err());
    }
}
