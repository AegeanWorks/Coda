# Coda

[![Built with AIKit](https://img.shields.io/badge/built%20with-AIKit-6f42c1)](https://github.com/AegeanWorks/AIKit)

Coda is a deterministic, dependency-free Rust sound compiler that turns a small textual score into reproducible stereo WAV audio, with seeded arrangement styles for locally generated music. It is a local-only project: no remote and no GitHub publication.

## ⚡ 30-Second Cold-Start Quick Reference

- **Runtime Prerequisites**: Rust 1.70+ with Cargo. No crates, no audio libraries, no network.
- **Canonical Regression Shield Gate**:
  ```bash
  cargo test
  ```
- **One-command verifier** (finds Cargo and a working linker):
  ```powershell
  powershell -ExecutionPolicy Bypass -File scripts/verify.ps1
  ```
- **Try it**:
  ```bash
  cargo run -- render examples/scale.coda -o scale.wav
  cargo run -- seed 2026 --bars 4 --style chords --wave saw -o generated.wav
  cargo run -- album 42 --tracks 3 -o album/
  cargo run -- render examples/scale.coda -o scale.mid
  ```

### 🛠️ AIKit Commands (run from the AIKit folder)

| Role | Command |
|---|---|
| Status, task and gates | `python aikit.py status coda` |
| Phase 4: every gate | `python aikit.py check coda` |
| Repair memory and map, then check | `python aikit.py fix coda` |
| Resume an interrupted task | `python aikit.py resume coda` |

## What it does

- **Score compiler** — tempo, waveform, gain, ADSR, vibrato, per-note velocity, per-note ADSR/vibrato/oscillator overrides, five named ADSR presets, equal-power stereo pan, `repeat` expansion, explicit beat placement (polyphony via stacked `at`).
- **Generative composition** — `seed` plus scale (major/minor/pentatonic/chromatic), tempo, bar count, waveform and arrangement style (`flow`, `arp`, `chords`). A seed always yields the same piece.
- **Album mode** — deterministic multi-track output plus a `manifest.json` recording the seed, settings, album fingerprint and per-track fingerprints.
- **MIDI export/import** — any score or seed can be written as a type-0 Standard MIDI File by choosing a `.mid`/`.midi` output extension; `import-midi` reads that subset back into an editable score with tempo, pitch, timing and velocity recovered.
- **Audio fingerprint** — `digest` prints a stable FNV-1a 64-bit ID for any rendered piece; golden vectors pin the renderer at two sample rates.
- **Score inspector** — `info` reports waveform, timing, note/rest counts and duration without writing audio.
- **Score linter** — `lint` reports deterministic warnings for extreme pitch, silent notes, clipping risk, duplicate notes and long values without changing the score.
- **Score round-trip** — `export-score` (or a `.coda` output extension) writes canonical source that parses back to the identical `Score`, including every per-note override.
- **Transposition** — `--semitones N` shifts every pitched note chromatically while leaving timing, velocity, pan and envelopes untouched; an inverse shift restores byte-identical audio.
- **Seeded variation** — `vary` applies bounded, reproducible changes to pitch, velocity, pan and timbre; `--amount 0` is an exact identity.
- **Comparison** — `compare` reports ordered structural differences between two scores and whether their rendered audio is byte-identical.
- **Joining** — `join` renders several scores at one sample rate and concatenates them, with an optional `--crossfade` overlap for seamless transitions.
- **Trimming** — `trim` cuts an exact time range or auto-removes leading/trailing silence with a threshold.
- **Fade presets** — `fade --preset` applies named envelopes to any Coda WAV using the deterministic fade code.
- **Looping** — `loop` repeats a score's phrase N times into editable canonical source.
- **Concatenation** — `concat` merges several score files into one arrangement with correct time offsets.
- **Statistics** — `stats` prints a deterministic summary: note/rest counts, duration, pitch range, distinct pitches, density, waveforms and pan span.
- **Normalization** — `normalize` scales a Coda WAV to a chosen peak or RMS target (dBFS) with deterministic clamping; it is idempotent and leaves silence untouched.
- **Templates** — five built-in sketches (`waltz`, `march`, `arp`, `lullaby`, `drone`) materialize as ordinary editable canonical scores.
- **Fades** — `--fade-in` and `--fade-out` apply deterministic linear envelopes to rendered audio, clamped to half the piece.
- **Echo** — `--echo-delay` and `--echo-feedback` add a bounded, reproducible feedback delay to any render.
- **Reversal** — `reverse` mirrors a score in time; applying it twice restores byte-identical audio.
- **Auto-pan** — `--autopan-rate` and `--autopan-depth` sweep the stereo image with an equal-power LFO; depth 0 is an identity.
- **Quantization** — `quantize` snaps note starts to a chosen rhythmic grid, leaving durations and timbres intact.
- **WAV reversal** — `reverse-wav` flips audio frames end-to-end; applying it twice restores the original bytes.
- **Mixing** — `mix` overlays two Coda WAVs with a gain on the second input, with safe clamping at the longer length.
- **Humanize** — `humanize` adds seeded, bounded timing and velocity jitter; amount 0 is an exact identity.
- **Diatonic transposition** — `diatonic` moves notes by scale degrees in the chosen key, unlike the chromatic `--semitones` option.
- **Swing** — `swing` delays off-beat eighths by a controlled ratio for a deterministic long-short feel.
- **Audio analysis** — `analyze` reports frames, duration, sample rate, peak and RMS (with dBFS) for any Coda WAV.
- **Time stretch** — `stretch` scales every note start and duration by a factor while preserving pitch and timbre.

Fingerprints use FNV-1a 64-bit: they detect accidental drift, not tampering. Rendered WAV is 16-bit interleaved stereo, 44.1 kHz by default.

## Command reference

| Command | Purpose |
|---|---|
| `render <score.coda>` | Compile a score to WAV. |
| `seed <n>` | Compose and render deterministically from a seed. |
| `album <n>` | Write a directory of deterministic tracks. |
| `info <score.coda>` | Inspect a score without rendering. |
| `stats <score.coda>` | Print deterministic score statistics. |
| `lint <score.coda>` | Print deterministic warnings; never changes the score. |
| `export-score <score.coda>` | Write canonical, re-parseable source. |
| `vary <score.coda> --seed N` | Write a seeded variation as canonical source. |
| `reverse <score.coda>` | Mirror a score in time. |
| `quantize <score.coda> [--grid B]` | Snap note starts to the nearest grid point. |
| `humanize <score.coda> --seed N [--amount 0..1]` | Add seeded timing and velocity jitter. |
| `diatonic <score.coda> --steps N [--scale name]` | Transpose by scale degrees. |
| `swing <score.coda> [--ratio 0..1]` | Apply a long-short eighth-note feel. |
| `analyze <in.wav>` | Report duration, peak and RMS levels. |
| `trim <in.wav> [--start S] [--end S] [--trim-silence]` | Cut a range or remove edge silence. |
| `fade <in.wav> --preset NAME` | Apply short/gentle/long/tail/attack. |
| `loop <score.coda> --times N` | Repeat a phrase into canonical source. |
| `concat <a.coda> <b.coda> [...]` | Merge scores into one arrangement. |
| `stretch <score.coda> --factor F` | Scale all note timing by a factor. |
| `reverse-wav <in.wav>` | Reverse audio frames end-to-end. |
| `mix <a.wav> <b.wav> [--gain-b 0..1]` | Overlay two WAVs. |
| `compare <a.coda> <b.coda>` | Report structural and audio differences. |
| `import-midi <file.mid>` | Reconstruct a score from a format-0 or format-1 MIDI file. |
| `join <a.coda> <b.coda> [...] [--crossfade S]` | Render sections into one WAV, optionally crossfaded. |
| `normalize <in.wav> [--peak P | --rms-target DB]` | Scale to a peak or RMS target. |
| `templates` / `template <name>` | List sketches or write one as canonical source. |
| `digest <score.coda>` | Print the audio fingerprint, byte count and frame count. |
| `help` | Show built-in help. |

Options: `-o/--out`, `--sample-rate`, `--bars`, `--tracks`, `--wave`, `--scale`, `--style`, `--tempo`, `--semitones`, `--seed`, `--amount`, `--peak`, `--fade-in`, `--fade-out`, `--echo-delay`, `--echo-feedback`, `--autopan-rate`, `--autopan-depth`. Output routing by extension: `.wav` renders audio, `.mid`/`.midi` writes MIDI, `.coda` writes canonical source. Exit codes: `0` success, `2` usage/IO error.

## Score language

One directive per line; `#` starts a comment. Durations are in beats, envelopes in seconds.

| Directive | Meaning |
|---|---|
| `tempo <bpm>` | Beats per minute (default 120). |
| `wave <name>` | `sine`, `square`, `triangle`, `saw`, `noise`, `pulse`, `organ`, `pluck`. |
| `gain <0..1>` | Master gain before soft clipping (default 0.3). |
| `attack/decay/sustain/release <s>` | Score-wide ADSR envelope. |
| `env <a> <d> <s> <r>` | Per-note ADSR override; `env default` restores the score envelope. |
| `preset <name>` | Named ADSR (`pluck`, `stab`, `pad`, `organ`, `bow`); `preset default` restores it. |
| `vibrato <rate_hz> <cents>` | Score-wide pitch oscillation; `0 0` disables it. |
| `vib <rate_hz> <cents>` | Per-note vibrato override; `vib default` restores the score setting. |
| `inst <wave>` | Per-note oscillator override; `inst default` restores the score wave. |
| `pan <-1..1>` | Running stereo position: -1 left, 0 centre, 1 right. |
| `repeat <n>` | Repeat the following `note`/`rest` events `n` times. |
| `note <pitch> <beats> [velocity] [pan]` | Note at the running cursor; advances it. |
| `at <beat> <pitch> <beats> [velocity] [pan]` | Note at an explicit beat; stack for chords. |
| `rest <beats>` | Silence that advances the cursor. |
| `at <beat> - <beats>` | Silence at an explicit beat, without moving the cursor. |

Pitches are `C4`, `A#4`, `Bb3`, etc. (A4 = 440 Hz, equal temperament). Examples: `examples/scale.coda`, `arp.coda`, `chords.coda`, `pulse.coda`, `stereo.coda`.

## 📦 What's Inside

```text
coda/
├── .gitattributes            <- LF line endings on every platform
├── .gitignore                <- transient AIKit state plus build/audio output
├── LICENSE                   <- Aegean License 1.1 (AL-1.1): free personal use
├── COMMERCIAL.md             <- Commercial licensing terms
├── LICENSES/                 <- REUSE license texts
├── REUSE.toml                <- Machine-readable SPDX metadata
├── README.md
├── Cargo.toml                <- Zero dependencies
├── scripts/                  <- One-command cross-toolchain test runner
│   └── verify.ps1
├── inventory.py              <- AI-compat inventory: kind|name|locator lines (ADR-315)
├── demo.py                   <- AI-compat scripted drive: WORLD-DELTA lines (ADR-315)
├── examples/                 <- Five example scores
├── src/                      <- Twelve Rust modules (DSP, parser, WAV/MIDI encoders, linter, transposition, CLI)
├── tests/                    <- Pipeline, golden vectors and CLI tests
│   └── integration.rs
└── .aikit/                   <- this project's memory: record, ledger, decisions
```

## 📜 License

Free for personal use under the Aegean License 1.1 (AL-1.1); see [LICENSE](LICENSE). Any commercial use needs the Aegean Commercial License 1.0 (ACL-1.0); see [COMMERCIAL.md](COMMERCIAL.md).
