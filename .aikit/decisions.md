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
