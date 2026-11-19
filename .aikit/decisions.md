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

### ADR-020: Normalization is a deterministic post-render WAV transform
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Quiet renders and joined sections need usable levels without another audio tool.
- **Decision**: Decode 16-bit stereo Coda WAV frames, scale by target peak divided by the measured peak, clamp, and re-encode with the original sample rate; silence is returned unchanged.
- **Consequences**: The transform is idempotent and byte-deterministic; invalid WAVs and out-of-range peaks fail clearly; individual render golden vectors are unchanged.

### ADR-021: Templates are source strings, not a second score format
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Built-in starting points must stay in sync with the score language instead of becoming a parallel representation.
- **Decision**: Store each template as ordinary score source and parse it with the existing parser; template materializes the parsed result through the canonical serializer.
- **Consequences**: Every template automatically gains new directives; round-trip and listing tests pin behavior; no production unwrap because the parser reports errors normally.

### ADR-022: Fades are linear, clamped and applied only to audio output
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Hard starts and stops make rendered sections and joins sound unfinished.
- **Decision**: Apply a linear envelope over --fade-in/--fade-out seconds to both channels before WAV encoding, clamping each fade to half the buffer; zero fades leave bytes unchanged.
- **Consequences**: Audio paths (render, seed, album, join, digest) share one helper, while MIDI and score export are unaffected; golden vectors remain valid when fades are absent.

### ADR-023: Echo is a bounded comb delay in the shared audio path
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Dry renders need a simple spatial effect without external processors.
- **Decision**: Add a feedback delay applied before fades in the shared render helper, with feedback clamped below 1 and no effect when delay is zero or longer than the buffer.
- **Consequences**: Zero-delay renders stay byte-identical; the impulse response is pinned by tests; MIDI and score export are unaffected.

### ADR-024: Reversal mirrors starts only and preserves event order
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: A time-reversal transform should be a pure, invertible operation on an existing phrase.
- **Decision**: Set each note's start to total duration minus its original end, leaving order and all other fields untouched; expose it as a CLI command writing canonical source.
- **Consequences**: Applying reverse twice is byte-identical including chords; a test arithmetic slip in the expected mirrored starts was caught and corrected.

### ADR-025: Auto-pan is an equal-power LFO in the shared audio path
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Stereo output was static unless each note set its own pan; a sweep adds motion without editing scores.
- **Decision**: Apply a sine LFO to pan with equal-power gains normalised at centre, so depth 0 is an identity; group all audio-effect settings into AudioOptions to keep the render helper small.
- **Consequences**: Zero-depth renders stay byte-identical; gains are bounded; CLI wiring uses one options struct across render, seed, album, join and digest.

### ADR-026: Quantization changes starts only and is idempotent
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Off-grid note starts from variation or live input need a deterministic snapping tool.
- **Decision**: Round each note start to the nearest multiple of a positive grid, leaving duration, pitch and all timbral fields unchanged; expose it as quantize writing canonical source.
- **Consequences**: Re-quantizing an already snapped score is byte-identical; invalid grids fail clearly; the CLI test verifies snap positions through the canonical serializer.

### ADR-027: WAV reversal is a byte-invertible audio transform
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Sound design often needs backward audio; the score-level reverse does not reverse the rendered waveform.
- **Decision**: Decode 16-bit stereo Coda WAV frames, reverse their order and re-encode with the original sample rate; expose it as reverse-wav.
- **Consequences**: Double reversal is byte-identical; invalid WAVs fail clearly; score reversal and WAV reversal remain distinct operations.

### ADR-028: Mixing overlays WAVs at the longer length with bounded gain
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Join handles sequential sections but layering generated tracks needs a parallel composition operation.
- **Decision**: Decode two Coda WAVs, sum their frames with a gain on the second input, clamp to range and emit the longer length; reject sample-rate mismatches.
- **Consequences**: Zero gain is an identity on the first input; invalid inputs fail clearly; mix and join remain distinct composition tools.

### ADR-029: Humanize jitters sounding notes and explicit rests serialize via at -
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Jittering a rest's start broke the round-trip because rest was always written at the running cursor; jittering only pitched notes was not enough once t events leave the cursor behind.
- **Decision**: Leave rests untouched by humanize, and let the serializer emit t <beat> - <beats> when a rest's start differs from its cursor.
- **Consequences**: Every Score still round-trips exactly; the score language gains an explicit-rest form without a new directive; humanized output remains editable canonical source.

### ADR-030: Diatonic transposition reuses scale degrees
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Chromatic transposition can leave a melody outside its intended key; degree-based movement preserves key membership.
- **Decision**: Map each pitch to the nearest degree of the chosen scale, shift by an integer number of degrees with octave wrap, and reuse the existing scale definitions and note-name helpers.
- **Consequences**: Zero steps is byte-identical, a full scale walks an octave, and non-scale notes resolve deterministically to the nearest lower degree.

### ADR-031: Swing delays only on-grid off-beats
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: A deterministic long-short feel sits between rigid quantization and random humanize.
- **Decision**: Shift notes whose start sits on an odd eighth-note grid point by ratio times a quarter beat, leaving downbeats and all non-timing fields untouched; ratio 0 is an identity.
- **Consequences**: Swing output remains exactly serializable, off-beat positions are pinned by byte tests, and it composes with quantize and humanize.

### ADR-032: Audio analysis is read-only with a fixed report format
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Mixing decisions need inspectable level and duration data without leaving Coda.
- **Decision**: Add a pure analyzer that computes frames, duration, sample rate, peak and RMS with dBFS from 16-bit stereo frames, and a read-only analyze command with an exact pinned report.
- **Consequences**: Silence reports finite -inf dBFS values; invalid WAVs fail clearly; no score, WAV or gate behavior changes.

### ADR-033: Stretch scales the event timeline only
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Tempo override changes beat duration but not the score's written rhythm; scaling starts and durations changes the piece itself.
- **Decision**: Multiply every note start and beat length by a positive factor, leaving pitch, velocity, pan, envelopes, vibrato and waveform unchanged; factor 1 is an identity.
- **Consequences**: Stretched scores re-serialize exactly, invalid factors fail clearly, and stretch composes with swing, humanize and quantization.

### ADR-034: Normalize supports RMS targets alongside peak
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Peak normalization does not control perceived loudness; an RMS target mode is needed for level matching.
- **Decision**: Add normalize_rms that scales frames to a dBFS target with shared decoding and clamping, selected by --rms-target while --peak keeps its exact previous behavior.
- **Consequences**: RMS mode is idempotent and leaves silence untouched; the CLI report names its mode; analyze verifies the achieved level byte-for-byte.

### ADR-035: Join can crossfade adjacent sections with equal power
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Hard cuts between rendered sections click and sound abrupt.
- **Decision**: Add an optional overlap in join that blends the tail and head of adjacent sections with equal-power gains, clamping the overlap to half of the shorter section; zero overlap keeps the old byte-identical path.
- **Consequences**: Crossfaded output is shorter by the summed overlaps, the blend is deterministic, and section-level WAV bytes remain unchanged.

### ADR-036: Trim selects frames by time or by detected silence
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Rendered audio often has empty edges or needs an excerpt, with no Coda tool to cut it.
- **Decision**: Add trim that selects a frame range from seconds or auto-detects content above a dBFS threshold, keeping a single silent frame for fully silent input and preserving the sample rate.
- **Consequences**: No-op ranges are byte-identical, invalid ranges fail clearly, and edge-frame assertions tolerate 16-bit quantization.

### ADR-037: Fade presets reuse the deterministic fade engine on existing WAVs
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Render-time fades are insufficient for already-rendered or joined audio that needs finishing.
- **Decision**: Expose short/gentle/long/tail/attack presets that decode a Coda WAV, apply the shared fade function and re-encode at the original sample rate.
- **Consequences**: Preset names and edge attenuation are pinned by byte tests, unknown presets fail clearly, and output length never changes.

### ADR-038: Loop repeats a phrase into editable source
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Repeating a phrase as rendered audio loses editability; a source-level loop keeps the arrangement inspectable.
- **Decision**: Repeat every note with offsets of the phrase length into a new Score, with one repetition as an identity and zero rejected.
- **Consequences**: Looped scores re-serialize exactly, compose with other transforms, and their note counts and duration are pinned by byte tests.

### ADR-039: Concat merges scores in source form
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Audio join and phrase loop cover assembling sound, but distinct score files need a source-level merge for further editing.
- **Decision**: Append each subsequent score's notes with an offset of the preceding span, keeping the first score's global settings, and write canonical source.
- **Consequences**: A single input is byte-identical, offsets and note counts are pinned by byte tests, and the merged score re-serializes exactly.

### ADR-040: Resampling uses linear interpolation with an exact same-rate path
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: A single render sample rate is not usable by all downstream tools; conversion must stay deterministic and dependency-free.
- **Decision**: Decode Coda WAV frames and interpolate linearly at target-rate positions, rebuilding the header; same-rate input is returned through re-encoding unchanged.
- **Consequences**: Frame counts round from the rate ratio, interpolation midpoints are pinned by byte tests, and invalid rates fail clearly.

### ADR-041: Channel operations are exact frame transforms
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Stereo WAVs could only be panned at render time; post-render channel folding or isolation was unavailable.
- **Decision**: Add mono/swap/left/right modes that transform decoded frames and re-encode at the original rate with unchanged length.
- **Consequences**: Each mode's frame math is pinned by byte tests, unknown modes fail clearly, and no existing vectors change.

### ADR-042: Eight-bit export uses unsigned PCM with exact sample mapping
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Retro and embedded targets need 8-bit audio; Coda only wrote 16-bit.
- **Decision**: Convert decoded frames to unsigned 8-bit PCM with a rebuilt header and exact -1/0/1 mapping, while --bits 16 returns the original bytes unchanged.
- **Consequences**: Header fields, sample bytes and length are pinned by byte tests; analyze remains 16-bit-only by design.

### ADR-043: Seed root key reuses chromatic transposition
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Generated arrangements were fixed to C; a root option lets the same seed be heard in another key without changing the generator.
- **Decision**: Parse --root as a note name and transpose the generated score by its semitone offset from C4, reusing the existing transpose function.
- **Consequences**: C4 is an identity, other roots shift uniformly, invalid names fail clearly, and no-root output stays byte-identical.

### ADR-044: Verify checks rendered bytes against the source score
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: After chains of effects and edits there was no direct check that a WAV still matches its score.
- **Decision**: Re-render a score with the current settings, compare against a WAV byte-for-byte, report the first difference offset, and use exit 0 for match, 1 for mismatch, 2 for usage errors.
- **Consequences**: Match and single-byte mismatch cases are pinned by byte tests; verify reuses the shared render path so effect options are honored.

### ADR-045: WAV provenance uses a small CODA chunk
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Rendered audio did not record which score produced it, breaking the studio's 'explains itself' guarantee.
- **Decision**: Append an even-padded CODA chunk with source digest, note count and tempo, updating the RIFF size while leaving audio bytes untouched; stamp-info reads it back.
- **Consequences**: Audio payloads are unchanged, missing stamps report cleanly, and the chunk round-trip is pinned by byte tests.

### ADR-046: MIDI export can write an explicit time signature
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: MIDI files carried tempo but no meter, so DAWs assumed 4/4.
- **Decision**: Add an optional N/D time signature that writes FF 58 04 with a power-of-two denominator exponent at tick 0; omitting it preserves previous MIDI bytes.
- **Consequences**: The meta event bytes and invalid-meter rejection are pinned by byte tests; existing golden MIDI vectors are unchanged.

### ADR-047: Report composes existing analyses into one deterministic document
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Score, lint, fingerprint and audio facts were scattered across commands, making a piece hard to summarize.
- **Decision**: Build a Markdown report from the existing pure analyzers with a fixed section order, including audio analysis only when a WAV is supplied.
- **Consequences**: The document is byte-stable, warnings and stats are reused verbatim, and invalid audio fails clearly.

### ADR-048: Stereo width uses mid/side scaling
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Pan and channel operations could not change the width of an existing stereo image.
- **Decision**: Scale the side component by an amount in 0..2 while keeping mid fixed, clamping to range; amount 1 is byte-identical and 0 folds to mono.
- **Consequences**: Width changes are deterministic and length-preserving; widening may clamp peaks, which is pinned by byte tests.

### ADR-049: Gain is an exact dB-to-linear WAV transform
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Peak and RMS normalization target a level; a simple relative dB trim was missing.
- **Decision**: Apply a 10^(dB/20) factor to decoded frames with clamping, limiting the range to -100..+6 dB; 0 dB returns byte-identical audio.
- **Consequences**: Halving at -6.0206 dB and unity identity are pinned by byte tests; invalid WAVs and out-of-range dB fail clearly.

### ADR-050: DC removal subtracts the measured per-channel mean
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Constant offsets waste headroom and are inaudible; no Coda tool removed them.
- **Decision**: Compute the mean of each channel across all frames and subtract it, clamping to range; zero-mean input returns byte-identical audio.
- **Consequences**: Known-offset removal and zero-mean identity are pinned by byte tests; invalid WAVs fail clearly.

### ADR-051: Padding inserts exact silent frames around audio
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Assembling sections needs precise silence gaps without external tools.
- **Decision**: Round before/after seconds to whole frames at the source rate and emit zero frames around unchanged audio; zero padding is byte-identical.
- **Consequences**: Frame counts, byte shifting and zero-padding identity are pinned by byte tests; invalid WAVs fail clearly.

### ADR-052: Fade-to applies exact durations to existing WAVs
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Presets cover common fades but cannot express an exact transition length.
- **Decision**: Expose --fade-in/--fade-out seconds on WAVs through the shared fade engine, requiring at least one non-zero duration.
- **Consequences**: Edge attenuation, unchanged length and zero-duration identity are pinned by byte tests; presets remain untouched.

### ADR-053: Harmonization adds interval voices at reduced velocity
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Melodies could be moved and varied but could not grow harmony voices from their existing notes.
- **Decision**: Append one voice per requested semitone interval after each pitched note at 70% velocity, skipping rests and out-of-range pitches.
- **Consequences**: Original notes are untouched, intervals are bounded, and the result re-serializes exactly.

### ADR-054: Arpeggiation rolls simultaneities onto a grid
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Chords written as simultaneous notes could not be turned into a linear figure.
- **Decision**: Group notes by their grid-aligned start and reassign sequential starts within each group while preserving all other fields.
- **Consequences**: Note count and timbre are unchanged, output re-serializes exactly, and invalid grids fail clearly.

### ADR-055: Melodic inversion mirrors pitches around a center
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Time reversal and transposition existed but pitch reflection around a center did not.
- **Decision**: Reflect each pitch as 2*center - pitch, skipping rests and out-of-range results, preserving all other note fields.
- **Consequences**: Reflection values, field preservation and round-trip are pinned by byte tests; invalid centers fail clearly.

### ADR-056: Extraction clips scores to a beat range
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Scores could be trimmed only by hand while WAVs had a trim command.
- **Decision**: Keep notes overlapping [start,end), clip their starts and durations to the window, preserve all other fields, and reject invalid ranges.
- **Consequences**: Clipped positions and durations are pinned by byte tests and the result re-serializes exactly.

### ADR-057: Accent applies a repeating velocity pattern
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Per-note velocities could be set but not cycled as a groove pattern.
- **Decision**: Apply a list of velocities cyclically to sounding notes only, preserving all other fields and rejecting invalid patterns.
- **Consequences**: Pattern cycling, rest skipping and round-trip are pinned by byte tests; invalid inputs fail clearly.

### ADR-058: Tremolo applies a cosine amplitude LFO to WAVs
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Auto-pan modulated the stereo image at render time but there was no way to apply a repeating amplitude swell to an existing WAV.
- **Decision**: Expose tremolo as a WAV transform with --lfo-rate and --lfo-depth, using gain = 1 - depth/2 + (depth/2)*cos(2*pi*rate*t) so it starts at unity, stays within [1-depth,1], and depth 0 is byte-identical.
- **Consequences**: Unity start, bounded modulation, determinism and zero-depth identity are pinned by byte tests; invalid rates or depths fail clearly.

### ADR-059: Register fitting moves notes by octave into a range
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Transpose shifts every note by a fixed interval and inversion mirrors pitches, but neither can fold stray notes into a target register.
- **Decision**: For each sounding pitch choose the octave congruent to it that lies inside [low,high] and is closest to the original, leaving the note unchanged when no such octave exists.
- **Consequences**: Octave folding, preservation of all other fields, exact round-trip and bound validation are pinned by byte tests.

### ADR-060: Noise gate silences quiet WAV frames by dBFS threshold
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Trim removes edge silence only; quiet frames inside a WAV had no tool to silence them.
- **Decision**: Gate each frame on its peak amplitude against a linear threshold derived from --threshold dBFS, zeroing frames below it and passing louder frames unchanged.
- **Consequences**: Selective gating, pass-through identity at -100 dBFS, all-frame gating at 0 dBFS and bound validation are pinned by byte tests.

### ADR-061: Crush reduces WAV resolution and holds samples
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Bitdepth changed the container format and resample interpolated smoothly, leaving no lo-fi effect inside a 16-bit WAV.
- **Decision**: Quantize each sample by masking its 16-bit integer to the requested bit depth and repeat it for a hold factor, with 16 bits and hold 1 byte-identical.
- **Consequences**: Full-resolution identity, exact quantization grid, sample-and-hold behaviour and argument validation are pinned by byte tests.

### ADR-062: Ring modulation multiplies two Coda WAVs
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: mix overlays WAVs by addition but could not multiply them for amplitude modulation or lo-fi timbres.
- **Decision**: Multiply matching-rate WAVs per channel and per frame, zero-pad the shorter input to the longer length, clamp products, and treat full scale as an exact identity.
- **Consequences**: Per-channel products, identity, zero padding, commutativity and rate validation are pinned by byte tests.

### ADR-063: Echo-to applies the shared feedback delay to existing WAVs
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Echo was only available at render time, so processed WAVs could not receive a feedback delay.
- **Decision**: Reuse the bounded comb echo on decoded Coda WAV frames with the existing --echo-delay and --echo-feedback options, requiring a positive delay and preserving frame count and rate.
- **Consequences**: Delayed tail, zero-delay identity, determinism, length preservation and missing-delay failure are pinned by byte tests.

### ADR-064: Augmentation scales note durations in place
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: stretch moved note starts as well as durations, leaving no way to lengthen notes in place for overlap or legato.
- **Decision**: Multiply only each note's beats by a positive factor, leaving starts, pitches, timbres and velocities untouched, with unit factor byte-identical.
- **Consequences**: Duration-only scaling, start preservation, exact round-trip and factor validation are pinned by byte tests.

### ADR-065: Polarity inversion flips every WAV sample as an exact involution
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Pitch inversion and WAV reversal existed but there was no phase-flip utility for comparing correlated signals.
- **Decision**: Negate each interleaved i16 sample in place with wrapping_neg and copy the header unchanged, so applying polarity twice restores every byte including the i16::MIN extreme.
- **Consequences**: Sign flips, byte-exact double inversion at the extreme, header preservation and invalid-WAV rejection are pinned by byte tests.

### ADR-066: Overlay superimposes scores at their absolute beats
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: concat joined scores sequentially and mix overlaid rendered audio, but neither left a superimposition of two editable scores.
- **Decision**: Union the note lists without shifting starts, sort the result by start then pitch for deterministic canonical output, and keep the first score's global settings.
- **Consequences**: Start preservation, stable ordering, round-trip and empty-input rejection are pinned by byte tests.

### ADR-067: Health reports WAV silence, clipping, DC and crest
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: analyze covered peak and RMS only, leaving clipping, silence, DC offset, zero crossings and crest factor unmeasured.
- **Decision**: Print a fixed-format health report with exact silent-frame and clipped-sample counts, per-channel zero crossings and DC offset, and a finite-or--inf crest factor.
- **Consequences**: Exact counts, stable formatting, silence handling and invalid-WAV rejection are pinned by byte tests.

### ADR-068: Pedal adds a spanning drone to a score
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Harmonize spawned a voice per note but there was no way to anchor a whole passage with one sustained bass note.
- **Decision**: Append a single note at beat zero whose duration is the score's maximum note end, using the requested pitch and velocity and the score's default timbre, preserving all existing notes.
- **Consequences**: Span, pitch, velocity, preservation and exact round-trip are pinned by byte tests; empty scores and invalid arguments fail.

### ADR-069: Snap-scale moves pitches to the nearest scale degree
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: quantize aligned note timing but no tool made an existing melody diatonic to a chosen key.
- **Decision**: For each sounding pitch choose the scale degree nearest its pitch class relative to the root, keeping the lower degree on ties and preserving all other note fields.
- **Consequences**: Nearest-degree snapping, tie-breaking, preservation, round-trip and root validation are pinned by byte tests.

### ADR-070: Splice inserts a WAV at an exact frame offset
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: pad inserted only silence and mix added signals, with no way to place one recording inside another and shift the tail.
- **Decision**: Round the offset to frames at the shared sample rate, keep the base head, zero-fill any gap past its end, append the insert, then reattach the shifted tail.
- **Consequences**: Frame-exact insertion, zero padding, rate validation and tail preservation are pinned by byte tests.

### ADR-071: Lowpass applies a deterministic one-pole filter
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: No effect could smooth harsh high-frequency content in a rendered WAV.
- **Decision**: Filter each channel with y = y + alpha*(x - y), seeded from the first sample, with alpha=1 byte-identical and alpha=0 holding the initial value.
- **Consequences**: Identity, freeze, step response, determinism and coefficient validation are pinned by byte tests.

### ADR-072: Static pan places a WAV with equal-power gains
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: autopan swept the image over time and width folded or widened it, but no tool set a fixed stereo position on a finished WAV.
- **Decision**: Apply fixed equal-power cos/sin gains derived from --pan, with centre byte-identical and hard sides silencing the opposite channel.
- **Consequences**: Centre identity, hard-side isolation, bounded partial gains and determinism are pinned by byte tests.

### ADR-073: Highpass subtracts a one-pole low-pass state
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Lowpass could remove top end but no tool removed slow drift and rumble from a rendered WAV.
- **Decision**: Subtract a one-pole low-pass state per channel; alpha 1 passes sample-to-sample changes (first difference), alpha 0 removes the first-sample offset, and a constant signal is silent at any coefficient.
- **Consequences**: First-difference behaviour, DC removal, constant-signal silence and determinism are pinned by byte tests.

### ADR-074: Speed changes duration without changing sample rate
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: resample altered both playback speed and the declared sample rate, with no way to time-compress while keeping the output rate.
- **Decision**: Resample the frame timeline by a factor with deterministic linear interpolation and write the original sample rate; factor 1 is byte-identical.
- **Consequences**: Unit identity, frame-count rounding, interpolation, rate preservation and factor validation are pinned by byte tests.

### ADR-075: Velocity normalization rescales dynamics to a peak
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: accent cycled fixed velocities and humanize jittered them, but no tool rescaled a score's dynamics while keeping their ratios.
- **Decision**: Multiply every sounding note velocity by peak divided by the current maximum, leaving rests and all other fields unchanged; a silent score is untouched.
- **Consequences**: Peak targeting, ratio preservation, rest handling, round-trip and validation are pinned by byte tests.

### ADR-076: MIDI inspection reports header and recovered content
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: MIDI files could be exported and imported but not inspected read-only without writing a score.
- **Decision**: Parse the MThd header for format, track count and division, then summarize the score recovered by the existing import path with tempo, note/rest counts, duration and numeric pitch range.
- **Consequences**: Exact header fields, recovered metrics and invalid-file rejection are pinned by byte tests.

### ADR-077: Chord inserts a named-quality triad
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: harmonize spawned voices from existing notes and pedal added one note, but no tool inserted a chord of a chosen quality at a chosen beat.
- **Decision**: Append a major, minor, diminished or augmented triad at the requested beat and duration with 0.7 velocity and score-default timbre, preserving existing notes.
- **Consequences**: All four qualities, preservation, exact round-trip and argument validation are pinned by byte tests.

### ADR-078: Distort applies bounded soft saturation
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: gain trimmed linearly with a hard clamp and crush quantized levels, leaving no smooth saturation effect.
- **Decision**: Map each sample through (1+drive)*x/(1+drive*|x|), which is bounded, preserves sign, boosts quiet samples more than loud ones, and is byte-identical at drive 0.
- **Consequences**: Zero-drive identity, bounded output, monotonic magnitude growth, determinism and validation are pinned by byte tests.

### ADR-079: Bandpass cascades high-pass and low-pass stages
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: Lowpass and highpass each addressed one end of the spectrum but removing both needed two separate commands.
- **Decision**: Apply a one-pole high-pass then low-pass in a single frame pass with shared deterministic state, requiring both coefficients in 0..1.
- **Consequences**: Edge coefficient combinations, constant-signal removal, bounded output and validation are pinned by byte tests.

### ADR-080: Intervals reports melodic step sequences
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: stats reported pitch range and density but not the contour between consecutive notes.
- **Decision**: Print signed semitone steps between consecutive sounding pitches, with extremes and up/down/level counts, skipping rests and handling single-note scores.
- **Consequences**: Exact step sequences, rest handling, single-note output and empty-score rejection are pinned by byte tests.

### ADR-081: Reverse-region flips an exact time window
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: reverse-wav flipped whole files, with no way to reverse just one phrase inside a WAV.
- **Decision**: Round the window to frames at the source rate and reverse only that slice, leaving head and tail unchanged and out-of-range windows as no-ops.
- **Consequences**: Window reversal, head/tail preservation, full-file equivalence and range validation are pinned by byte tests.

### ADR-082: Repeat-wav tiles a finished WAV into a loop
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: loop repeated scores but there was no way to repeat an already-processed WAV without external editing.
- **Decision**: Append the source frame buffer N times with a bounded count, preserving the sample rate and all audio bytes; N=1 is byte-identical.
- **Consequences**: Exact tiling, frame scaling, rate preservation and count validation are pinned by byte tests.

### ADR-083: Crossfade blends two rendered WAVs
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: join crossfaded scores before rendering, with no way to overlap two already-rendered files.
- **Decision**: Match rates, overlap the tail of the first and head of the second over a frame-rounded duration with linear weights, cap the overlap to the shorter input, and concatenate at zero duration.
- **Consequences**: Zero-overlap concatenation, exact midpoint blending, length and rate preservation, and validation are pinned by byte tests.

### ADR-084: Limit caps WAV samples at a ceiling
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: gain trimmed uniformly and distort saturated smoothly, but no tool imposed a hard symmetric ceiling on every sample.
- **Decision**: Clamp each sample to [-limit, limit] with a bounded coefficient, preserving frame count and rate; full scale is byte-identical on ordinary signals.
- **Consequences**: Full-scale identity, exact capping, determinism and validation are pinned by byte tests.

### ADR-085: Sequence appends transposed phrase copies
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: loop repeated a phrase exactly and transposed shifted it once, but building a transposed sequence needed both steps manually.
- **Decision**: Append one copy per requested semitone shift at successive phrase endings, transposing sounding pitches when in range and preserving rhythm, velocity and timbre.
- **Consequences**: Offset placement, transposition, exact repeats, round-trip and shift validation are pinned by byte tests.

### ADR-086: Mid/side transform converts stereo components
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: channels and width could fold or widen stereo but not represent it as mid and side signals.
- **Decision**: Apply the √2-normalized sum/difference butterfly, which is self-inverse for both encode and decode, with mode validated for caller clarity.
- **Consequences**: Mono-to-zero-side, mid-only dual-mono, bounded round-trip and mode validation are pinned by byte tests.

### ADR-087: Noise adds seeded bounded samples to a WAV
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: seed generation and humanize used randomness in scores, but no effect added reproducible noise to an existing WAV.
- **Decision**: Generate xorshift64* samples from the seed, scale by amount, add to each channel and clamp; amount 0 is byte-identical and equal seeds reproduce equal output.
- **Consequences**: Identity, determinism, seed variation, bounded output and validation are pinned by byte tests.

### ADR-088: Midi-render turns a MIDI file directly into WAV
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: import-midi and render were separate steps, so playing a MIDI file required writing and re-reading an intermediate score.
- **Decision**: Import the MIDI file to a Score and feed it through the shared render pipeline at the chosen sample rate, producing a deterministic WAV with a digest.
- **Consequences**: Byte-equality with direct rendering of the source score, sample-rate control and invalid-file rejection are pinned by byte tests.

### ADR-089: Pan-sweep applies auto-pan to existing WAVs
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: autopan was render-time only, so processed WAVs could not receive a moving stereo image.
- **Decision**: Reuse the shared equal-power sine LFO on decoded frames with --autopan-rate and --autopan-depth, requiring a positive rate; depth 0 is byte-identical.
- **Consequences**: Zero-depth identity, sweep, determinism, bounded gains and validation are pinned by byte tests.

### ADR-090: Mswidth scales the mid/side side component
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: width folded or widened with L/R mixing but could not scale the side component of a mid/side image.
- **Decision**: Scale the side component by a bounded factor, with 0 collapsing to dual mono, 1 byte-identical, and values above 1 widening the image within clamped bounds.
- **Consequences**: Mono collapse, identity, widening, bounded output and validation are pinned by byte tests.

### ADR-091: Vibrato warps a WAV timeline with a sine LFO
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: score vibrato applied at render time, leaving finished WAVs without pitch modulation.
- **Decision**: Warp sample positions by a sine LFO of chosen rate and frame depth with deterministic linear interpolation; depth 0 is byte-identical and length is preserved.
- **Consequences**: Identity, determinism, length preservation, bounded output and validation are pinned by byte tests.

### ADR-092: Chorus blends an LFO-modulated delayed copy
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: echo was a fixed feedback delay with no modulated short delay mixed into the dry signal.
- **Decision**: Mix the dry signal with a copy delayed by base_delay plus a sine swing, using deterministic interpolation and a wet coefficient; zero mix is a no-op.
- **Consequences**: Zero-mix identity, determinism, length preservation, bounded output and validation are pinned by byte tests.

### ADR-093: Compress reduces loud passages above a threshold
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: limit capped peaks and normalize scaled a whole file, but neither reduced only the audio above a threshold.
- **Decision**: Map samples above the threshold to threshold + (magnitude - threshold)/ratio with sign preserved; threshold 1 or ratio 1 is byte-identical.
- **Consequences**: Identity, quiet preservation, loud attenuation, sign preservation, determinism and validation are pinned by byte tests.

### ADR-094: Sustain extends notes to the next onset
- **Date**: 2026-11-19
- **Status**: Accepted
- **Context**: augment scaled durations and stretch shifted timing, but no tool made a line legato by joining each note to the next onset.
- **Decision**: Set each note's duration to the distance to the next distinct onset, leaving the final note and all other fields untouched.
- **Consequences**: Legato extension, chord handling, final-note preservation and round-trip are pinned by byte tests.
