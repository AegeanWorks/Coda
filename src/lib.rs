//! # Coda
//!
//! A deterministic, dependency-free sound compiler. A small textual score is
//! parsed, rendered with pure-Rust DSP, and encoded as 16-bit PCM WAV. The same
//! input always yields bit-identical output: no clock, no randomness (unless
//! seeded explicitly), no network, no crates.

pub mod accent;
pub mod analyze;
pub mod arpeggiate;
pub mod autopan;
pub mod bitdepth;
pub mod channels;
pub mod compare;
pub mod concat;
pub mod dc;
pub mod diatonic;
pub mod digest;
pub mod echo;
pub mod envelope;
pub mod extract;
pub mod fade;
pub mod fade_presets;
pub mod gain;
pub mod gate;
pub mod gen;
pub mod harmonize;
pub mod humanize;
pub mod invert;
pub mod join;
pub mod lint;
pub mod loop_score;
pub mod midi;
pub mod midi_import;
pub mod mix;
pub mod normalize;
pub mod notes;
pub mod pad;
pub mod quantize;
pub mod register;
pub mod render;
pub mod report;
pub mod resample;
pub mod reverse;
pub mod score;
pub mod stamp;
pub mod stats;
pub mod stretch;
pub mod swing;
pub mod templates;
pub mod transpose;
pub mod tremolo;
pub mod trim;
pub mod vary;
pub mod verify;
pub mod wav_reverse;
pub mod wave;
pub mod width;
