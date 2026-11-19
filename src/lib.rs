//! # Coda
//!
//! A deterministic, dependency-free sound compiler. A small textual score is
//! parsed, rendered with pure-Rust DSP, and encoded as 16-bit PCM WAV. The same
//! input always yields bit-identical output: no clock, no randomness (unless
//! seeded explicitly), no network, no crates.

pub mod compare;
pub mod digest;
pub mod envelope;
pub mod gen;
pub mod join;
pub mod lint;
pub mod midi;
pub mod midi_import;
pub mod notes;
pub mod render;
pub mod score;
pub mod stats;
pub mod transpose;
pub mod vary;
pub mod wave;
