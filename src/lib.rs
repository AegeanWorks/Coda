//! # Coda
//!
//! A deterministic, dependency-free sound compiler. A small textual score is
//! parsed, rendered with pure-Rust DSP, and encoded as 16-bit PCM WAV. The same
//! input always yields bit-identical output: no clock, no randomness (unless
//! seeded explicitly), no network, no crates.

pub mod analyze;
pub mod autopan;
pub mod bitdepth;
pub mod channels;
pub mod compare;
pub mod concat;
pub mod diatonic;
pub mod digest;
pub mod echo;
pub mod envelope;
pub mod fade;
pub mod fade_presets;
pub mod gen;
pub mod humanize;
pub mod join;
pub mod lint;
pub mod loop_score;
pub mod midi;
pub mod midi_import;
pub mod mix;
pub mod normalize;
pub mod notes;
pub mod quantize;
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
pub mod trim;
pub mod vary;
pub mod verify;
pub mod wav_reverse;
pub mod wave;
