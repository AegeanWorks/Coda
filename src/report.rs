//! One self-describing Markdown report for a score and optional audio.

use crate::{analyze, digest, lint, score, stats};

pub fn report(composed: &score::Score, audio: Option<&[u8]>) -> Result<String, String> {
    let canonical = score::serialize(composed);
    let fingerprint = digest::fingerprint(canonical.as_bytes());
    let warnings = lint::lint(composed);

    let mut out = String::from("# Coda report\n\n## Score\n\n");
    out.push_str(stats::report(composed).trim_end());
    out.push_str("\n\n## Lint\n\n");
    if warnings.is_empty() {
        out.push_str("no warnings");
    } else {
        for warning in warnings {
            out.push_str("- ");
            out.push_str(&warning.message);
            out.push('\n');
        }
        out.truncate(out.trim_end().len());
    }
    out.push_str("\n\n## Fingerprint\n\n");
    out.push_str(&fingerprint);

    if let Some(bytes) = audio {
        out.push_str("\n\n## Audio\n\n");
        out.push_str(analyze::analyze(bytes)?.trim_end());
    }
    out.push('\n');
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::wav_bytes;

    #[test]
    fn score_only_report_is_exact() {
        let score = score::parse("tempo 120\nwave sine\nnote C4 1\n").unwrap();
        let text = report(&score, None).unwrap();
        assert!(text.starts_with("# Coda report\n\n## Score\n\nnotes       1 sounding, 0 rests"));
        assert!(text.contains("## Lint\n\nno warnings"));
        assert!(text.contains("## Fingerprint\n\n"));
        assert!(!text.contains("## Audio"));
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn report_is_deterministic() {
        let score = score::parse("note C4 1\nnote E4 0.5\n").unwrap();
        assert_eq!(report(&score, None).unwrap(), report(&score, None).unwrap());
    }

    #[test]
    fn includes_audio_section_when_given() {
        let score = score::parse("note C4 1\n").unwrap();
        let wav = wav_bytes(&[(0.5, 0.5); 8], 8_000);
        let text = report(&score, Some(&wav)).unwrap();
        assert!(text.contains("## Audio\n\nframes      8"));
        assert!(report(&score, Some(b"bad")).is_err());
    }

    #[test]
    fn lists_lint_warnings() {
        let score = score::parse("tempo 300\nnote C9 1\n").unwrap();
        let text = report(&score, None).unwrap();
        assert!(text.contains("## Lint\n\n- score: tempo 300"));
        assert!(text.contains("- note 0 @ beat 0: pitch C9"));
    }
}
