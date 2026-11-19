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
