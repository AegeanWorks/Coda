//! End-to-end checks: parse a score, render it, encode a WAV, fingerprint it.

use coda::digest;
use coda::gen::{self, GenOptions, Scale};
use coda::notes;
use coda::render::{render, wav_bytes, Frame};
use coda::score;
use coda::wave::Wave;
use std::process::Command;

const SCALE_FILE: &str = include_str!("../examples/scale.coda");

const SCALE: &str = "\
# a simple ascending figure
tempo 100
wave triangle
gain 0.4
attack 0.01
decay 0.05
sustain 0.6
release 0.12

note C4 0.5
note D4 0.5
note E4 0.5
note G4 0.5
at 2.5 A4 1.5
";

#[test]
fn score_to_wav_pipeline() {
    let parsed = score::parse(SCALE).unwrap();
    let samples = render(&parsed, 44_100);
    let wav = wav_bytes(&samples, 44_100);

    assert!(samples.len() > 44_100);
    assert_eq!(&wav[0..4], b"RIFF");
    assert_eq!(&wav[8..12], b"WAVE");
    assert_eq!(&wav[36..40], b"data");
    assert_eq!(wav.len(), 44 + samples.len() * 4);
    assert_eq!(u16::from_le_bytes([wav[22], wav[23]]), 2);
}

#[test]
fn identical_scores_produce_identical_bytes() {
    let a = wav_bytes(&render(&score::parse(SCALE).unwrap(), 22_050), 22_050);
    let b = wav_bytes(&render(&score::parse(SCALE).unwrap(), 22_050), 22_050);
    assert_eq!(a, b);
    assert_eq!(digest::fingerprint(&a), digest::fingerprint(&b));
}

#[test]
fn different_scores_produce_different_bytes() {
    let a = wav_bytes(
        &render(&score::parse("wave sine\nnote C4 1\n").unwrap(), 8_000),
        8_000,
    );
    let b = wav_bytes(
        &render(&score::parse("wave saw\nnote C4 1\n").unwrap(), 8_000),
        8_000,
    );
    assert_ne!(a, b);
    assert_ne!(digest::fingerprint(&a), digest::fingerprint(&b));
}

#[test]
fn velocity_changes_loudness() {
    let loud = score::parse("wave sine\nnote A4 1 1.0\n").unwrap();
    let soft = score::parse("wave sine\nnote A4 1 0.2\n").unwrap();
    let a = render(&loud, 8_000);
    let b = render(&soft, 8_000);
    let peak = |v: &[Frame]| {
        v.iter()
            .map(|f| f.0.abs().max(f.1.abs()))
            .fold(0.0_f64, f64::max)
    };
    assert!(peak(&a) > peak(&b) * 2.0);
}

#[test]
fn rests_are_silent_but_take_time() {
    let score = score::parse("tempo 60\nrest 2\n").unwrap();
    let samples = render(&score, 8_000);
    assert!(samples.iter().all(|f| f.0 == 0.0 && f.1 == 0.0));
    assert_eq!(
        samples.len(),
        16_000 + (score.envelope.tail() * 8_000.0) as usize
    );
}

#[test]
fn generative_mode_is_repeatable_and_scale_aware() {
    let opts = GenOptions {
        bars: 2,
        wave: Wave::Saw,
        ..GenOptions::default()
    };
    let a = gen::generate(2026, opts);
    let b = gen::generate(2026, opts);
    assert_eq!(a, b);
    assert!(a.notes.len() > 4);
    assert!(a
        .notes
        .iter()
        .all(|n| n.pitch == "-" || notes::frequency(&n.pitch).is_some()));

    let minor = GenOptions {
        scale: Scale::Minor,
        ..opts
    };
    assert_ne!(gen::generate(2026, minor), a);

    assert_eq!(
        digest::fingerprint(&wav_bytes(&render(&a, 8_000), 8_000)),
        digest::fingerprint(&wav_bytes(&render(&b, 8_000), 8_000))
    );
}

#[test]
fn per_note_envelope_changes_render() {
    let plain = score::parse("wave sine\nnote A4 1\n").unwrap();
    let sharp = score::parse("wave sine\nenv 0 0 1 0\nnote A4 1\n").unwrap();
    let a = render(&plain, 8_000);
    let b = render(&sharp, 8_000);
    assert_ne!(a, b);
    assert_eq!(b, render(&sharp, 8_000));
}

#[test]
fn vibrato_changes_the_waveform_deterministically() {
    let plain = score::parse("tempo 120\nwave sine\nnote A4 1\n").unwrap();
    let vib = score::parse("tempo 120\nwave sine\nvibrato 6 35\nnote A4 1\n").unwrap();
    let a = render(&plain, 8_000);
    let b = render(&vib, 8_000);
    assert_ne!(a, b);
    assert_eq!(b, render(&vib, 8_000));
}

#[test]
fn repeat_matches_written_out_notes() {
    let compact = score::parse("repeat 3\nnote C4 0.5\n").unwrap();
    let written = score::parse("note C4 0.5\nnote C4 0.5\nnote C4 0.5\n").unwrap();
    assert_eq!(compact, written);
    assert_eq!(
        digest::fingerprint(&wav_bytes(&render(&compact, 8_000), 8_000)),
        digest::fingerprint(&wav_bytes(&render(&written, 8_000), 8_000))
    );
}

#[test]
fn golden_vectors_pin_the_renderer() {
    let score = score::parse(SCALE_FILE).unwrap();
    let at_44100 = wav_bytes(&render(&score, 44_100), 44_100);
    let at_22050 = wav_bytes(&render(&score, 22_050), 22_050);
    assert_eq!(digest::fingerprint(&at_44100), "b47a4f9fedd35262");
    assert_eq!(digest::fingerprint(&at_22050), "8e9803773a2eddba");
}

#[test]
fn cli_album_is_deterministic() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-album-{}", std::process::id()));
    let a = root.join("a");
    let b = root.join("b");
    let _ = std::fs::remove_dir_all(&root);

    for dir in [&a, &b] {
        let out = Command::new(exe)
            .args([
                "album", "42", "--tracks", "2", "--bars", "1", "--scale", "minor", "-o",
            ])
            .arg(dir)
            .output()
            .expect("coda binary should run");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    for name in ["track-01.wav", "track-02.wav"] {
        assert_eq!(
            std::fs::read(a.join(name)).unwrap(),
            std::fs::read(b.join(name)).unwrap(),
            "{name} should be byte-identical"
        );
    }
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_info_and_digest_agree_with_library() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let out = Command::new(exe)
        .args(["digest", "examples/scale.coda", "--sample-rate", "22050"])
        .output()
        .expect("coda binary should run");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.starts_with("8e9803773a2eddba"), "{text}");
}

#[test]
fn cli_style_flag_changes_the_composition() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-style-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);

    let mut digests = Vec::new();
    for style in ["flow", "arp", "chords"] {
        let path = root.join(format!("{style}.wav"));
        let out = Command::new(exe)
            .args(["seed", "11", "--bars", "1", "--style", style, "-o"])
            .arg(&path)
            .output()
            .expect("coda binary should run");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        digests.push(digest::fingerprint(&std::fs::read(path).unwrap()));
    }
    digests.sort();
    digests.dedup();
    assert_eq!(digests.len(), 3, "each style should render differently");

    let bad = Command::new(exe)
        .args(["seed", "11", "--style", "bogus"])
        .output()
        .expect("coda binary should run");
    assert_eq!(bad.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn malformed_scores_fail_with_line_numbers() {
    assert!(score::parse("tempo 120\nwave nonsense\n")
        .unwrap_err()
        .contains("line 2"));
    assert!(score::parse("at 1 C4\n").unwrap_err().contains("at beats"));
    assert!(score::parse("rest\n").unwrap_err().contains("rest beats"));
}
