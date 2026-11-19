//! Reproducible generative composition: a seed always produces the same score.
//!
//! Three arrangement styles turn the same scale/seed into genuinely different
//! music: a melodic `flow`, a running `arp` with bass, and block `chords`.

use crate::envelope::Envelope;
use crate::notes;
use crate::score::{Note, Score};
use crate::wave::Wave;

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[(self.next() as usize) % items.len()]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scale {
    Major,
    Minor,
    Pentatonic,
    Chromatic,
}

impl Scale {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "major" => Some(Scale::Major),
            "minor" => Some(Scale::Minor),
            "pentatonic" => Some(Scale::Pentatonic),
            "chromatic" => Some(Scale::Chromatic),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Scale::Major => "major",
            Scale::Minor => "minor",
            Scale::Pentatonic => "pentatonic",
            Scale::Chromatic => "chromatic",
        }
    }

    pub fn degrees(self) -> &'static [i32] {
        match self {
            Scale::Major => &[0, 2, 4, 5, 7, 9, 11],
            Scale::Minor => &[0, 2, 3, 5, 7, 8, 10],
            Scale::Pentatonic => &[0, 3, 5, 7, 10],
            Scale::Chromatic => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Flow,
    Arp,
    Chords,
}

impl Style {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "flow" => Some(Style::Flow),
            "arp" => Some(Style::Arp),
            "chords" => Some(Style::Chords),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Style::Flow => "flow",
            Style::Arp => "arp",
            Style::Chords => "chords",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GenOptions {
    pub bars: u32,
    pub wave: Wave,
    pub scale: Scale,
    pub tempo: f64,
    pub style: Style,
}

impl Default for GenOptions {
    fn default() -> Self {
        GenOptions {
            bars: 4,
            wave: Wave::Triangle,
            scale: Scale::Major,
            tempo: 120.0,
            style: Style::Flow,
        }
    }
}

fn pitched(midi: i32, start: f64, beats: f64, velocity: f64, pan: f64) -> Note {
    Note {
        pitch: notes::name(midi),
        start,
        beats,
        velocity,
        pan,
        envelope: Envelope::default(),
        vibrato: crate::score::Vibrato {
            rate: 0.0,
            cents: 0.0,
        },
    }
}

fn rest(start: f64, beats: f64) -> Note {
    Note {
        pitch: "-".into(),
        start,
        beats,
        velocity: 1.0,
        pan: 0.0,
        envelope: Envelope::default(),
        vibrato: crate::score::Vibrato {
            rate: 0.0,
            cents: 0.0,
        },
    }
}

fn flow(rng: &mut Lcg, degrees: &[i32], bars: u32) -> Vec<Note> {
    const PATTERNS: [&[f64]; 4] = [
        &[1.0, 1.0, 1.0, 1.0],
        &[0.5, 0.5, 1.0, 1.0, 1.0],
        &[1.0, 0.5, 0.5, 1.0, 1.0],
        &[1.5, 0.5, 1.0, 1.0],
    ];
    let mut out = Vec::new();
    let mut index = 0_i32;
    let mut direction = 1_i32;
    let span = degrees.len() as i32;

    for bar in 0..bars {
        let pattern = rng.pick(&PATTERNS);
        let mut at = (bar * 4) as f64;
        for &length in pattern {
            if rng.next().is_multiple_of(9) {
                out.push(rest(at, length));
            } else {
                if rng.next().is_multiple_of(5) {
                    direction = -direction;
                }
                index = (index + direction).rem_euclid(span);
                let octave = if rng.next().is_multiple_of(3) { 5 } else { 4 };
                let midi = 60 + degrees[index as usize] + (octave - 4) * 12;
                let velocity = 0.6 + (rng.next() % 30) as f64 / 100.0;
                let pan = (rng.next() % 21) as f64 / 10.0 - 1.0;
                out.push(pitched(midi, at, length, velocity, pan));
            }
            at += length;
        }
        if bar % 2 == 0 {
            out.push(pitched(36 + degrees[0], (bar * 4) as f64, 2.0, 0.5, 0.0));
        }
    }
    out
}

fn arp(rng: &mut Lcg, degrees: &[i32], bars: u32) -> Vec<Note> {
    let mut out = Vec::new();
    let span = degrees.len();
    for bar in 0..bars {
        let base = bar * 4;
        let root = (rng.next() as usize) % span;
        out.push(pitched(36 + degrees[root], base as f64, 2.0, 0.55, 0.0));
        for i in 0..8 {
            let degree = degrees[(root + i) % span];
            let octave = if i >= 4 { 5 } else { 4 };
            let pan = if i % 2 == 0 { -0.5 } else { 0.5 };
            out.push(pitched(
                60 + degree + (octave - 4) * 12,
                base as f64 + i as f64 * 0.5,
                0.45,
                0.6,
                pan,
            ));
        }
    }
    out
}

fn chords(rng: &mut Lcg, degrees: &[i32], bars: u32) -> Vec<Note> {
    let mut out = Vec::new();
    let span = degrees.len();
    for bar in 0..bars {
        let base = (bar * 4) as f64;
        let first = (rng.next() as usize) % span;
        for &degree in &[first, (first + 2) % span, (first + 4) % span] {
            out.push(pitched(60 + degrees[degree], base, 1.5, 0.55, 0.0));
        }
        out.push(pitched(36 + degrees[first], base, 1.5, 0.5, 0.0));

        let second = (first + 3) % span;
        let second_at = base + 2.0;
        for &degree in &[second, (second + 2) % span, (second + 4) % span] {
            out.push(pitched(60 + degrees[degree], second_at, 1.5, 0.55, 0.0));
        }
        out.push(pitched(36 + degrees[second], second_at, 1.5, 0.5, 0.0));

        let melody = (rng.next() as usize) % span;
        out.push(pitched(72 + degrees[melody], base + 3.5, 0.5, 0.7, 0.3));
    }
    out
}

/// Build an arranged score from a 64-bit seed.
pub fn generate(seed: u64, options: GenOptions) -> Score {
    let degrees = options.scale.degrees();
    let mut rng = Lcg(seed);
    let notes = match options.style {
        Style::Flow => flow(&mut rng, degrees, options.bars),
        Style::Arp => arp(&mut rng, degrees, options.bars),
        Style::Chords => chords(&mut rng, degrees, options.bars),
    };
    Score {
        tempo: options.tempo,
        wave: options.wave,
        gain: 0.28,
        envelope: Envelope::default(),
        vibrato: crate::score::Vibrato {
            rate: 0.0,
            cents: 0.0,
        },
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_score() {
        assert_eq!(
            generate(42, GenOptions::default()),
            generate(42, GenOptions::default())
        );
    }

    #[test]
    fn different_seed_different_score() {
        assert_ne!(
            generate(42, GenOptions::default()),
            generate(43, GenOptions::default())
        );
    }

    #[test]
    fn different_scale_changes_score() {
        let major = GenOptions {
            scale: Scale::Major,
            ..GenOptions::default()
        };
        let minor = GenOptions {
            scale: Scale::Minor,
            ..GenOptions::default()
        };
        assert_ne!(generate(7, major), generate(7, minor));
    }

    #[test]
    fn different_style_changes_score() {
        let flow = GenOptions::default();
        for style in [Style::Arp, Style::Chords] {
            let opts = GenOptions { style, ..flow };
            assert_ne!(generate(7, flow), generate(7, opts));
        }
    }

    #[test]
    fn style_parse_roundtrip() {
        for style in [Style::Flow, Style::Arp, Style::Chords] {
            assert_eq!(Style::parse(style.name()), Some(style));
        }
        assert_eq!(Style::parse("bogus"), None);
    }

    #[test]
    fn tempo_is_preserved() {
        let opts = GenOptions {
            tempo: 90.0,
            ..GenOptions::default()
        };
        assert_eq!(generate(1, opts).tempo, 90.0);
    }

    #[test]
    fn bar_count_controls_length() {
        let empty = GenOptions {
            bars: 0,
            ..GenOptions::default()
        };
        let three = GenOptions {
            bars: 3,
            ..GenOptions::default()
        };
        assert!(generate(1, empty).notes.is_empty());
        assert!(generate(1, three).notes.len() > 3);
        assert!(generate(1, three).notes.iter().all(|n| n.start < 12.0));
    }

    #[test]
    fn every_style_produces_playable_notes() {
        for style in [Style::Flow, Style::Arp, Style::Chords] {
            let score = generate(
                7,
                GenOptions {
                    bars: 2,
                    style,
                    ..GenOptions::default()
                },
            );
            assert!(!score.notes.is_empty(), "{style:?}");
            assert!(
                score
                    .notes
                    .iter()
                    .all(|n| n.pitch == "-" || notes::frequency(&n.pitch).is_some()),
                "{style:?} has an invalid pitch"
            );
            assert!(score.notes.iter().all(|n| n.beats > 0.0 && n.start >= 0.0));
        }
    }

    #[test]
    fn chords_style_adds_bass_and_stacked_notes() {
        let score = generate(
            7,
            GenOptions {
                bars: 1,
                style: Style::Chords,
                ..GenOptions::default()
            },
        );
        // Two bass notes + two triads + one melody = nine notes in a bar.
        assert_eq!(score.notes.len(), 9);
        let bass = score
            .notes
            .iter()
            .filter(|n| n.pitch.ends_with('2'))
            .count();
        assert_eq!(bass, 2);
    }

    #[test]
    fn arp_style_has_eighth_notes() {
        let score = generate(
            7,
            GenOptions {
                bars: 1,
                style: Style::Arp,
                ..GenOptions::default()
            },
        );
        assert_eq!(score.notes.len(), 9);
        assert_eq!(score.notes.iter().filter(|n| n.beats == 0.45).count(), 8);
    }
}
