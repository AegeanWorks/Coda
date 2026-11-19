# Decisions

Why this project is built the way it is, oldest first. One entry per decision that changes a
contract, a dependency, a data format or the architecture; IDs are consecutive and never reused.

<!-- Entry format:
### ADR-NNN: <decision in one line>
- **Date**: YYYY-MM-DD
- **Status**: Accepted
- **Context**: <what forced the decision>
- **Decision**: <what was decided>
- **Consequences**: <what follows, including what it costs>
-->

### ADR-001: Deterministic, dependency-free Rust sound compiler
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: The portfolio has no Rust audio renderer; the user asked for something unique and local-only.
- **Decision**: Build Coda as a standard-library-only Rust crate that renders a textual score to byte-identical stereo WAV, with FNV-1a fingerprints for regression pinning.
- **Consequences**: No crates or network at build time; every behaviour is testable byte-for-byte; DSP code must stay transparent.

### ADR-002: Stereo output with equal-power panning
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: A flat mono blend cannot express the arrangement styles the generator produces.
- **Decision**: Render (left, right) frames using cos((pan+1)pi/4) and sin((pan+1)pi/4), with a per-note pan and running score default.
- **Consequences**: WAV data is interleaved 16-bit stereo; golden vectors were regenerated; near-zero channels use a 1e-12 tolerance.

### ADR-003: Three deterministic arrangement styles
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Random scale notes alone are not music and give the generator no structure.
- **Decision**: Implement flow (rhythmic patterns and contour walk), arp (eighths plus bass) and chords (triads plus bass) as pure functions of the seed.
- **Consequences**: Structural tests replace hard-coded note counts; CLI flag --style selects the arrangement.

### ADR-004: Coda stays local-only: no publish, no remote
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: The user explicitly said the project must not appear in their GitHub profile.
- **Decision**: Keep the AIKit-created repository with no remote; never run publish or push for this project.
- **Consequences**: All delivery stops at local commit and aikit check; no GitHub repo, no dossier entry, no upstream.

### ADR-005: Album output carries a deterministic manifest
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Album digests were printed to stdout only, so generated tracks had no inspectable record of their seed and settings.
- **Decision**: Write manifest.json into the album directory with seed, track count, bars, scale, style, wave, tempo, sample rate, album digest and per-track digests.
- **Consequences**: Generated albums are self-describing and byte-identical across runs; no WAV bytes changed; album/ stays gitignored.

### ADR-006: Symbolic output: type-0 MIDI export by file extension
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: A sound compiler that only writes audio cannot hand the score to a DAW or synthesizer.
- **Decision**: Encode the same Score as a single-track Standard MIDI File (480 ticks/quarter, tempo meta, note on/off, velocity, end of track), selected by a .mid/.midi output extension.
- **Consequences**: No new dependency; WAV bytes and golden vectors unchanged; VLQ and header byte tests pin the format; a chosen extension is the only routing rule.

### ADR-007: Vibrato is a per-note value with a running override
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Score-wide vibrato cannot ornament a single note inside an otherwise straight phrase.
- **Decision**: Give Note a concrete Vibrato field; the score-wide directive updates both the score and the running note value, while vib rate cents overrides only following notes and vib default restores the score setting.
- **Consequences**: The renderer reads note.vibrato with no branching; unornamented scores render byte-identically; rate and cents are bounded at parse time.

### ADR-008: Oscillator choice is a per-note value
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: A single score-wide wave cannot give a bass and a lead different timbres in one render.
- **Decision**: Give Note a concrete Wave field; the score-wide wave directive updates both the score and the running note value, inst <name> overrides following notes, and inst default restores the score wave. Generated notes inherit the selected --wave.
- **Consequences**: The renderer reads note.wave directly with no branching; unoverridden scores and generated pieces render byte-identically; wave names are validated at parse time.

### ADR-009: Linting is a read-only view, not a validation gate
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Legal scores can still be musically poor or risky; an error would prevent experimentation and change existing output.
- **Decision**: Add a pure lint module that returns ordered warnings and a lint command that only prints them, exiting 0 even with warnings; parse errors still exit 2.
- **Consequences**: Exact-output tests pin the warning format; no WAV, MIDI or score bytes change; CLI feedback is deterministic and reviewable.

### ADR-010: Named ADSR presets share the running envelope slot
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Four-number ADSR overrides are precise but unreadable for common articulations.
- **Decision**: Expose five named envelopes through Envelope::preset and a preset directive that writes the same running note envelope as env; preset default restores the score envelope.
- **Consequences**: Presets are plain Envelope values with no new data type; a preset is byte-equivalent to its explicit numbers; unoverridden scores keep their golden vectors.

### ADR-011: Canonical score source round-trips through the existing parser
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Generated arrangements were render-only; saving one as an editable score required a lossless inverse of the parser.
- **Decision**: Add score::serialize that writes score directives and explicit per-note overrides, tracking the parser's cursor so at/note/rest placement is preserved; route .coda output and export-score to it.
- **Consequences**: parse(serialize(score)) equals score for all styles and overrides; no new file format or parser; WAV and MIDI output unchanged.

### ADR-012: Transposition is a single post-composition transform
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Changing key required editing every pitch by hand, and a per-render option must work for files and generated pieces alike.
- **Decision**: Apply a pure transpose(score, semitones) once after composition via --semitones; only pitch strings change, and rests are skipped.
- **Consequences**: An inverse shift restores byte-identical audio; no WAV/MIDI format changes; the option applies uniformly to render, seed, digest, info, lint and source export.

### ADR-013: CLI overrides must reach per-note fields
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Moving oscillator choice onto Note silently broke --wave for file-based commands; a CLI check caught it.
- **Decision**: Propagate a --wave override to every note of a parsed score as well as the score default; seed already did this through its generator.
- **Consequences**: A byte-level regression test pins the behavior; golden vectors for unmodified scores are unchanged.

### ADR-014: Variation is a pure seeded transform with an exact identity
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: A score is a single point; a reproducible family of related pieces needs a bounded transform with no hidden state.
- **Decision**: Implement vary(score, seed, amount) with a seeded LCG, bounded velocity/pan drift, occasional pitch ornament and waveform swaps; amount 0 returns the input unchanged, and the CLI writes canonical .coda source.
- **Consequences**: Identical seeds give identical source bytes; all varied fields stay in valid ranges; the result is an ordinary editable score that re-parses exactly.

### ADR-015: Compare reports structure and audio bytes in a stable order
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Variation and transposition produce many similar scores; a reviewable, testable diff is needed.
- **Decision**: Add a pure compare module that emits ordered structural differences and renders both scores to compare WAV bytes, exposed as a read-only compare command that exits 0 even when scores differ.
- **Consequences**: Exact-output tests pin the report; no score or output file is modified; equivalent scores are reported as audio-identical.

### ADR-016: MIDI import reconstructs the supported subset only
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: MIDI has no waveform, pan or envelope fields, so a full Score cannot be recovered from a .mid file.
- **Decision**: Parse format-0 single-track MIDI into a Score with recovered tempo, pitches, durations and velocities, using score defaults for waveform, gain, pan and envelope; running status and meta events are supported.
- **Consequences**: A score built from defaults round-trips to byte-identical audio; richer scores lose non-MIDI fields on import, which compare reports explicitly.

### ADR-017: MIDI import accepts format 1 and merges tracks
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: DAW exports are usually format 1 with a conductor track and separate note tracks; format 0 only was too narrow.
- **Decision**: Parse every MTrk chunk, take the first tempo meta event found, merge all note events, and sort the combined notes by (start tick, pitch) into one Score.
- **Consequences**: Format-0 imports stay byte-identical; track-count mismatches are rejected; imported notes are ordered by time then pitch for determinism.

### ADR-018: Joining validates WAV headers and rebuilds the RIFF length
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Sections rendered separately must combine into one playable file without external audio tools.
- **Decision**: Add a pure join module that checks 16-bit stereo WAV headers and sample-rate parity, concatenates data payloads, and writes a fresh RIFF header with the summed length; expose it as the join command.
- **Consequences**: Sections at different sample rates are rejected; single-part joins are byte-identical; golden vectors for individual renders are unchanged.

### ADR-019: Statistics are a read-only, exactly formatted view
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Generated, varied and imported scores need a quick comparison that does not require reading source or audio.
- **Decision**: Add a pure stats reporter with a fixed line order and safe empty-score behavior, exposed as a read-only stats command.
- **Consequences**: Exact-output tests pin the format; no score or output file changes; the command reuses the same score loader as info and lint.
