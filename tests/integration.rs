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
fn cli_midi_export_then_import_preserves_audio() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-midi-roundtrip-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let source = root.join("source.coda");
    let midi = root.join("scale.mid");
    let imported = root.join("imported.coda");
    // MIDI cannot carry waveform, gain or envelope: use score defaults so the
    // supported subset round-trips to byte-identical audio.
    std::fs::write(&source, "tempo 120\nnote C4 1\nat 2 G4 0.5\n").unwrap();

    let out = Command::new(exe)
        .arg("render")
        .arg(&source)
        .arg("-o")
        .arg(&midi)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = Command::new(exe)
        .arg("import-midi")
        .arg(&midi)
        .arg("-o")
        .arg(&imported)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let original = Command::new(exe)
        .arg("digest")
        .arg(&source)
        .args(["--sample-rate", "8000"])
        .output()
        .unwrap();
    let restored = Command::new(exe)
        .arg("digest")
        .arg(&imported)
        .args(["--sample-rate", "8000"])
        .output()
        .unwrap();
    assert!(original.status.success());
    assert!(
        restored.status.success(),
        "{}",
        String::from_utf8_lossy(&restored.stderr)
    );
    assert_eq!(
        original.stdout, restored.stdout,
        "MIDI round-trip must preserve audio"
    );

    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_normalize_raises_peak_and_is_idempotent() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-normalize-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let source = root.join("quiet.coda");
    let input = root.join("quiet.wav");
    let once = root.join("once.wav");
    let twice = root.join("twice.wav");
    std::fs::write(&source, "gain 0.05\nnote A4 1\n").unwrap();

    let rendered = Command::new(exe)
        .arg("render")
        .arg(&source)
        .arg("-o")
        .arg(&input)
        .output()
        .unwrap();
    assert!(rendered.status.success());

    for (from, to) in [(&input, &once), (&once, &twice)] {
        let result = Command::new(exe)
            .arg("normalize")
            .arg(from)
            .args(["--peak", "0.9", "-o"])
            .arg(to)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let bytes = std::fs::read(&once).unwrap();
    let peak = bytes[44..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| (i16::from_le_bytes([c[0], c[1]]) as f64 / 32767.0).abs())
        .fold(0.0_f64, f64::max);
    assert!((peak - 0.9).abs() < 0.001, "{peak}");
    assert_eq!(bytes, std::fs::read(&twice).unwrap());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_join_concatenates_sections_byte_for_byte() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-join-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let a = root.join("a.coda");
    let b = root.join("b.coda");
    let out1 = root.join("one.wav");
    let out2 = root.join("two.wav");
    std::fs::write(&a, "note C4 1\n").unwrap();
    std::fs::write(&b, "note E4 0.5\n").unwrap();

    for out in [&out1, &out2] {
        let result = Command::new(exe)
            .arg("join")
            .arg(&a)
            .arg(&b)
            .arg("-o")
            .arg(out)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(String::from_utf8_lossy(&result.stdout).contains("2 sections"));
    }
    let joined = std::fs::read(&out1).unwrap();
    assert_eq!(&joined[0..4], b"RIFF");
    assert_eq!(&joined[36..40], b"data");
    assert_eq!(joined, std::fs::read(&out2).unwrap());

    let single = Command::new(exe)
        .args(["render", "examples/scale.coda", "-o"])
        .arg(root.join("single.wav"))
        .output()
        .unwrap();
    assert!(single.status.success());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_imports_multi_track_midi() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-multitrack-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let midi = root.join("multitrack.mid");
    let score_path = root.join("merged.coda");

    let conductor: Vec<u8> = vec![
        0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20, 0x00, 0xff, 0x2f, 0x00,
    ];
    let first: Vec<u8> = vec![
        0x00, 0x90, 60, 100, 0x83, 0x60, 0x80, 60, 0, 0x00, 0xff, 0x2f, 0x00,
    ];
    let second: Vec<u8> = vec![
        0x83, 0x60, 0x90, 64, 100, 0x83, 0x60, 0x80, 64, 0, 0x00, 0xff, 0x2f, 0x00,
    ];
    let tracks = [conductor, first, second];

    let mut bytes: Vec<u8> = vec![b'M', b'T', b'h', b'd', 0, 0, 0, 6, 0, 1];
    bytes.extend_from_slice(&(tracks.len() as u16).to_be_bytes());
    bytes.extend_from_slice(&480_u16.to_be_bytes());
    for track in &tracks {
        bytes.extend_from_slice(b"MTrk");
        bytes.extend_from_slice(&(track.len() as u32).to_be_bytes());
        bytes.extend_from_slice(track);
    }
    std::fs::write(&midi, &bytes).unwrap();

    let out = Command::new(exe)
        .arg("import-midi")
        .arg(&midi)
        .arg("-o")
        .arg(&score_path)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("2 notes, tempo 120"), "{stdout}");

    let info = Command::new(exe)
        .arg("info")
        .arg(&score_path)
        .output()
        .unwrap();
    let info = String::from_utf8_lossy(&info.stdout);
    assert!(info.contains("2 sounding"), "{info}");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_analyze_reports_exact_measurements() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-analyze-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let wav = root.join("constant.wav");
    std::fs::write(&wav, wav_bytes(&[(0.5, 0.5); 8], 8_000)).unwrap();

    let out = Command::new(exe).arg("analyze").arg(&wav).output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        concat!(
            "frames      8\n",
            "duration    0.001s\n",
            "sample rate 8000\n",
            "peak        0.500015 (-6.02 dBFS)\n",
            "rms         0.500015 (-6.02 dBFS)\n",
        )
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_stretch_scales_time_with_exact_identity() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-stretch-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let one = root.join("one.coda");
    let two = root.join("two.coda");
    let baseline = root.join("baseline.coda");

    for (path, value) in [(&one, "1"), (&two, "2")] {
        let out = Command::new(exe)
            .args(["stretch", "examples/scale.coda", "--factor", value, "-o"])
            .arg(path)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let out = Command::new(exe)
        .args(["export-score", "examples/scale.coda", "-o"])
        .arg(&baseline)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        std::fs::read(&one).unwrap(),
        std::fs::read(&baseline).unwrap()
    );
    assert_ne!(
        std::fs::read(&two).unwrap(),
        std::fs::read(&baseline).unwrap()
    );

    let info = Command::new(exe).arg("stats").arg(&two).output().unwrap();
    let text = String::from_utf8_lossy(&info.stdout);
    assert!(text.contains("8 beats / 4.800s"), "{text}");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_augment_scales_durations_only() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-augment-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let source = root.join("source.coda");
    let out = root.join("out.coda");
    std::fs::write(&source, "note C4 1\nat 3 E4 0.5\n").unwrap();

    let result = Command::new(exe)
        .arg("augment")
        .arg(&source)
        .args(["--factor", "2", "-o"])
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.contains("note C4 2"), "{text}");
    assert!(text.contains("at 3 E4 1"), "{text}");

    let missing = Command::new(exe)
        .arg("augment")
        .arg(&source)
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_register_fits_notes_into_a_range() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-register-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let source = root.join("source.coda");
    let out = root.join("out.coda");
    std::fs::write(&source, "note C2 1\nnote C4 1\nnote C6 1\n").unwrap();

    let result = Command::new(exe)
        .arg("register")
        .arg(&source)
        .args(["--low", "C4", "--high", "C5", "-o"])
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.contains("note C4"), "{text}");
    assert!(text.contains("note C5"), "{text}");
    assert!(!text.contains("C2"), "{text}");
    assert!(!text.contains("C6"), "{text}");

    let bad = Command::new(exe)
        .arg("register")
        .arg(&source)
        .args(["--low", "C6", "--high", "C4"])
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_accent_cycles_velocities() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-accent-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let source = root.join("source.coda");
    let out = root.join("out.coda");
    std::fs::write(&source, "note C4 1\nnote E4 1\nnote G4 1\n").unwrap();

    let result = Command::new(exe)
        .arg("accent")
        .arg(&source)
        .args(["--pattern", "1,0.5", "-o"])
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.contains("note C4 1 1"), "{text}");
    assert!(text.contains("note E4 1 0.5"), "{text}");
    assert!(text.contains("note G4 1 1"), "{text}");

    let bad = Command::new(exe)
        .arg("accent")
        .arg(&source)
        .args(["--pattern", "1.5"])
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_extract_clips_a_beat_range() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-extract-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let source = root.join("source.coda");
    let out = root.join("out.coda");
    std::fs::write(&source, "note C4 1\nnote E4 1\nnote G4 1\n").unwrap();

    let result = Command::new(exe)
        .arg("extract")
        .arg(&source)
        .args(["--start", "0.5", "--end", "2.5", "-o"])
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.contains("note C4 0.5"), "{text}");
    assert!(text.contains("note G4 0.5"), "{text}");

    let bad = Command::new(exe)
        .arg("extract")
        .arg(&source)
        .args(["--start", "2", "--end", "1"])
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_invert_mirrors_pitches() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-invert-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let source = root.join("source.coda");
    let out = root.join("out.coda");
    std::fs::write(&source, "note C4 1\nnote E4 1\n").unwrap();

    let result = Command::new(exe)
        .arg("invert")
        .arg(&source)
        .args(["--center", "C4", "-o"])
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.contains("note C4"), "{text}");
    assert!(text.contains("note G#3"), "{text}");

    let bad = Command::new(exe)
        .arg("invert")
        .arg(&source)
        .args(["--center", "H9"])
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_arpeggiate_rolls_a_chord() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-arp-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let source = root.join("chord.coda");
    let out = root.join("rolled.coda");
    std::fs::write(&source, "note C4 1\nat 0 E4 1\nat 0 G4 1\n").unwrap();

    let result = Command::new(exe)
        .arg("arpeggiate")
        .arg(&source)
        .args(["--grid", "0.25", "-o"])
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.contains("at 0.25 E4"), "{text}");
    assert!(text.contains("at 0.5 G4"), "{text}");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_harmonize_adds_voices() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-harm-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let source = root.join("one.coda");
    let out = root.join("harm.coda");
    std::fs::write(&source, "note C4 1\n").unwrap();

    let result = Command::new(exe)
        .arg("harmonize")
        .arg(&source)
        .args(["--intervals", "4,7", "-o"])
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );

    let stats = Command::new(exe).arg("stats").arg(&out).output().unwrap();
    let text = String::from_utf8_lossy(&stats.stdout);
    assert!(text.contains("notes       3 sounding"), "{text}");
    assert!(text.contains("pitch range C4..G4"), "{text}");

    let bad = Command::new(exe)
        .arg("harmonize")
        .arg(&source)
        .args(["--intervals", "99"])
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_report_writes_a_deterministic_document() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-report-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let wav = root.join("audio.wav");
    let report = root.join("report.md");
    let rendered = Command::new(exe)
        .args(["render", "examples/scale.coda", "-o"])
        .arg(&wav)
        .output()
        .unwrap();
    assert!(rendered.status.success());

    let result = Command::new(exe)
        .args(["report", "examples/scale.coda", "--audio"])
        .arg(&wav)
        .arg("-o")
        .arg(&report)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = std::fs::read_to_string(&report).unwrap();
    assert!(text.starts_with("# Coda report\n"));
    assert!(text.contains("## Score\n"));
    assert!(text.contains("## Lint\n\nno warnings"));
    assert!(text.contains("## Fingerprint\n\n"));
    assert!(text.contains("## Audio\n\nframes      111132"));

    let again = root.join("again.md");
    let second = Command::new(exe)
        .args(["report", "examples/scale.coda", "--audio"])
        .arg(&wav)
        .arg("-o")
        .arg(&again)
        .output()
        .unwrap();
    assert!(second.status.success());
    assert_eq!(text, std::fs::read_to_string(&again).unwrap());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_midi_time_signature_writes_meta_event() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-timesig-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let plain = root.join("plain.mid");
    let waltz = root.join("waltz.mid");

    let base = Command::new(exe)
        .args(["render", "examples/scale.coda", "-o"])
        .arg(&plain)
        .output()
        .unwrap();
    assert!(base.status.success());
    let meter = Command::new(exe)
        .args([
            "render",
            "examples/scale.coda",
            "--time-signature",
            "3/4",
            "-o",
        ])
        .arg(&waltz)
        .output()
        .unwrap();
    assert!(
        meter.status.success(),
        "{}",
        String::from_utf8_lossy(&meter.stderr)
    );

    let plain_bytes = std::fs::read(&plain).unwrap();
    let waltz_bytes = std::fs::read(&waltz).unwrap();
    assert!(!plain_bytes
        .windows(7)
        .any(|w| w == [0xff, 0x58, 0x04, 0x03, 0x02, 24, 8]));
    assert!(waltz_bytes
        .windows(7)
        .any(|w| w == [0xff, 0x58, 0x04, 0x03, 0x02, 24, 8]));

    let bad = Command::new(exe)
        .args(["render", "examples/scale.coda", "--time-signature", "3/3"])
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_stamp_writes_and_reads_provenance() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-stamp-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let wav = root.join("audio.wav");
    let stamped = root.join("stamped.wav");
    let rendered = Command::new(exe)
        .args(["render", "examples/scale.coda", "-o"])
        .arg(&wav)
        .output()
        .unwrap();
    assert!(rendered.status.success());

    let result = Command::new(exe)
        .args(["stamp", "examples/scale.coda"])
        .arg(&wav)
        .arg("-o")
        .arg(&stamped)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );

    let info = Command::new(exe)
        .arg("stamp-info")
        .arg(&stamped)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&info.stdout);
    assert!(
        text.contains("digest:") && text.contains("notes:5"),
        "{text}"
    );

    let before = std::fs::read(&wav).unwrap();
    let after = std::fs::read(&stamped).unwrap();
    assert_eq!(&after[8..before.len()], &before[8..]);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_verify_matches_and_locates_a_byte_change() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-verify-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let wav_path = root.join("audio.wav");
    let source = "examples/scale.coda";

    let rendered = Command::new(exe)
        .args(["render", source, "-o"])
        .arg(&wav_path)
        .output()
        .unwrap();
    assert!(rendered.status.success());

    let ok = Command::new(exe)
        .args(["verify", source])
        .arg(&wav_path)
        .output()
        .unwrap();
    assert_eq!(ok.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&ok.stdout).starts_with("OK "));

    let mut bytes = std::fs::read(&wav_path).unwrap();
    bytes[100] ^= 0xff;
    std::fs::write(&wav_path, &bytes).unwrap();
    let bad = Command::new(exe)
        .args(["verify", source])
        .arg(&wav_path)
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&bad.stdout).starts_with("MISMATCH at byte 100"));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_seed_root_transposes_the_generated_piece() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-root-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let mut sources = Vec::new();
    for (name, extra) in [
        ("c.coda", vec!["--root", "C4"]),
        ("d.coda", vec!["--root", "D4"]),
    ] {
        let path = root.join(name);
        let mut args = vec!["seed", "42", "--bars", "2"];
        args.extend(extra);
        args.push("-o");
        let out = Command::new(exe).args(args).arg(&path).output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        sources.push(std::fs::read_to_string(&path).unwrap());
    }
    assert_ne!(sources[0], sources[1], "D4 should shift the key");

    let no_root = root.join("none.coda");
    let out = Command::new(exe)
        .args(["seed", "42", "--bars", "2", "-o"])
        .arg(&no_root)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(sources[0], std::fs::read_to_string(&no_root).unwrap());

    let bad = Command::new(exe)
        .args(["seed", "42", "--root", "H9"])
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_bitdepth_writes_8bit_pcm() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-bitdepth-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let output = root.join("output.wav");
    std::fs::write(&input, wav_bytes(&[(0.0, 0.0); 100], 8_000)).unwrap();

    let result = Command::new(exe)
        .arg("bitdepth")
        .arg(&input)
        .args(["--bits", "8", "-o"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let out = std::fs::read(&output).unwrap();
    assert_eq!(u16::from_le_bytes([out[34], out[35]]), 8);
    assert_eq!(out.len(), 44 + 100 * 2);
    assert_eq!(out[44], 128);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_channels_transforms_stereo_frames() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-channels-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let swapped = root.join("swap.wav");
    let mono = root.join("mono.wav");
    std::fs::write(&input, wav_bytes(&[(0.8, 0.2); 100], 8_000)).unwrap();

    for (mode, path) in [("swap", &swapped), ("mono", &mono)] {
        let out = Command::new(exe)
            .arg("channels")
            .arg(&input)
            .args(["--mode", mode, "-o"])
            .arg(path)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let original = std::fs::read(&input).unwrap();
    assert_eq!(original.len(), std::fs::read(&swapped).unwrap().len());
    assert_ne!(original, std::fs::read(&swapped).unwrap());

    let bad = Command::new(exe)
        .arg("channels")
        .arg(&input)
        .args(["--mode", "nope"])
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_resample_halves_rate_and_frames() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-resample-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let output = root.join("output.wav");
    std::fs::write(&input, wav_bytes(&[(0.25, 0.25); 1_000], 44_100)).unwrap();

    let result = Command::new(exe)
        .arg("resample")
        .arg(&input)
        .args(["--rate", "22050", "-o"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );

    let analysis = Command::new(exe)
        .arg("analyze")
        .arg(&output)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&analysis.stdout);
    assert!(text.contains("sample rate 22050"), "{text}");
    assert!(text.contains("frames      500"), "{text}");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_concat_merges_scores_into_source() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-concat-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let a = root.join("a.coda");
    let b = root.join("b.coda");
    let out = root.join("merged.coda");
    std::fs::write(&a, "tempo 120\nnote C4 1\n").unwrap();
    std::fs::write(&b, "tempo 200\nnote E4 0.5\n").unwrap();

    let result = Command::new(exe)
        .arg("concat")
        .arg(&a)
        .arg(&b)
        .arg("-o")
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );

    let stats = Command::new(exe).arg("stats").arg(&out).output().unwrap();
    let text = String::from_utf8_lossy(&stats.stdout);
    assert!(text.contains("notes       2 sounding"), "{text}");
    assert!(text.contains("1.5 beats / 0.750s"), "{text}");

    let bad = Command::new(exe).arg("concat").arg(&a).output().unwrap();
    assert_eq!(bad.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_loop_repeats_the_phrase() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-loop-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let once = root.join("once.coda");
    let twice = root.join("twice.coda");

    for (path, value) in [(&once, "1"), (&twice, "2")] {
        let out = Command::new(exe)
            .args(["loop", "examples/scale.coda", "--times", value, "-o"])
            .arg(path)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let stats = Command::new(exe).arg("stats").arg(&twice).output().unwrap();
    let text = String::from_utf8_lossy(&stats.stdout);
    assert!(text.contains("notes       10 sounding"), "{text}");
    assert!(text.contains("8 beats / 4.800s"), "{text}");

    let baseline = root.join("baseline.coda");
    let out = Command::new(exe)
        .args(["export-score", "examples/scale.coda", "-o"])
        .arg(&baseline)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        std::fs::read(&once).unwrap(),
        std::fs::read(&baseline).unwrap()
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_fade_presets_change_edges_without_resizing() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-fade-pre-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let output = root.join("output.wav");
    std::fs::write(&input, wav_bytes(&[(0.5, 0.5); 1_000], 8_000)).unwrap();

    let result = Command::new(exe)
        .arg("fade")
        .arg(&input)
        .args(["--preset", "short", "-o"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let before = std::fs::read(&input).unwrap();
    let after = std::fs::read(&output).unwrap();
    assert_eq!(before.len(), after.len());
    assert_ne!(before, after);
    let first = i16::from_le_bytes([after[44], after[45]]).abs();
    assert!(first < 5, "head should be quiet: {first}");

    let bad = Command::new(exe)
        .arg("fade")
        .arg(&input)
        .args(["--preset", "nope"])
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_trim_removes_silence_and_shortens_audio() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-trim-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let output = root.join("output.wav");
    std::fs::write(
        &input,
        wav_bytes(
            &[(0.0, 0.0); 100]
                .into_iter()
                .chain([(0.5, 0.5); 100])
                .chain([(0.0, 0.0); 100])
                .collect::<Vec<_>>(),
            8_000,
        ),
    )
    .unwrap();

    let result = Command::new(exe)
        .arg("trim")
        .arg(&input)
        .args(["--trim-silence", "-o"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(std::fs::read(&output).unwrap().len() < std::fs::read(&input).unwrap().len());

    let analysis = Command::new(exe)
        .arg("analyze")
        .arg(&output)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&analysis.stdout);
    assert!(text.contains("frames      100"), "{text}");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_rms_normalization_hits_the_target() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-rms-norm-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let output = root.join("output.wav");
    std::fs::write(&input, wav_bytes(&[(0.3, 0.3), (-0.2, -0.1)], 8_000)).unwrap();

    let result = Command::new(exe)
        .arg("normalize")
        .arg(&input)
        .args(["--rms-target", "-18", "-o"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("RMS -18 dBFS"));

    let analysis = Command::new(exe)
        .arg("analyze")
        .arg(&output)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&analysis.stdout);
    let rms_dbfs: f64 = text
        .lines()
        .find(|line| line.starts_with("rms "))
        .and_then(|line| line.split('(').nth(1))
        .and_then(|part| part.split_whitespace().next())
        .and_then(|value| value.parse().ok())
        .expect("rms dBFS in analyze report");
    assert!((rms_dbfs + 18.0).abs() < 0.1, "{text}");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_swing_delays_offbeats_with_exact_identity() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-swing-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let straight = root.join("straight.coda");
    let swung = root.join("swung.coda");

    let out = Command::new(exe)
        .args(["swing", "examples/scale.coda", "--ratio", "0", "-o"])
        .arg(&straight)
        .output()
        .unwrap();
    assert!(out.status.success());
    let out = Command::new(exe)
        .args(["swing", "examples/scale.coda", "--ratio", "0.8", "-o"])
        .arg(&swung)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let baseline = root.join("baseline.coda");
    let out = Command::new(exe)
        .args(["export-score", "examples/scale.coda", "-o"])
        .arg(&baseline)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        std::fs::read(&straight).unwrap(),
        std::fs::read(&baseline).unwrap()
    );
    assert_ne!(
        std::fs::read(&swung).unwrap(),
        std::fs::read(&baseline).unwrap()
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_diatonic_moves_by_degrees_with_exact_identity() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-diatonic-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let zero = root.join("zero.coda");
    let moved = root.join("moved.coda");
    let minor = root.join("minor.coda");

    for (path, extra) in [
        (&zero, vec!["--steps", "0"]),
        (&moved, vec!["--steps", "2"]),
    ] {
        let mut args = vec!["diatonic", "examples/scale.coda"];
        args.extend(extra);
        args.push("-o");
        let out = Command::new(exe).args(args).arg(path).output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let out = Command::new(exe)
        .args([
            "diatonic",
            "examples/scale.coda",
            "--steps",
            "2",
            "--scale",
            "minor",
            "-o",
        ])
        .arg(&minor)
        .output()
        .unwrap();
    assert!(out.status.success());

    let baseline = root.join("baseline.coda");
    let out = Command::new(exe)
        .args(["export-score", "examples/scale.coda", "-o"])
        .arg(&baseline)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        std::fs::read(&zero).unwrap(),
        std::fs::read(&baseline).unwrap()
    );
    assert_ne!(
        std::fs::read(&moved).unwrap(),
        std::fs::read(&minor).unwrap()
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_humanize_is_seeded_with_exact_identity() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-humanize-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let mut outputs = Vec::new();
    for (name, seed, amount) in [
        ("a.coda", "42", "0.7"),
        ("b.coda", "42", "0.7"),
        ("c.coda", "43", "0.7"),
        ("zero.coda", "42", "0"),
    ] {
        let path = root.join(name);
        let out = Command::new(exe)
            .args([
                "humanize",
                "examples/scale.coda",
                "--seed",
                seed,
                "--amount",
                amount,
                "-o",
            ])
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        outputs.push(std::fs::read(&path).unwrap());
    }
    assert_eq!(outputs[0], outputs[1]);
    assert_ne!(outputs[0], outputs[2]);

    let baseline = root.join("baseline.coda");
    let out = Command::new(exe)
        .args(["export-score", "examples/scale.coda", "-o"])
        .arg(&baseline)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(outputs[3], std::fs::read(&baseline).unwrap());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_join_crossfade_shortens_the_result() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-xfade-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let a = root.join("a.coda");
    let b = root.join("b.coda");
    std::fs::write(&a, "note C4 1\n").unwrap();
    std::fs::write(&b, "note E4 1\n").unwrap();

    let hard = root.join("hard.wav");
    let soft = root.join("soft.wav");
    for (path, extra) in [(&hard, Vec::new()), (&soft, vec!["--crossfade", "0.05"])] {
        let mut args = vec!["join"];
        args.push(a.to_str().unwrap());
        args.push(b.to_str().unwrap());
        args.extend(extra);
        args.push("-o");
        let out = Command::new(exe).args(args).arg(path).output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let hard_bytes = std::fs::read(&hard).unwrap();
    let soft_bytes = std::fs::read(&soft).unwrap();
    assert!(soft_bytes.len() < hard_bytes.len());
    assert_eq!(&soft_bytes[0..4], b"RIFF");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_fade_to_uses_explicit_durations() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-fadeto-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let output = root.join("output.wav");
    std::fs::write(&input, wav_bytes(&[(0.5, 0.5); 100], 8_000)).unwrap();

    let result = Command::new(exe)
        .arg("fade-to")
        .arg(&input)
        .args(["--fade-in", "0.001", "-o"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let after = std::fs::read(&output).unwrap();
    assert_eq!(std::fs::read(&input).unwrap().len(), after.len());
    let first = i16::from_le_bytes([after[44], after[45]]);
    assert!(first < 1000, "head should be quiet: {first}");

    let no_args = Command::new(exe)
        .arg("fade-to")
        .arg(&input)
        .output()
        .unwrap();
    assert_eq!(no_args.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_pad_adds_exact_silence() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-pad-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let output = root.join("output.wav");
    std::fs::write(&input, wav_bytes(&[(0.5, 0.5); 100], 8_000)).unwrap();

    let result = Command::new(exe)
        .arg("pad")
        .arg(&input)
        .args(["--before", "0.01", "--after", "0.005", "-o"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );

    let analysis = Command::new(exe)
        .arg("analyze")
        .arg(&output)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&analysis.stdout);
    assert!(text.contains("frames      220"), "{text}");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_dc_removes_a_constant_offset() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-dc-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let output = root.join("output.wav");
    std::fs::write(&input, wav_bytes(&[(0.25, 0.25); 100], 8_000)).unwrap();

    let result = Command::new(exe)
        .arg("dc")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let after = std::fs::read(&output).unwrap();
    assert_eq!(std::fs::read(&input).unwrap().len(), after.len());
    let first = i16::from_le_bytes([after[44], after[45]]);
    assert!(
        first.abs() <= 1,
        "first sample should be near zero: {first}"
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_snap_scale_makes_a_score_diatonic() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-snap-{}-{}", std::process::id(), 7));
    let _ = std::fs::create_dir_all(&root);
    let source = root.join("source.coda");
    let out = root.join("out.coda");
    std::fs::write(&source, "note C4 1\nnote C#4 1\nnote D#4 1\nnote F#4 1\n").unwrap();

    let result = Command::new(exe)
        .arg("snap-scale")
        .arg(&source)
        .args(["--root", "C4", "--scale", "major", "-o"])
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.contains("note C4"), "{text}");
    assert!(text.contains("note D4"), "{text}");
    assert!(text.contains("note F4"), "{text}");
    assert!(!text.contains("C#4"), "{text}");
    assert!(!text.contains("D#4"), "{text}");

    let stats = Command::new(exe).arg("stats").arg(&out).output().unwrap();
    let summary = String::from_utf8_lossy(&stats.stdout);
    assert!(summary.contains("pitch range C4..F4"), "{summary}");

    let bad = Command::new(exe)
        .arg("snap-scale")
        .arg(&source)
        .args(["--root", "H9"])
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_pedal_adds_a_spanning_drone() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-pedal-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let source = root.join("source.coda");
    let out = root.join("out.coda");
    std::fs::write(&source, "note C4 1\nat 2 E4 0.5\n").unwrap();

    let result = Command::new(exe)
        .arg("pedal")
        .arg(&source)
        .args(["--note", "C2", "--velocity", "0.4", "-o"])
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.contains("at 0 C2 2.5 0.4"), "{text}");

    let stats = Command::new(exe).arg("stats").arg(&out).output().unwrap();
    let summary = String::from_utf8_lossy(&stats.stdout);
    assert!(summary.contains("notes       3 sounding"), "{summary}");
    assert!(summary.contains("pitch range C2..E4"), "{summary}");

    let bad = Command::new(exe)
        .arg("pedal")
        .arg(&source)
        .args(["--note", "H9"])
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_health_reports_exact_diagnostics() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-health-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    std::fs::write(
        &input,
        wav_bytes(&[(0.5, 0.5), (-0.5, -0.5), (0.5, 0.5), (0.0, 0.0)], 8_000),
    )
    .unwrap();

    let result = Command::new(exe)
        .arg("health")
        .arg(&input)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = String::from_utf8_lossy(&result.stdout);
    assert!(text.contains("frames          4"), "{text}");
    assert!(text.contains("silent frames   1"), "{text}");
    assert!(text.contains("clipped samples 0"), "{text}");
    assert!(text.contains("zero crossings  2/2"), "{text}");
    assert!(text.contains("crest factor    1.25 dB"), "{text}");

    let missing = Command::new(exe).arg("health").output().unwrap();
    assert_eq!(missing.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_overlay_superimposes_scores_at_absolute_beats() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-overlay-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let a = root.join("a.coda");
    let b = root.join("b.coda");
    let out = root.join("out.coda");
    std::fs::write(&a, "note C4 1\nat 2 E4 0.5\n").unwrap();
    std::fs::write(&b, "note G3 2\nat 1 B3 0.5\n").unwrap();

    let result = Command::new(exe)
        .arg("overlay")
        .arg(&a)
        .arg(&b)
        .args(["-o"])
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.contains("note C4 1 1"), "{text}");
    assert!(text.contains("at 0 G3 2 1"), "{text}");
    assert!(text.contains("note B3 0.5 1"), "{text}");
    assert!(text.contains("at 2 E4 0.5 1"), "{text}");

    let stats = Command::new(exe).arg("stats").arg(&out).output().unwrap();
    let summary = String::from_utf8_lossy(&stats.stdout);
    assert!(summary.contains("notes       4 sounding"), "{summary}");
    assert!(summary.contains("pitch range G3..E4"), "{summary}");

    let missing = Command::new(exe).arg("overlay").arg(&a).output().unwrap();
    assert_eq!(missing.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_polarity_flips_and_restores_samples() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-polarity-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let output = root.join("output.wav");
    let restored = root.join("restored.wav");
    std::fs::write(&input, wav_bytes(&[(0.5, -0.25), (-0.8, 0.3)], 8_000)).unwrap();

    let result = Command::new(exe)
        .arg("polarity")
        .arg(&input)
        .args(["-o"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let before = std::fs::read(&input).unwrap();
    let after = std::fs::read(&output).unwrap();
    assert_eq!(before.len(), after.len());
    let original = i16::from_le_bytes([before[44], before[45]]);
    let flipped = i16::from_le_bytes([after[44], after[45]]);
    assert_eq!(flipped, -original, "first sample sign is flipped");

    let result = Command::new(exe)
        .arg("polarity")
        .arg(&output)
        .args(["-o"])
        .arg(&restored)
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(std::fs::read(&restored).unwrap(), before);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_echo_to_adds_a_delayed_tail() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-echoto-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let output = root.join("output.wav");
    let mut frames = vec![(0.0_f64, 0.0_f64); 12];
    frames[0] = (1.0, 0.5);
    std::fs::write(&input, wav_bytes(&frames, 8_000)).unwrap();

    let result = Command::new(exe)
        .arg("echo-to")
        .arg(&input)
        .args(["--echo-delay", "0.001", "--echo-feedback", "0.5", "-o"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let after = std::fs::read(&output).unwrap();
    assert_eq!(after.len(), std::fs::read(&input).unwrap().len());
    let delayed = i16::from_le_bytes([after[44 + 8 * 4], after[45 + 8 * 4]]);
    assert!((delayed as f64 / 32767.0 - 0.5).abs() < 0.01, "{delayed}");
    assert_eq!(
        i16::from_le_bytes([after[48], after[49]]),
        0,
        "no early tail"
    );

    let missing = Command::new(exe)
        .arg("echo-to")
        .arg(&input)
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_chorus_blends_a_modulated_delay() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-chorus-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let output = root.join("output.wav");
    let unity = root.join("unity.wav");
    std::fs::write(
        &input,
        wav_bytes(
            &[(0.2, 0.2), (0.6, 0.6), (1.0, 1.0), (0.6, 0.6), (0.2, 0.2)],
            8_000,
        ),
    )
    .unwrap();

    let result = Command::new(exe)
        .arg("chorus")
        .arg(&input)
        .args([
            "--chorus-rate",
            "200",
            "--chorus-depth",
            "2",
            "--chorus-delay",
            "2",
            "--mix",
            "0.5",
            "-o",
        ])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let after = std::fs::read(&output).unwrap();
    let before = std::fs::read(&input).unwrap();
    assert_eq!(after.len(), before.len());
    assert_ne!(after, before, "a wet chorus changes samples");

    let zero = Command::new(exe)
        .arg("chorus")
        .arg(&input)
        .args(["--mix", "0", "-o"])
        .arg(&unity)
        .output()
        .unwrap();
    assert_eq!(zero.status.code(), Some(2), "a zero mix is a no-op");
    let missing = Command::new(exe)
        .arg("chorus")
        .arg(&input)
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_vibrato_warps_the_timeline() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-vib-{}-{}", std::process::id(), 23));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let output = root.join("output.wav");
    let unity = root.join("unity.wav");
    std::fs::write(
        &input,
        wav_bytes(&[(0.0, 0.0), (0.4, 0.4), (1.0, 1.0), (0.4, 0.4)], 8_000),
    )
    .unwrap();

    let result = Command::new(exe)
        .arg("vibrato")
        .arg(&input)
        .args(["--vib-rate", "1000", "--vib-depth", "1.5", "-o"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let after = std::fs::read(&output).unwrap();
    let before = std::fs::read(&input).unwrap();
    assert_eq!(after.len(), before.len());
    assert_ne!(after, before, "positive depth changes samples");

    let result = Command::new(exe)
        .arg("vibrato")
        .arg(&input)
        .args(["--vib-rate", "5", "--vib-depth", "0", "-o"])
        .arg(&unity)
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(std::fs::read(&unity).unwrap(), before);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_mswidth_adjusts_the_side_component() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-msw-{}-{}", std::process::id(), 19));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let mono = root.join("mono.wav");
    let unity = root.join("unity.wav");
    std::fs::write(&input, wav_bytes(&[(0.6, 0.2), (-0.3, 0.8)], 8_000)).unwrap();

    let result = Command::new(exe)
        .arg("mswidth")
        .arg(&input)
        .args(["--side", "0", "-o"])
        .arg(&mono)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let after = std::fs::read(&mono).unwrap();
    assert_eq!(after[44..46], after[46..48], "first frame is dual mono");

    let result = Command::new(exe)
        .arg("mswidth")
        .arg(&input)
        .args(["--side", "1", "-o"])
        .arg(&unity)
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(
        std::fs::read(&unity).unwrap(),
        std::fs::read(&input).unwrap()
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_pan_sweep_moves_the_stereo_image() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-psweep-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let output = root.join("output.wav");
    std::fs::write(&input, wav_bytes(&[(0.5, 0.5); 64], 8_000)).unwrap();

    let result = Command::new(exe)
        .arg("pan-sweep")
        .arg(&input)
        .args(["--autopan-rate", "100", "--autopan-depth", "1", "-o"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let after = std::fs::read(&output).unwrap();
    assert_eq!(after.len(), std::fs::read(&input).unwrap().len());
    let left = |i: usize| i16::from_le_bytes([after[44 + i * 4], after[45 + i * 4]]);
    let right = |i: usize| i16::from_le_bytes([after[46 + i * 4], after[47 + i * 4]]);
    assert!(left(0) > 0 && right(0) > 0, "centre at start");
    assert!((0..64).any(|i| left(i) > right(i)), "sweeps left");
    assert!((0..64).any(|i| right(i) > left(i)), "sweeps right");

    let missing = Command::new(exe)
        .arg("pan-sweep")
        .arg(&input)
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_midi_render_matches_direct_render() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-mr-{}-{}", std::process::id(), 17));
    let _ = std::fs::create_dir_all(&root);
    let score = root.join("source.coda");
    let midi = root.join("song.mid");
    let direct = root.join("direct.wav");
    let via = root.join("via.wav");
    std::fs::write(&score, "tempo 120\nnote C4 1\nnote E4 1\nnote G4 2\n").unwrap();

    let r = Command::new(exe)
        .arg("render")
        .arg(&score)
        .args(["--sample-rate", "8000", "-o"])
        .arg(&direct)
        .output()
        .unwrap();
    assert!(r.status.success());

    let r = Command::new(exe)
        .arg("render")
        .arg(&score)
        .args(["-o"])
        .arg(&midi)
        .output()
        .unwrap();
    assert!(r.status.success());

    let r = Command::new(exe)
        .arg("midi-render")
        .arg(&midi)
        .args(["--sample-rate", "8000", "-o"])
        .arg(&via)
        .output()
        .unwrap();
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));

    assert_eq!(
        std::fs::read(&via).unwrap(),
        std::fs::read(&direct).unwrap()
    );

    let bad = Command::new(exe).arg("midi-render").output().unwrap();
    assert_eq!(bad.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_noise_is_seeded_and_bounded() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-noise-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let a = root.join("a.wav");
    let b = root.join("b.wav");
    let unity = root.join("unity.wav");
    std::fs::write(&input, wav_bytes(&[(0.0, 0.0); 32], 8_000)).unwrap();

    for (path, seed) in [(&a, "42"), (&b, "43")] {
        let result = Command::new(exe)
            .arg("noise")
            .arg(&input)
            .args(["--seed", seed, "--amount", "0.3", "-o"])
            .arg(path)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    assert_ne!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());

    let result = Command::new(exe)
        .arg("noise")
        .arg(&input)
        .args(["--seed", "42", "--amount", "0", "-o"])
        .arg(&unity)
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(
        std::fs::read(&unity).unwrap(),
        std::fs::read(&input).unwrap()
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_midside_roundtrips_a_stereo_wav() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-ms-{}-{}", std::process::id(), 13));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let encoded = root.join("encoded.wav");
    let decoded = root.join("decoded.wav");
    std::fs::write(&input, wav_bytes(&[(0.6, 0.2), (-0.3, 0.8)], 8_000)).unwrap();

    let result = Command::new(exe)
        .arg("midside")
        .arg(&input)
        .args(["--mode", "encode", "-o"])
        .arg(&encoded)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );

    let result = Command::new(exe)
        .arg("midside")
        .arg(&encoded)
        .args(["--mode", "decode", "-o"])
        .arg(&decoded)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );

    let original = std::fs::read(&input).unwrap();
    let restored = std::fs::read(&decoded).unwrap();
    assert_eq!(original.len(), restored.len());
    for i in (44..original.len()).step_by(2) {
        let before = i16::from_le_bytes([original[i], original[i + 1]]);
        let after = i16::from_le_bytes([restored[i], restored[i + 1]]);
        assert!((before - after).abs() <= 2, "{before} vs {after}");
    }
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_sequence_transposes_phrase_copies() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-seq-{}-{}", std::process::id(), 11));
    let _ = std::fs::create_dir_all(&root);
    let source = root.join("source.coda");
    let out = root.join("out.coda");
    std::fs::write(&source, "note C4 1\nnote E4 1\n").unwrap();

    let result = Command::new(exe)
        .arg("sequence")
        .arg(&source)
        .args(["--shifts", "0,12", "-o"])
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.contains("C5 1 1"), "{text}");
    assert!(text.contains("E5 1 1"), "{text}");

    let stats = Command::new(exe).arg("stats").arg(&out).output().unwrap();
    let summary = String::from_utf8_lossy(&stats.stdout);
    assert!(summary.contains("notes       6 sounding"), "{summary}");
    assert!(summary.contains("pitch range C4..E5"), "{summary}");

    let bad = Command::new(exe)
        .arg("sequence")
        .arg(&source)
        .args(["--shifts", "99"])
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_limit_caps_every_sample() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-limit-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let output = root.join("output.wav");
    let unity = root.join("unity.wav");
    std::fs::write(&input, wav_bytes(&[(0.2, -0.2), (0.9, -0.9)], 8_000)).unwrap();

    let result = Command::new(exe)
        .arg("limit")
        .arg(&input)
        .args(["--limit", "0.5", "-o"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let after = std::fs::read(&output).unwrap();
    assert_eq!(after.len(), std::fs::read(&input).unwrap().len());
    let first = i16::from_le_bytes([after[44], after[45]]);
    let second = i16::from_le_bytes([after[48], after[49]]);
    assert!((first as f64 / 32767.0 - 0.2).abs() < 0.01, "quiet kept");
    assert!((second as f64 / 32767.0 - 0.5).abs() < 0.01, "loud capped");

    let result = Command::new(exe)
        .arg("limit")
        .arg(&input)
        .args(["--limit", "1", "-o"])
        .arg(&unity)
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(
        std::fs::read(&unity).unwrap(),
        std::fs::read(&input).unwrap()
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_crossfade_blends_two_wavs() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-xfade-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let a = root.join("a.wav");
    let b = root.join("b.wav");
    let out = root.join("out.wav");
    std::fs::write(&a, wav_bytes(&[(0.0, 0.0); 4], 8_000)).unwrap();
    std::fs::write(&b, wav_bytes(&[(1.0, 1.0); 4], 8_000)).unwrap();

    let result = Command::new(exe)
        .arg("crossfade")
        .arg(&a)
        .arg(&b)
        .args(["--crossfade", "0.00025", "-o"])
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let after = std::fs::read(&out).unwrap();
    assert_eq!(after.len(), 44 + 6 * 4, "4 + 4 - 2 overlap");
    let middle = i16::from_le_bytes([after[44 + 3 * 4], after[45 + 3 * 4]]);
    assert!((middle as f64 / 32767.0 - 0.5).abs() < 0.01, "{middle}");

    let missing = Command::new(exe)
        .arg("crossfade")
        .arg(&a)
        .arg(&b)
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_repeat_wav_tiles_exactly() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-repwav-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let output = root.join("output.wav");
    let unity = root.join("unity.wav");
    std::fs::write(&input, wav_bytes(&[(0.2, -0.2), (0.5, 0.5)], 8_000)).unwrap();

    let result = Command::new(exe)
        .arg("repeat-wav")
        .arg(&input)
        .args(["--times", "3", "-o"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let after = std::fs::read(&output).unwrap();
    let before = std::fs::read(&input).unwrap();
    assert_eq!(after.len(), 44 + 3 * (before.len() - 44));
    assert_eq!(&after[44..before.len()], &before[44..]);
    assert_eq!(&after[before.len()..2 * before.len() - 44], &before[44..]);

    let result = Command::new(exe)
        .arg("repeat-wav")
        .arg(&input)
        .args(["--times", "1", "-o"])
        .arg(&unity)
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(std::fs::read(&unity).unwrap(), before);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_reverse_region_flips_only_the_window() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-revreg-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let output = root.join("output.wav");
    std::fs::write(
        &input,
        wav_bytes(&[(0.1, 0.1), (0.2, 0.2), (0.3, 0.3), (0.4, 0.4)], 8_000),
    )
    .unwrap();

    let result = Command::new(exe)
        .arg("reverse-region")
        .arg(&input)
        .args(["--start", "0.000125", "--end", "0.000375", "-o"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let after = std::fs::read(&output).unwrap();
    let before = std::fs::read(&input).unwrap();
    assert_eq!(after.len(), before.len());
    assert_eq!(after[44..48], before[44..48], "head unchanged");
    assert_eq!(after[56..], before[56..], "tail unchanged");
    assert_eq!(after[48..52], before[52..56], "middle reversed");

    let missing = Command::new(exe)
        .arg("reverse-region")
        .arg(&input)
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_intervals_reports_melodic_steps() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-intervals-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let source = root.join("source.coda");
    std::fs::write(&source, "note C4 1\nnote E4 1\nnote G4 1\nnote E4 1\n").unwrap();

    let result = Command::new(exe)
        .arg("intervals")
        .arg(&source)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = String::from_utf8_lossy(&result.stdout);
    assert!(text.contains("sequence    +4 +3 -3"), "{text}");
    assert!(
        text.contains("direction   up 2 / down 1 / level 0"),
        "{text}"
    );

    std::fs::write(&source, "tempo 120\n").unwrap();
    let empty = Command::new(exe)
        .arg("intervals")
        .arg(&source)
        .output()
        .unwrap();
    assert_eq!(empty.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_bandpass_filters_a_step_signal() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-bandpass-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let output = root.join("output.wav");
    std::fs::write(&input, wav_bytes(&[(0.5, 0.5); 32], 8_000)).unwrap();

    let result = Command::new(exe)
        .arg("bandpass")
        .arg(&input)
        .args(["--high-alpha", "1", "--low-alpha", "1", "-o"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let after = std::fs::read(&output).unwrap();
    assert_eq!(after.len(), std::fs::read(&input).unwrap().len());
    assert!(
        after[44..].iter().all(|b| *b == 0),
        "constant signal removed"
    );

    let missing = Command::new(exe)
        .arg("bandpass")
        .arg(&input)
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_distort_saturates_quiet_samples_more() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-distort-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let output = root.join("output.wav");
    let unity = root.join("unity.wav");
    std::fs::write(&input, wav_bytes(&[(0.2, -0.2), (1.0, -1.0)], 8_000)).unwrap();

    let result = Command::new(exe)
        .arg("distort")
        .arg(&input)
        .args(["--drive", "1", "-o"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let after = std::fs::read(&output).unwrap();
    assert_eq!(after.len(), std::fs::read(&input).unwrap().len());
    let first = i16::from_le_bytes([after[44], after[45]]);
    assert!(
        first > 0.2_f64.mul_add(32767.0, 0.0) as i16,
        "quiet sample boosted"
    );

    let result = Command::new(exe)
        .arg("distort")
        .arg(&input)
        .args(["--drive", "0", "-o"])
        .arg(&unity)
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(
        std::fs::read(&unity).unwrap(),
        std::fs::read(&input).unwrap()
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_chord_inserts_a_triad() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-chord-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let source = root.join("source.coda");
    let out = root.join("out.coda");
    std::fs::write(&source, "note C4 1\n").unwrap();

    let result = Command::new(exe)
        .arg("chord")
        .arg(&source)
        .args([
            "--at",
            "1",
            "--pitch",
            "C4",
            "--quality",
            "major",
            "--beats",
            "2",
            "-o",
        ])
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.contains("note C4 2 0.7"), "{text}");
    assert!(text.contains("at 1 E4 2 0.7"), "{text}");
    assert!(text.contains("at 1 G4 2 0.7"), "{text}");

    let stats = Command::new(exe).arg("stats").arg(&out).output().unwrap();
    assert!(String::from_utf8_lossy(&stats.stdout).contains("notes       4 sounding"));

    let bad = Command::new(exe)
        .arg("chord")
        .arg(&source)
        .args(["--at", "0", "--pitch", "C4", "--quality", "seventh"])
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_midi_info_describes_a_generated_file() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-mi-{}-{}", std::process::id(), 3));
    let _ = std::fs::create_dir_all(&root);
    let score = root.join("source.coda");
    let midi = root.join("out.mid");
    std::fs::write(&score, "tempo 120\nnote C4 1\nnote E4 1\nnote G4 2\n").unwrap();

    let rendered = Command::new(exe)
        .arg("render")
        .arg(&score)
        .args(["-o"])
        .arg(&midi)
        .output()
        .unwrap();
    assert!(rendered.status.success());

    let result = Command::new(exe)
        .arg("midi-info")
        .arg(&midi)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = String::from_utf8_lossy(&result.stdout);
    assert!(text.contains("format      0"), "{text}");
    assert!(text.contains("tracks      1"), "{text}");
    assert!(text.contains("notes       3"), "{text}");
    assert!(text.contains("pitch range C4..G4"), "{text}");

    let missing = Command::new(exe).arg("midi-info").output().unwrap();
    assert_eq!(missing.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_velocity_scales_to_a_peak() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-velocity-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let source = root.join("source.coda");
    let out = root.join("out.coda");
    std::fs::write(&source, "note C4 1 0.8\nnote E4 1 0.4\n").unwrap();

    let result = Command::new(exe)
        .arg("velocity")
        .arg(&source)
        .args(["--peak", "1", "-o"])
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.contains("note C4 1 1"), "{text}");
    assert!(text.contains("note E4 1 0.5"), "{text}");

    let missing = Command::new(exe)
        .arg("velocity")
        .arg(&source)
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_speed_changes_duration_at_fixed_rate() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-speed-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let output = root.join("output.wav");
    let unity = root.join("unity.wav");
    std::fs::write(
        &input,
        wav_bytes(&[(0.0, 0.0), (0.5, 0.5), (1.0, 1.0), (1.0, 1.0)], 8_000),
    )
    .unwrap();

    let result = Command::new(exe)
        .arg("speed")
        .arg(&input)
        .args(["--factor", "2", "-o"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let after = std::fs::read(&output).unwrap();
    assert_eq!(after.len(), 44 + 2 * 4, "4 frames -> 2");
    let analysis = Command::new(exe)
        .arg("analyze")
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&analysis.stdout).contains("sample rate 8000"),
        "rate must be unchanged"
    );

    let result = Command::new(exe)
        .arg("speed")
        .arg(&input)
        .args(["--factor", "1", "-o"])
        .arg(&unity)
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(
        std::fs::read(&unity).unwrap(),
        std::fs::read(&input).unwrap()
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_highpass_removes_a_constant_component() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-highpass-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let output = root.join("output.wav");
    std::fs::write(&input, wav_bytes(&[(0.5, 0.5); 32], 8_000)).unwrap();

    let result = Command::new(exe)
        .arg("highpass")
        .arg(&input)
        .args(["--alpha", "1", "-o"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let after = std::fs::read(&output).unwrap();
    assert_eq!(after.len(), std::fs::read(&input).unwrap().len());
    assert!(
        after[44..].iter().all(|b| *b == 0),
        "constant signal removed"
    );

    let missing = Command::new(exe)
        .arg("highpass")
        .arg(&input)
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_pan_places_the_stereo_image() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-pan-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let left = root.join("left.wav");
    let unity = root.join("unity.wav");
    std::fs::write(&input, wav_bytes(&[(0.5, -0.25); 8], 8_000)).unwrap();

    let result = Command::new(exe)
        .arg("pan")
        .arg(&input)
        .args(["--pan", "-1", "-o"])
        .arg(&left)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let after = std::fs::read(&left).unwrap();
    assert_eq!(
        i16::from_le_bytes([after[46], after[47]]),
        0,
        "right silenced"
    );

    let result = Command::new(exe)
        .arg("pan")
        .arg(&input)
        .args(["--pan", "0", "-o"])
        .arg(&unity)
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(
        std::fs::read(&unity).unwrap(),
        std::fs::read(&input).unwrap()
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_lowpass_smooths_a_step() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-lowpass-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let output = root.join("output.wav");
    std::fs::write(
        &input,
        wav_bytes(
            &[(0.0, 0.0), (1.0, 1.0), (1.0, 1.0), (1.0, 1.0), (1.0, 1.0)],
            8_000,
        ),
    )
    .unwrap();

    let result = Command::new(exe)
        .arg("lowpass")
        .arg(&input)
        .args(["--alpha", "0.5", "-o"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let after = std::fs::read(&output).unwrap();
    assert_eq!(after.len(), std::fs::read(&input).unwrap().len());
    let sample = |i: usize| i16::from_le_bytes([after[44 + i * 4], after[45 + i * 4]]);
    assert_eq!(sample(0), 0);
    assert!((sample(1) as f64 / 32767.0 - 0.5).abs() < 0.01);
    assert!((sample(2) as f64 / 32767.0 - 0.75).abs() < 0.01);

    let unity = root.join("unity.wav");
    let result = Command::new(exe)
        .arg("lowpass")
        .arg(&input)
        .args(["--alpha", "1", "-o"])
        .arg(&unity)
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(
        std::fs::read(&unity).unwrap(),
        std::fs::read(&input).unwrap()
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_splice_inserts_at_an_offset() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-splice-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let a = root.join("a.wav");
    let b = root.join("b.wav");
    let out = root.join("out.wav");
    std::fs::write(&a, wav_bytes(&[(0.1, 0.1), (0.2, 0.2), (0.3, 0.3)], 8_000)).unwrap();
    std::fs::write(&b, wav_bytes(&[(0.9, 0.9), (0.8, 0.8)], 8_000)).unwrap();

    let result = Command::new(exe)
        .arg("splice")
        .arg(&a)
        .arg(&b)
        .args(["--at", "0.000125", "-o"])
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let after = std::fs::read(&out).unwrap();
    assert_eq!(after.len(), 44 + 5 * 4, "3 + 2 frames");
    let frame = |i: usize| i16::from_le_bytes([after[44 + i * 4], after[45 + i * 4]]);
    assert!((frame(0) as f64 / 32767.0 - 0.1).abs() < 0.01);
    assert!((frame(1) as f64 / 32767.0 - 0.9).abs() < 0.01);
    assert!((frame(2) as f64 / 32767.0 - 0.8).abs() < 0.01);
    assert!((frame(3) as f64 / 32767.0 - 0.2).abs() < 0.01);
    assert!((frame(4) as f64 / 32767.0 - 0.3).abs() < 0.01);

    let missing = Command::new(exe)
        .arg("splice")
        .arg(&a)
        .arg(&b)
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_ring_multiplies_two_wavs() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-ring-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let a = root.join("a.wav");
    let b = root.join("b.wav");
    let unit = root.join("unit.wav");
    let out = root.join("out.wav");
    let identity = root.join("identity.wav");
    std::fs::write(&a, wav_bytes(&[(0.5, -0.5)], 8_000)).unwrap();
    std::fs::write(&b, wav_bytes(&[(0.8, 0.4), (0.3, 0.3)], 8_000)).unwrap();
    std::fs::write(&unit, wav_bytes(&[(1.0, 1.0)], 8_000)).unwrap();

    let result = Command::new(exe)
        .arg("ring")
        .arg(&a)
        .arg(&b)
        .args(["-o"])
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let after = std::fs::read(&out).unwrap();
    assert_eq!(after.len(), 44 + 8, "longer input sets the length");
    let first = i16::from_le_bytes([after[44], after[45]]);
    assert!((first as f64 / 32767.0 - 0.4).abs() < 0.01, "{first}");
    assert_eq!(i16::from_le_bytes([after[48], after[49]]), 0, "zero padded");

    let result = Command::new(exe)
        .arg("ring")
        .arg(&a)
        .arg(&unit)
        .args(["-o"])
        .arg(&identity)
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(
        std::fs::read(&identity).unwrap(),
        std::fs::read(&a).unwrap()
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_crush_reduces_resolution_and_holds_samples() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-crush-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let output = root.join("output.wav");
    std::fs::write(
        &input,
        wav_bytes(&[(0.5, -0.25), (0.1, 0.9), (-0.8, 0.3), (0.6, -0.6)], 8_000),
    )
    .unwrap();

    let result = Command::new(exe)
        .arg("crush")
        .arg(&input)
        .args(["--crush-bits", "8", "--hold", "2", "-o"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let after = std::fs::read(&output).unwrap();
    let before = std::fs::read(&input).unwrap();
    assert_eq!(before.len(), after.len());
    let first = i16::from_le_bytes([after[44], after[45]]);
    let second = i16::from_le_bytes([after[48], after[49]]);
    assert_eq!(first, second, "held pair must match");
    assert_eq!(first & 0x00FF, 0, "8-bit grid clears low byte");

    let unity = root.join("unity.wav");
    let result = Command::new(exe)
        .arg("crush")
        .arg(&input)
        .args(["--crush-bits", "16", "--hold", "1", "-o"])
        .arg(&unity)
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(std::fs::read(&unity).unwrap(), before);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_gate_silences_quiet_frames() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-gate-{}-{}", std::process::id(), 1));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let output = root.join("output.wav");
    std::fs::write(
        &input,
        wav_bytes(&[(0.8, -0.6), (0.001, 0.0005), (0.5, 0.4)], 8_000),
    )
    .unwrap();

    let result = Command::new(exe)
        .arg("gate")
        .arg(&input)
        .args(["--threshold", "-40", "-o"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let before = std::fs::read(&input).unwrap();
    let after = std::fs::read(&output).unwrap();
    assert_eq!(before.len(), after.len());
    assert_eq!(
        i16::from_le_bytes([after[44], after[45]]),
        i16::from_le_bytes([before[44], before[45]]),
        "loud first frame passes"
    );
    assert_eq!(
        i16::from_le_bytes([after[48], after[49]]),
        0,
        "quiet frame gated"
    );
    assert_eq!(
        i16::from_le_bytes([after[52], after[53]]),
        i16::from_le_bytes([before[52], before[53]]),
        "loud third frame passes"
    );

    let unity = root.join("unity.wav");
    let result = Command::new(exe)
        .arg("gate")
        .arg(&input)
        .args(["--threshold", "-100", "-o"])
        .arg(&unity)
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(std::fs::read(&unity).unwrap(), before);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_tremolo_modulates_amplitude() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-tremolo-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let output = root.join("output.wav");
    std::fs::write(&input, wav_bytes(&[(0.8, -0.6); 200], 8_000)).unwrap();

    let result = Command::new(exe)
        .arg("tremolo")
        .arg(&input)
        .args(["--lfo-rate", "40", "--lfo-depth", "1", "-o"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let original = std::fs::read(&input).unwrap();
    let after = std::fs::read(&output).unwrap();
    assert_eq!(original.len(), after.len());
    let source = i16::from_le_bytes([original[44], original[45]]);
    let changed = i16::from_le_bytes([after[44], after[45]]);
    assert_eq!(source, changed, "starts at unity gain");
    let mut dipped = false;
    for chunk in after[44..].as_chunks::<4>().0 {
        let value = i16::from_le_bytes([chunk[0], chunk[1]]);
        if value.abs() < source.abs() - 2000 {
            dipped = true;
            break;
        }
    }
    assert!(dipped, "amplitude should dip below the source");

    let unity = root.join("unity.wav");
    let result = Command::new(exe)
        .arg("tremolo")
        .arg(&input)
        .args(["--lfo-depth", "0", "-o"])
        .arg(&unity)
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(std::fs::read(&unity).unwrap(), original);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_gain_trims_level_in_db() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-gain-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let quieter = root.join("quiet.wav");
    std::fs::write(&input, wav_bytes(&[(0.8, -0.4); 100], 8_000)).unwrap();

    let result = Command::new(exe)
        .arg("gain")
        .arg(&input)
        .args(["--db", "-6.0206", "-o"])
        .arg(&quieter)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let original = std::fs::read(&input).unwrap();
    let after = std::fs::read(&quieter).unwrap();
    assert_eq!(original.len(), after.len());
    let before_peak = i16::from_le_bytes([original[44], original[45]]).abs();
    let after_peak = i16::from_le_bytes([after[44], after[45]]).abs();
    assert!((after_peak as f64 / before_peak as f64 - 0.5).abs() < 0.01);

    let unity = root.join("unity.wav");
    let result = Command::new(exe)
        .arg("gain")
        .arg(&input)
        .args(["--db", "0", "-o"])
        .arg(&unity)
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(std::fs::read(&unity).unwrap(), original);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_width_controls_stereo_image() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-width-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let input = root.join("input.wav");
    let mono = root.join("mono.wav");
    std::fs::write(&input, wav_bytes(&[(0.8, 0.2); 100], 8_000)).unwrap();

    let result = Command::new(exe)
        .arg("width")
        .arg(&input)
        .args(["--width", "0", "-o"])
        .arg(&mono)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = std::fs::read(&mono).unwrap();
    assert_eq!(&bytes[44..46], &bytes[46..48]);

    let unity = root.join("unity.wav");
    let result = Command::new(exe)
        .arg("width")
        .arg(&input)
        .args(["--width", "1", "-o"])
        .arg(&unity)
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(
        std::fs::read(&unity).unwrap(),
        std::fs::read(&input).unwrap()
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_mix_layers_wavs_with_gain_control() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-mix-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let a = root.join("a.wav");
    let b = root.join("b.wav");
    let mixed = root.join("mixed.wav");
    let identity = root.join("identity.wav");

    for (score, path) in [("note C4 1\n", &a), ("note E4 0.5\n", &b)] {
        let source = root.join(format!(
            "{}.coda",
            path.file_stem().unwrap().to_string_lossy()
        ));
        std::fs::write(&source, score).unwrap();
        let out = Command::new(exe)
            .arg("render")
            .arg(&source)
            .arg("-o")
            .arg(path)
            .output()
            .unwrap();
        assert!(out.status.success());
    }

    let result = Command::new(exe)
        .arg("mix")
        .arg(&a)
        .arg(&b)
        .args(["--gain-b", "0.5", "-o"])
        .arg(&mixed)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let result_bytes = std::fs::read(&mixed).unwrap();
    let first = std::fs::read(&a).unwrap();
    assert_eq!(
        result_bytes.len(),
        first.len().max(std::fs::read(&b).unwrap().len())
    );

    let zero = Command::new(exe)
        .arg("mix")
        .arg(&a)
        .arg(&b)
        .args(["--gain-b", "0", "-o"])
        .arg(&identity)
        .output()
        .unwrap();
    assert!(zero.status.success());
    assert_eq!(std::fs::read(&identity).unwrap(), first);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_reverse_wav_is_byte_invertible() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-rev-wav-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let original = root.join("original.wav");
    let once = root.join("once.wav");
    let twice = root.join("twice.wav");

    let rendered = Command::new(exe)
        .args(["render", "examples/scale.coda", "-o"])
        .arg(&original)
        .output()
        .unwrap();
    assert!(rendered.status.success());
    for (from, to) in [(&original, &once), (&once, &twice)] {
        let out = Command::new(exe)
            .arg("reverse-wav")
            .arg(from)
            .arg("-o")
            .arg(to)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let before = std::fs::read(&original).unwrap();
    assert_ne!(before, std::fs::read(&once).unwrap());
    assert_eq!(before, std::fs::read(&twice).unwrap());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_quantize_snaps_starts_and_is_exact_on_grid() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-quantize-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let off = root.join("off.coda");
    let snapped = root.join("snapped.coda");
    std::fs::write(&off, "at 0.11 C4 0.5\nat 0.62 E4 0.5\n").unwrap();

    let result = Command::new(exe)
        .arg("quantize")
        .arg(&off)
        .args(["--grid", "0.25", "-o"])
        .arg(&snapped)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let source = std::fs::read_to_string(&snapped).unwrap();
    // Serialization uses sequential `note` lines because the snapped starts
    // land exactly on the running cursor.
    assert!(source.contains("note C4 0.5 1"), "{source}");
    assert!(source.contains("note E4 0.5 1"), "{source}");

    let before = Command::new(exe).arg("digest").arg(&off).output().unwrap();
    let after = Command::new(exe)
        .arg("digest")
        .arg(&snapped)
        .output()
        .unwrap();
    assert_ne!(before.stdout, after.stdout);

    let again = root.join("again.coda");
    let second = Command::new(exe)
        .arg("quantize")
        .arg(&snapped)
        .args(["--grid", "0.25", "-o"])
        .arg(&again)
        .output()
        .unwrap();
    assert!(second.status.success());
    assert_eq!(
        std::fs::read(&snapped).unwrap(),
        std::fs::read(&again).unwrap()
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_autopan_changes_balance_with_zero_depth_identity() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-autopan-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let plain = root.join("plain.wav");
    let panned = root.join("panned.wav");

    let base = Command::new(exe)
        .args(["render", "examples/scale.coda", "-o"])
        .arg(&plain)
        .output()
        .unwrap();
    assert!(base.status.success());
    let effect = Command::new(exe)
        .args([
            "render",
            "examples/scale.coda",
            "--autopan-rate",
            "0.3",
            "--autopan-depth",
            "0.8",
            "-o",
        ])
        .arg(&panned)
        .output()
        .unwrap();
    assert!(
        effect.status.success(),
        "{}",
        String::from_utf8_lossy(&effect.stderr)
    );

    let before = std::fs::read(&plain).unwrap();
    let after = std::fs::read(&panned).unwrap();
    assert_eq!(before.len(), after.len());
    assert_ne!(before, after);

    let identity = Command::new(exe)
        .args([
            "digest",
            "examples/scale.coda",
            "--autopan-rate",
            "0.3",
            "--autopan-depth",
            "0",
        ])
        .output()
        .unwrap();
    let plain_digest = Command::new(exe)
        .args(["digest", "examples/scale.coda"])
        .output()
        .unwrap();
    assert_eq!(identity.stdout, plain_digest.stdout);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_reverse_changes_audio_and_double_reverse_restores_it() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-reverse-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let once = root.join("once.coda");
    let twice = root.join("twice.coda");

    let original = Command::new(exe)
        .args(["digest", "examples/scale.coda"])
        .output()
        .unwrap();
    let first = Command::new(exe)
        .arg("reverse")
        .arg("examples/scale.coda")
        .arg("-o")
        .arg(&once)
        .output()
        .unwrap();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let reversed = Command::new(exe).arg("digest").arg(&once).output().unwrap();
    assert_ne!(original.stdout, reversed.stdout);

    let second = Command::new(exe)
        .arg("reverse")
        .arg(&once)
        .arg("-o")
        .arg(&twice)
        .output()
        .unwrap();
    assert!(second.status.success());
    let restored = Command::new(exe)
        .arg("digest")
        .arg(&twice)
        .output()
        .unwrap();
    assert_eq!(original.stdout, restored.stdout);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_echo_changes_audio_but_identity_is_exact() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-echo-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let plain = root.join("plain.wav");
    let echoed = root.join("echo.wav");

    let base = Command::new(exe)
        .args(["render", "examples/scale.coda", "-o"])
        .arg(&plain)
        .output()
        .unwrap();
    assert!(base.status.success());
    let effect = Command::new(exe)
        .args([
            "render",
            "examples/scale.coda",
            "--echo-delay",
            "0.08",
            "--echo-feedback",
            "0.4",
            "-o",
        ])
        .arg(&echoed)
        .output()
        .unwrap();
    assert!(
        effect.status.success(),
        "{}",
        String::from_utf8_lossy(&effect.stderr)
    );

    let before = std::fs::read(&plain).unwrap();
    let after = std::fs::read(&echoed).unwrap();
    assert_eq!(before.len(), after.len());
    assert_ne!(before, after);

    let no_delay = Command::new(exe)
        .args(["digest", "examples/scale.coda", "--echo-delay", "0"])
        .output()
        .unwrap();
    let plain_digest = Command::new(exe)
        .args(["digest", "examples/scale.coda"])
        .output()
        .unwrap();
    assert_eq!(no_delay.stdout, plain_digest.stdout);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_fades_change_edges_but_preserve_length() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-fades-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let plain = root.join("plain.wav");
    let faded = root.join("faded.wav");

    let base = Command::new(exe)
        .args(["render", "examples/scale.coda", "-o"])
        .arg(&plain)
        .output()
        .unwrap();
    assert!(base.status.success());
    let with_fades = Command::new(exe)
        .args([
            "render",
            "examples/scale.coda",
            "--fade-in",
            "0.02",
            "--fade-out",
            "0.05",
            "-o",
        ])
        .arg(&faded)
        .output()
        .unwrap();
    assert!(
        with_fades.status.success(),
        "{}",
        String::from_utf8_lossy(&with_fades.stderr)
    );

    let before = std::fs::read(&plain).unwrap();
    let after = std::fs::read(&faded).unwrap();
    assert_eq!(before.len(), after.len());
    assert_ne!(before, after);
    // Exact edge samples are already silent because of ADSR, so compare
    // frames inside the fade regions instead.
    let sample = |wav: &[u8], frame: usize| {
        let offset = 44 + frame * 4;
        i16::from_le_bytes([wav[offset], wav[offset + 1]]).abs()
    };
    let frames = (after.len() - 44) / 4;
    assert!(
        sample(&after, 441) <= sample(&before, 441),
        "fade-in should soften early frames"
    );
    assert!(
        sample(&after, frames - 441) <= sample(&before, frames - 441),
        "fade-out should soften late frames"
    );

    let zero = Command::new(exe)
        .args([
            "digest",
            "examples/scale.coda",
            "--fade-in",
            "0",
            "--fade-out",
            "0",
        ])
        .output()
        .unwrap();
    let plain_digest = Command::new(exe)
        .args(["digest", "examples/scale.coda"])
        .output()
        .unwrap();
    assert_eq!(zero.stdout, plain_digest.stdout);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_templates_list_and_materialize() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let list = Command::new(exe).arg("templates").output().unwrap();
    assert!(list.status.success());
    assert_eq!(
        String::from_utf8_lossy(&list.stdout),
        "waltz\nmarch\narp\nlullaby\ndrone\n"
    );

    let root = std::env::temp_dir().join(format!("coda-templates-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let path = root.join("waltz.coda");
    let out = Command::new(exe)
        .args(["template", "waltz", "-o"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let check = Command::new(exe).arg("stats").arg(&path).output().unwrap();
    let stats = String::from_utf8_lossy(&check.stdout);
    assert!(stats.contains("notes       8 sounding, 0 rests"), "{stats}");

    let bad = Command::new(exe)
        .arg("template")
        .arg("nope")
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(2));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_stats_has_exact_deterministic_output() {
    let exe = env!("CARGO_BIN_EXE_coda");
    let root = std::env::temp_dir().join(format!("coda-stats-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let path = root.join("stats.coda");
    std::fs::write(
        &path,
        "tempo 120\nnote C4 0.5 1 -0.5\nat 1 A4 0.5 1 0.5\nrest 1.5\n",
    )
    .unwrap();

    let out = Command::new(exe).arg("stats").arg(&path).output().unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
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
