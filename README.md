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
- **Overlay** — `overlay` superimposes scores at their original beats for editable polyphonic layering.
- **Resampling** — `resample` converts a Coda WAV to another sample rate with deterministic linear interpolation.
- **Channel tools** — `channels --mode mono|swap|left|right` folds, swaps or isolates stereo sides.
- **Bit depth** — `bitdepth --bits 8|16` exports unsigned 8-bit PCM or keeps the original 16-bit bytes.
- **Root key** — `seed --root D4` generates an arrangement transposed from C into the chosen key.
- **Verification** — `verify` re-renders a score and compares bytes to a WAV, locating the first difference.
- **Provenance** — `stamp` appends a CODA chunk with the source digest and note count; `stamp-info` reads it back.
- **MIDI meter** — `--time-signature N/D` writes a standard time-signature meta event into exported MIDI.
- **Reports** — `report` writes one deterministic Markdown document with score stats, lint findings, fingerprint and optional audio analysis.
- **Width** — `width --width 0..2` folds to mono at 0, preserves audio at 1, and widens above 1.
- **Gain** — `gain --db N` trims level in decibels with 0 dB byte-identical.
- **DC removal** — `dc` subtracts the per-channel mean, leaving zero-mean audio untouched.
- **Padding** — `pad` prepends and appends exact durations of silence.
- **Exact fades** — `fade-to` applies `--fade-in`/`--fade-out` seconds to any Coda WAV.
- **Harmonization** — `harmonize --intervals 4,7` adds lower-velocity harmony voices to a melody.
- **Arpeggiation** — `arpeggiate` rolls simultaneous notes into a sequence on a grid.
- **Inversion** — `invert` mirrors pitches around a center note, keeping rhythm and timbre.
- **Extraction** — `extract` clips a score to a beat range, trimming overlapping notes.
- **Accents** — `accent --pattern 1.0,0.5` cycles velocities across sounding notes.
- **Tremolo** — `tremolo --lfo-rate HZ --lfo-depth 0..1` applies a cosine LFO to amplitude; depth 0 is an identity.
- **Register fitting** — `register --low C4 --high C5` octave-displaces notes into a target range.
- **Noise gate** — `gate --threshold DB` silences frames below a dBFS level, leaving louder frames untouched.
- **Bitcrush** — `crush --crush-bits N --hold N` reduces resolution and holds samples for a lo-fi effect.
- **Ring modulation** — `ring a.wav b.wav` multiplies two WAVs sample by sample.
- **WAV echo** — `echo-to --echo-delay S --echo-feedback 0..1` adds a bounded feedback delay to any WAV.
- **Polarity** — `polarity` flips every sample's sign; applying it twice restores the original bytes.
- **Health diagnostics** — `health` reports silence, clipping, DC offset, zero crossings and crest factor.
- **Pedal point** — `pedal --note C2` anchors a score with one sustained drone spanning its whole length.
- **Scale snapping** — `snap-scale --root C4 --scale major` moves pitches to the nearest scale degree.
- **Splicing** — `splice a.wav b.wav --at S` inserts audio at an exact time, shifting the original tail.
- **Low-pass** — `lowpass --alpha 0..1` smooths harsh content with a one-pole filter; alpha 1 is an identity.
- **Static pan** — `pan --pan -1..1` places a WAV at a fixed equal-power stereo position.
- **High-pass** — `highpass --alpha 0..1` removes slow drift and rumble; alpha 1 cancels a constant signal.
- **Speed** — `speed --factor F` time-compresses or expands audio while keeping the sample rate.
- **Velocity normalization** — `velocity --peak N` rescales dynamics so the loudest note reaches the target.
- **MIDI inspection** — `midi-info` reports a MIDI file's header and recovered musical content.
- **Chords** — `chord --at B --pitch NOTE --quality NAME` inserts a named-quality triad.
- **Distortion** — `distort --drive 0..1` adds bounded soft saturation; drive 0 is an identity.
- **Band-pass** — `bandpass --high-alpha N --low-alpha N` removes rumble and hiss in one pass.
- **Interval analysis** — `intervals` prints signed semitone steps and up/down counts for a melody.
- **Region reversal** — `reverse-region --start S --end S` flips a time window, keeping the head and tail.
- **WAV repetition** — `repeat-wav --times N` tiles a finished WAV into a loop with untouched bytes.
- **WAV crossfade** — `crossfade a.wav b.wav --crossfade S` blends two rendered files over an exact overlap.
- **Limiter** — `limit --limit 0..1` caps every sample at a chosen ceiling; 1 is an identity.
- **Sequencing** — `sequence --shifts 0,12,7` appends transposed copies of a phrase.
- **Mid/side** — `midside --mode encode|decode` converts stereo to mid/side components and back.
- **Noise overlay** — `noise --seed N --amount 0..1` adds reproducible bounded noise; amount 0 is an identity.
- **MIDI rendering** — `midi-render <file.mid>` turns a MIDI file directly into a deterministic WAV.
- **WAV auto-pan** — `pan-sweep --autopan-rate HZ --autopan-depth 0..1` sweeps a finished WAV's stereo image.
- **Mid/side width** — `mswidth --side 0..2` scales the side component; 0 is mono, 1 an identity.
- **Vibrato** — `vibrato --vib-rate HZ --vib-depth N` warps a WAV's pitch with a sine LFO.
- **Chorus** — `chorus --mix 0..1` blends a dry WAV with an LFO-modulated delayed copy.
- **Compression** — `compress --comp-threshold T --comp-ratio R` reduces loud passages above a threshold.
- **Sustain** — `sustain` extends each note to the next onset for a legato line.
- **Velocity ramp** — `ramp --from 0.2 --to 1` draws a crescendo or diminuendo across notes.
- **WAV concatenation** — `concat-wav a.wav b.wav [...]` appends several WAVs losslessly.
- **Overwrite** — `overwrite base.wav insert.wav --at S` replaces a region in place, keeping the base length.
- **Tuplets** — `tuplets --start B --end B --tuplet-ratio R` compresses or expands a beat window.
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
- **Augmentation** — `augment --factor F` scales only note durations in place, creating overlap without moving starts.

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
| `overlay <a.coda> <b.coda> [...]` | Superimpose scores at absolute beats. |
| `resample <in.wav> --rate HZ` | Convert to another sample rate. |
| `channels <in.wav> --mode NAME` | Fold, swap or isolate stereo channels. |
| `bitdepth <in.wav> --bits 8|16` | Export 8-bit or keep 16-bit PCM. |
| `verify <score.coda> <in.wav>` | Confirm a WAV matches its source. |
| `stamp` / `stamp-info` | Write or read a provenance chunk. |
| `report <score.coda> [--audio in.wav]` | Write a Markdown report. |
| `width <in.wav> --width 0..2` | Narrow or widen the stereo image. |
| `gain <in.wav> --db N` | Trim level in decibels. |
| `tremolo <in.wav> [--lfo-rate HZ] [--lfo-depth 0..1]` | Modulate amplitude with a cosine LFO. |
| `register <score.coda> --low NOTE --high NOTE` | Fit pitches into a target register. |
| `gate <in.wav> [--threshold DB]` | Silence frames below a dBFS threshold. |
| `crush <in.wav> [--crush-bits 1..16] [--hold N]` | Bitcrush and decimate audio. |
| `ring <a.wav> <b.wav>` | Multiply two WAVs (ring modulation). |
| `echo-to <in.wav> --echo-delay S [--echo-feedback 0..1]` | Add feedback delay to a WAV. |
| `augment <score.coda> --factor F` | Scale only note durations. |
| `polarity <in.wav>` | Invert sample polarity (phase flip). |
| `health <in.wav>` | Report silence, clipping, DC, crossings and crest. |
| `pedal <score.coda> --note NOTE [--velocity 0..1]` | Add a spanning drone. |
| `snap-scale <score.coda> --root NOTE [--scale NAME]` | Snap pitches to a scale. |
| `splice <a.wav> <b.wav> --at S` | Insert a WAV at a time offset. |
| `lowpass <in.wav> --alpha 0..1` | Apply a one-pole low-pass filter. |
| `pan <in.wav> --pan -1..1` | Set a fixed stereo position. |
| `highpass <in.wav> --alpha 0..1` | Apply a one-pole high-pass filter. |
| `speed <in.wav> --factor F` | Change duration at a fixed sample rate. |
| `velocity <score.coda> --peak 0..1` | Rescale note velocities to a peak. |
| `midi-info <file.mid>` | Inspect a MIDI file read-only. |
| `chord <score.coda> --at B --pitch NOTE --quality NAME` | Insert a triad. |
| `distort <in.wav> --drive 0..1` | Apply soft-clip saturation. |
| `bandpass <in.wav> --high-alpha N --low-alpha N` | Apply a cascaded band-pass filter. |
| `intervals <score.coda>` | Report melodic interval steps. |
| `reverse-region <in.wav> --start S --end S` | Reverse a time window. |
| `repeat-wav <in.wav> --times N` | Repeat a WAV into a loop. |
| `crossfade <a.wav> <b.wav> --crossfade S` | Crossfade two WAVs. |
| `limit <in.wav> --limit 0..1` | Cap samples at a ceiling. |
| `sequence <score.coda> --shifts LIST` | Append transposed phrase copies. |
| `midside <in.wav> --mode encode|decode` | Convert between stereo and mid/side. |
| `noise <in.wav> [--seed N] [--amount 0..1]` | Add seeded bounded noise. |
| `midi-render <file.mid> [--sample-rate N]` | Render a MIDI file to WAV. |
| `pan-sweep <in.wav> [--autopan-rate HZ] [--autopan-depth 0..1]` | Sweep a WAV's stereo image. |
| `mswidth <in.wav> --side 0..2` | Adjust mid/side width. |
| `vibrato <in.wav> --vib-rate HZ --vib-depth N` | Add pitch vibrato to a WAV. |
| `chorus <in.wav> [--chorus-rate HZ] [--chorus-depth N] [--chorus-delay N] --mix 0..1` | Add a chorus effect. |
| `compress <in.wav> [--comp-threshold 0..1] [--comp-ratio 1..20]` | Compress loud passages. |
| `sustain <score.coda>` | Extend notes to the next onset. |
| `ramp <score.coda> --from 0..1 --to 0..1` | Draw a velocity ramp. |
| `concat-wav <a.wav> <b.wav> [...]` | Append WAVs losslessly. |
| `overwrite <base.wav> <insert.wav> --at S` | Replace a region in place. |
| `tuplets <score.coda> --start B --end B --tuplet-ratio R` | Remap a beat window in time. |
| `dc <in.wav>` | Remove per-channel DC offset. |
| `pad <in.wav> [--before S] [--after S]` | Add leading/trailing silence. |
| `fade-to <in.wav> [--fade-in S] [--fade-out S]` | Apply exact fade durations. |
| `harmonize <score.coda> --intervals LIST` | Add harmony voices by semitone. |
| `arpeggiate <score.coda> [--grid B]` | Roll chords into a sequence. |
| `invert <score.coda> [--center NOTE]` | Mirror pitches around a center. |
| `extract <score.coda> --start B --end B` | Clip a score to a beat range. |
| `accent <score.coda> --pattern LIST` | Cycle a velocity pattern. |
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

Options: `-o/--out`, `--sample-rate`, `--bars`, `--tracks`, `--wave`, `--scale`, `--style`, `--tempo`, `--semitones`, `--seed`, `--amount`, `--peak`, `--fade-in`, `--fade-out`, `--echo-delay`, `--echo-feedback`, `--autopan-rate`, `--autopan-depth`, `--root`, `--time-signature`. Output routing by extension: `.wav` renders audio, `.mid`/`.midi` writes MIDI, `.coda` writes canonical source. Exit codes: `0` success, `2` usage/IO error.

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
