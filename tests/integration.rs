//! End-to-end checks: parse a score, render it, encode a WAV, fingerprint it.

use coda::digest;
use coda::gen::{self, GenOptions, Scale};
use coda::midi::midi_bytes;
use coda::notes;
use coda::render::{render, wav_bytes, Frame};
use coda::score;
use coda::transpose::transpose;
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
fn envelope_presets_change_render_and_reset() {
    let plain = score::parse("wave sine\nnote A4 1\nat 2 C5 1\n").unwrap();
    let marked = score::parse("wave sine\nnote A4 1\npreset pluck\nat 2 C5 1\n").unwrap();
    assert_ne!(
        wav_bytes(&render(&plain, 8_000), 8_000),
        wav_bytes(&render(&marked, 8_000), 8_000)
    );

    let via_preset =
        score::parse("wave sine\npreset pad\nnote A4 1\npreset default\nat 2 C5 1\n").unwrap();
    let explicit = score::parse(
        "wave sine\nenv 0.35 0.4 0.75 0.9\nnote A4 1\nenv 0.01 0.08 0.65 0.15\nat 2 C5 1\n",
    )
    .unwrap();
    assert_eq!(
        wav_bytes(&render(&via_preset, 8_000), 8_000),
        wav_bytes(&render(&explicit, 8_000), 8_000)
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
fn per_note_wave_changes_only_the_marked_note() {
    let plain = score::parse("wave sine\nnote A4 1\nat 2 C5 1\n").unwrap();
    let marked = score::parse("wave sine\nnote A4 1\ninst organ\nat 2 C5 1\n").unwrap();
    assert_ne!(
        wav_bytes(&render(&plain, 8_000), 8_000),
        wav_bytes(&render(&marked, 8_000), 8_000)
    );

    let via_default =
        score::parse("wave sine\ninst organ\nnote A4 1\ninst default\nat 2 C5 1\n").unwrap();
    let explicit =
        score::parse("wave sine\ninst organ\nnote A4 1\nwave sine\nat 2 C5 1\n").unwrap();
    assert_eq!(via_default.notes[1].wave, Wave::Sine);
    assert_eq!(
        wav_bytes(&render(&via_default, 8_000), 8_000),
        wav_bytes(&render(&explicit, 8_000), 8_000)
    );
}

#[test]
fn generated_notes_inherit_the_chosen_wave() {
    let opts = GenOptions {
        bars: 1,
        wave: coda::wave::Wave::Pluck,
        ..GenOptions::default()
    };
    let score = gen::generate(7, opts);
    assert!(score
        .notes
        .iter()
        .all(|n| n.wave == coda::wave::Wave::Pluck));
}

#[test]
fn per_note_vibrato_changes_only_the_marked_note() {
    let plain = score::parse("wave sine\nnote A4 1\nat 2 C5 1\n").unwrap();
    let marked = score::parse("wave sine\nnote A4 1\nvib 7 40\nat 2 C5 1\n").unwrap();
    assert_ne!(
        wav_bytes(&render(&plain, 8_000), 8_000),
        wav_bytes(&render(&marked, 8_000), 8_000)
    );

    let via_default =
        score::parse("wave sine\nvib 7 40\nnote A4 1\nvib default\nat 2 C5 1\n").unwrap();
    let via_score =
        score::parse("wave sine\nvibrato 7 40\nnote A4 1\nvibrato 0 0\nat 2 C5 1\n").unwrap();
    assert_eq!(
        wav_bytes(&render(&via_default, 8_000), 8_000),
        wav_bytes(&render(&via_score, 8_000), 8_000)
    );
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

    for name in ["track-01.wav", "track-02.wav", "manifest.json"] {
        assert_eq!(
            std::fs::read(a.join(name)).unwrap(),
            std::fs::read(b.join(name)).unwrap(),
            "{name} should be byte-identical"
        );
    }
    let manifest = std::fs::read_to_string(a.join("manifest.json")).unwrap();
    assert!(manifest.contains("\"seed\": 42"));
    assert!(manifest.contains("\"scale\": \"minor\""));
    assert!(manifest.contains("\"album_digest\":"));
    assert!(manifest.contains("\"track_digests\": ["));
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
fn midi_export_is_deterministic_and_well_formed() {
    let score = score::parse("tempo 120\nnote C4 1\nat 2 G4 1\n").unwrap();
    let a = midi_bytes(&score);
    assert_eq!(&a[0..4], b"MThd");
    assert_eq!(&a[14..18], b"MTrk");
    assert!(a.ends_with(&[0xff, 0x2f, 0x00]));
    assert_eq!(a, midi_bytes(&score));
    assert_ne!(
        a,
        midi_bytes(&score::parse("tempo 90\nnote C4 1\nat 2 G4 1\n").unwrap())
    );
}

#[test]
fn cli_extension_routes_midi_and_wav() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-midi-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let midi_path = root.join("scale.mid");
    let wav_path = root.join("scale.wav");

    for path in [&midi_path, &wav_path] {
        let out = Command::new(exe)
            .args(["render", "examples/scale.coda", "-o"])
            .arg(path)
            .output()
            .expect("coda binary should run");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    assert_eq!(&std::fs::read(&midi_path).unwrap()[0..4], b"MThd");
    assert_eq!(&std::fs::read(&wav_path).unwrap()[0..4], b"RIFF");

    let seed_midi = root.join("seed.midi");
    let out = Command::new(exe)
        .args(["seed", "42", "--bars", "1", "-o"])
        .arg(&seed_midi)
        .output()
        .expect("coda binary should run");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(&std::fs::read(&seed_midi).unwrap()[0..4], b"MThd");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn inverse_transpose_restores_exact_audio() {
    let score = score::parse(SCALE_FILE).unwrap();
    let original = wav_bytes(&render(&score, 8_000), 8_000);
    let shifted = transpose(&score, 7);
    let restored = transpose(&shifted, -7);
    assert_ne!(original, wav_bytes(&render(&shifted, 8_000), 8_000));
    assert_eq!(original, wav_bytes(&render(&restored, 8_000), 8_000));
}

#[test]
fn cli_semitones_changes_output_and_source() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let plain = Command::new(exe)
        .args(["digest", "examples/scale.coda", "--sample-rate", "8000"])
        .output()
        .unwrap();
    let shifted = Command::new(exe)
        .args([
            "digest",
            "examples/scale.coda",
            "--sample-rate",
            "8000",
            "--semitones",
            "12",
        ])
        .output()
        .unwrap();
    assert!(plain.status.success());
    assert!(shifted.status.success());
    assert_ne!(plain.stdout, shifted.stdout);

    let root = std::env::temp_dir().join(format!("coda-transpose-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let source = root.join("up.coda");
    let out = Command::new(exe)
        .args(["render", "examples/scale.coda", "--semitones", "-12", "-o"])
        .arg(&source)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = std::fs::read_to_string(&source).unwrap();
    assert!(text.contains("note C3 "), "{text}");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_wave_override_reaches_parsed_notes() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let base = Command::new(exe)
        .args(["digest", "examples/scale.coda", "--sample-rate", "8000"])
        .output()
        .unwrap();
    let overridden = Command::new(exe)
        .args([
            "digest",
            "examples/scale.coda",
            "--sample-rate",
            "8000",
            "--wave",
            "sine",
        ])
        .output()
        .unwrap();
    assert!(base.status.success());
    assert!(overridden.status.success());
    assert_ne!(
        base.stdout, overridden.stdout,
        "--wave must change rendered notes"
    );
}

#[test]
fn cli_score_export_is_parseable_source() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-roundtrip-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let generated = root.join("generated.coda");
    let exported = root.join("exported.coda");

    let out = Command::new(exe)
        .args(["seed", "42", "--bars", "2", "--style", "chords", "-o"])
        .arg(&generated)
        .output()
        .expect("coda binary should run");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = Command::new(exe)
        .args(["export-score", "examples/scale.coda", "-o"])
        .arg(&exported)
        .output()
        .expect("coda binary should run");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    for path in [&generated, &exported] {
        let text = std::fs::read_to_string(path).unwrap();
        assert!(text.starts_with("tempo "), "{text}");
        assert!(text.contains("env "), "{text}");
        let checked = Command::new(exe).arg("lint").arg(path).output().unwrap();
        assert!(
            checked.status.success(),
            "{}",
            String::from_utf8_lossy(&checked.stderr)
        );
    }
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_vary_is_seeded_and_writes_parseable_source() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-vary-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let mut outputs = Vec::new();

    for (name, seed) in [("a.coda", "42"), ("b.coda", "42"), ("c.coda", "43")] {
        let path = root.join(name);
        let out = Command::new(exe)
            .args([
                "vary",
                "examples/scale.coda",
                "--seed",
                seed,
                "--amount",
                "0.8",
                "-o",
            ])
            .arg(&path)
            .output()
            .expect("coda binary should run");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("tempo "), "{text}");
        let checked = Command::new(exe).arg("lint").arg(&path).output().unwrap();
        assert!(checked.status.success());
        outputs.push(std::fs::read(&path).unwrap());
    }
    assert_eq!(outputs[0], outputs[1], "same seed must be identical");
    assert_ne!(outputs[0], outputs[2], "different seeds must differ");

    let identity = root.join("identity.coda");
    let zero = root.join("zero.coda");
    for (command, path) in [("export-score", &identity), ("vary", &zero)] {
        let mut args = vec![command, "examples/scale.coda"];
        if command == "vary" {
            args.extend(["--seed", "42", "--amount", "0"]);
        }
        args.push("-o");
        let out = Command::new(exe).args(args).arg(path).output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    assert_eq!(
        std::fs::read(&identity).unwrap(),
        std::fs::read(&zero).unwrap()
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_compare_has_exact_deterministic_output() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-compare-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let a = root.join("a.coda");
    let b = root.join("b.coda");
    std::fs::write(&a, "tempo 120\nwave sine\nnote C4 1\n").unwrap();
    std::fs::write(&b, "tempo 96\nwave sine\nnote D4 1\n").unwrap();

    let out = Command::new(exe)
        .arg("compare")
        .arg(&a)
        .arg(&a)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "identical: yes\naudio: identical\n"
    );

    let out = Command::new(exe)
        .arg("compare")
        .arg(&a)
        .arg(&b)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        concat!(
            "identical: no\n",
            "difference: tempo 120 -> 96\n",
            "difference: note 0 pitch C4 -> D4\n",
            "audio: different\n",
        )
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_lint_has_exact_deterministic_output() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-lint-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let bad = root.join("bad.coda");
    let good = root.join("good.coda");
    std::fs::write(&bad, "tempo 300\nnote C9 9 0 -1\n").unwrap();
    std::fs::write(&good, "tempo 120\nnote C4 1\n").unwrap();

    let out = Command::new(exe).arg("lint").arg(&bad).output().unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        concat!(
            "WARN score: tempo 300 is outside 40..200\n",
            "WARN note 0 @ beat 0: pitch C9 (MIDI 120) is outside C2..C7\n",
            "WARN note 0 @ beat 0: duration 9 beats is very long\n",
            "WARN note 0 @ beat 0: velocity 0 is silent\n",
            "WARN note 0 @ beat 0: pan -1 is hard-panned\n",
        )
    );

    let out = Command::new(exe).arg("lint").arg(&good).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout), "OK: no warnings\n");

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
