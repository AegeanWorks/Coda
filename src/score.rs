//! Tiny line-oriented score language.
//!
//! ```text
//! # comment
//! tempo 120          # beats per minute
//! wave triangle      # sine | square | triangle | saw | noise
//! gain 0.3
//! attack 0.01
//! decay 0.08
//! sustain 0.65
//! release 0.15
//! vibrato 5 20       # score-wide rate in Hz, depth in cents
//! vib 8 45           # per-note override; `vib default` restores
//! inst saw           # per-note oscillator; `inst default` restores
//! pan -0.6           # running stereo position: -1 left, 0 centre, 1 right
//! env 0 0 1 0        # per-note ADSR override; `env default` resets
//! preset pad         # named ADSR; preset default resets
//! repeat 4           # repeat the next note/rest events
//! note C4 1          # sequential note, duration in beats
//! note E4 1 0.6      # optional velocity (0..1)
//! rest 0.5           # silence, advances the cursor
//! at 3 G4 2          # note at an explicit beat position
//! ```

use crate::envelope::Envelope;
use crate::wave::Wave;

#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub pitch: String,
    pub start: f64,
    pub beats: f64,
    /// Per-note gain multiplier, `0.0..=1.0` (default 1.0).
    pub velocity: f64,
    /// Stereo position, `-1.0` left to `1.0` right (default 0.0, centre).
    pub pan: f64,
    /// The ADSR envelope used for this note (overridable with `env`).
    pub envelope: Envelope,
    /// The vibrato used for this note (overridable with `vib`).
    pub vibrato: Vibrato,
    /// The oscillator used for this note (overridable with `inst`).
    pub wave: Wave,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vibrato {
    pub rate: f64,
    pub cents: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Score {
    pub tempo: f64,
    pub wave: Wave,
    pub gain: f64,
    pub envelope: Envelope,
    pub vibrato: Vibrato,
    pub notes: Vec<Note>,
}

impl Default for Score {
    fn default() -> Self {
        Score {
            tempo: 120.0,
            wave: Wave::Triangle,
            gain: 0.3,
            envelope: Envelope::default(),
            vibrato: Vibrato {
                rate: 0.0,
                cents: 0.0,
            },
            notes: Vec::new(),
        }
    }
}

fn line_error(idx: usize, message: String) -> String {
    format!("line {}: {}", idx + 1, message)
}

fn number(value: Option<&str>, what: &str) -> Result<f64, String> {
    let raw = value.ok_or_else(|| format!("{what} needs a number"))?;
    raw.parse::<f64>()
        .map_err(|_| format!("{what}: expected a number, got '{raw}'"))
}

fn optional_pan(value: Option<&str>, idx: usize, current: f64) -> Result<f64, String> {
    match value {
        None => Ok(current),
        Some(raw) => {
            let p = raw
                .parse::<f64>()
                .map_err(|_| line_error(idx, format!("pan: expected a number, got '{raw}'")))?;
            if !(-1.0..=1.0).contains(&p) {
                return Err(line_error(idx, "pan must be between -1 and 1".into()));
            }
            Ok(p)
        }
    }
}

fn positive_int(value: Option<&str>, what: &str) -> Result<usize, String> {
    let raw = value.ok_or_else(|| format!("{what} needs an integer"))?;
    let n = raw
        .parse::<usize>()
        .map_err(|_| format!("{what}: expected a positive integer, got '{raw}'"))?;
    if n == 0 {
        return Err(format!("{what} must be at least 1"));
    }
    Ok(n)
}

fn optional_velocity(value: Option<&str>, idx: usize) -> Result<f64, String> {
    match value {
        None => Ok(1.0),
        Some(raw) => {
            let v = raw.parse::<f64>().map_err(|_| {
                line_error(idx, format!("velocity: expected a number, got '{raw}'"))
            })?;
            if !(0.0..=1.0).contains(&v) {
                return Err(line_error(idx, "velocity must be between 0 and 1".into()));
            }
            Ok(v)
        }
    }
}

pub fn parse(text: &str) -> Result<Score, String> {
    let mut score = Score::default();
    let mut cursor = 0.0;
    let mut repeat = 1_usize;
    let mut pan = 0.0_f64;
    let mut note_envelope = score.envelope;
    let mut note_vibrato = score.vibrato;
    let mut note_wave = score.wave;

    for (idx, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split_whitespace();
        let directive = fields.next().unwrap();
        let a = fields.next();
        let b = fields.next();

        match directive {
            "tempo" => score.tempo = number(a, "tempo").map_err(|m| line_error(idx, m))?,
            "wave" => {
                let raw = a.ok_or_else(|| line_error(idx, "wave needs a name".into()))?;
                score.wave = Wave::parse(raw)
                    .ok_or_else(|| line_error(idx, format!("unknown wave '{raw}'")))?;
                note_wave = score.wave;
            }
            "inst" => {
                note_wave = match a.map(str::to_ascii_lowercase).as_deref() {
                    Some("default") => score.wave,
                    _ => {
                        let raw =
                            a.ok_or_else(|| line_error(idx, "inst needs a wave name".into()))?;
                        Wave::parse(raw)
                            .ok_or_else(|| line_error(idx, format!("unknown wave '{raw}'")))?
                    }
                };
            }
            "gain" => score.gain = number(a, "gain").map_err(|m| line_error(idx, m))?,
            "attack" => {
                score.envelope.attack = number(a, "attack").map_err(|m| line_error(idx, m))?;
                note_envelope = score.envelope;
            }
            "decay" => {
                score.envelope.decay = number(a, "decay").map_err(|m| line_error(idx, m))?;
                note_envelope = score.envelope;
            }
            "sustain" => {
                score.envelope.sustain = number(a, "sustain").map_err(|m| line_error(idx, m))?;
                note_envelope = score.envelope;
            }
            "release" => {
                score.envelope.release = number(a, "release").map_err(|m| line_error(idx, m))?;
                note_envelope = score.envelope;
            }
            "env" => {
                note_envelope = match a.map(str::to_ascii_lowercase).as_deref() {
                    Some("default") => score.envelope,
                    _ => {
                        let env = Envelope {
                            attack: number(a, "env attack").map_err(|m| line_error(idx, m))?,
                            decay: number(b, "env decay").map_err(|m| line_error(idx, m))?,
                            sustain: number(fields.next(), "env sustain")
                                .map_err(|m| line_error(idx, m))?,
                            release: number(fields.next(), "env release")
                                .map_err(|m| line_error(idx, m))?,
                        };
                        if env.attack < 0.0
                            || env.decay < 0.0
                            || env.release < 0.0
                            || !(0.0..=1.0).contains(&env.sustain)
                        {
                            return Err(line_error(idx, "invalid envelope values".into()));
                        }
                        env
                    }
                };
            }
            "preset" => {
                note_envelope = match a {
                    Some("default") => score.envelope,
                    Some(name) => Envelope::preset(name).ok_or_else(|| {
                        line_error(idx, format!("unknown envelope preset '{name}'"))
                    })?,
                    None => return Err(line_error(idx, "preset needs a name".into())),
                };
            }
            "vibrato" => {
                let rate = number(a, "vibrato rate").map_err(|m| line_error(idx, m))?;
                let cents = number(b, "vibrato cents").map_err(|m| line_error(idx, m))?;
                score.vibrato = Vibrato { rate, cents };
                note_vibrato = score.vibrato;
            }
            "vib" => {
                note_vibrato = match a.map(str::to_ascii_lowercase).as_deref() {
                    Some("default") => score.vibrato,
                    _ => {
                        let vib = Vibrato {
                            rate: number(a, "vib rate").map_err(|m| line_error(idx, m))?,
                            cents: number(b, "vib cents").map_err(|m| line_error(idx, m))?,
                        };
                        if !(0.0..=100.0).contains(&vib.rate)
                            || !(0.0..=1200.0).contains(&vib.cents)
                        {
                            return Err(line_error(idx, "invalid vibrato values".into()));
                        }
                        vib
                    }
                };
            }
            "repeat" => {
                repeat = positive_int(a, "repeat").map_err(|m| line_error(idx, m))?;
            }
            "pan" => {
                pan = optional_pan(a, idx, 0.0)?;
            }
            "note" => {
                let pitch = a
                    .ok_or_else(|| line_error(idx, "note needs a pitch".into()))?
                    .to_string();
                let beats = number(b, "note beats").map_err(|m| line_error(idx, m))?;
                let velocity = optional_velocity(fields.next(), idx)?;
                let note_pan = optional_pan(fields.next(), idx, pan)?;
                for n in 0..repeat {
                    score.notes.push(Note {
                        pitch: pitch.clone(),
                        start: cursor + n as f64 * beats,
                        beats,
                        velocity,
                        pan: note_pan,
                        envelope: note_envelope,
                        vibrato: note_vibrato,
                        wave: note_wave,
                    });
                }
                cursor += beats * repeat as f64;
            }
            "at" => {
                let start = number(a, "at beat").map_err(|m| line_error(idx, m))?;
                let pitch = b
                    .ok_or_else(|| line_error(idx, "at needs a pitch".into()))?
                    .to_string();
                let beats = number(fields.next(), "at beats").map_err(|m| line_error(idx, m))?;
                let velocity = optional_velocity(fields.next(), idx)?;
                let note_pan = optional_pan(fields.next(), idx, pan)?;
                score.notes.push(Note {
                    pitch,
                    start,
                    beats,
                    velocity,
                    pan: note_pan,
                    envelope: note_envelope,
                    vibrato: note_vibrato,
                    wave: note_wave,
                });
            }
            "rest" => {
                let beats = number(a, "rest beats").map_err(|m| line_error(idx, m))?;
                // `"-"` is the silence marker: note names cannot contain it,
                // so the renderer simply skips these entries.
                for n in 0..repeat {
                    score.notes.push(Note {
                        pitch: "-".into(),
                        start: cursor + n as f64 * beats,
                        beats,
                        velocity: 1.0,
                        pan: 0.0,
                        envelope: note_envelope,
                        vibrato: note_vibrato,
                        wave: note_wave,
                    });
                }
                cursor += beats * repeat as f64;
            }
            other => return Err(line_error(idx, format!("unknown directive '{other}'"))),
        }
    }

    if score.tempo <= 0.0 {
        return Err("tempo must be greater than zero".into());
    }
    Ok(score)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_defaults_and_notes() {
        let score =
            parse("# a tiny tune\ntempo 90\nwave saw\nnote C4 0.5\nnote E4 0.5\nat 2 G4 1\n")
                .unwrap();
        assert_eq!(score.tempo, 90.0);
        assert_eq!(score.wave, Wave::Saw);
        assert_eq!(score.notes.len(), 3);
        assert_eq!(score.notes[0].start, 0.0);
        assert_eq!(score.notes[1].start, 0.5);
        assert_eq!(score.notes[2].start, 2.0);
    }

    #[test]
    fn errors_carry_line_numbers() {
        let err = parse("tempo 120\nbogus 1\n").unwrap_err();
        assert!(err.starts_with("line 2:"), "{err}");
        let err = parse("note C4\n").unwrap_err();
        assert!(err.contains("note beats"), "{err}");
    }

    #[test]
    fn rejects_zero_tempo() {
        assert!(parse("tempo 0\n").is_err());
    }

    #[test]
    fn repeat_expands_sequential_events() {
        let score = parse("repeat 3\nnote C4 0.5\nrepeat 2\nrest 1\nat 9 G4 1\n").unwrap();
        assert_eq!(score.notes.len(), 6);
        assert_eq!(score.notes[0].start, 0.0);
        assert_eq!(score.notes[1].start, 0.5);
        assert_eq!(score.notes[2].start, 1.0);
        assert_eq!(score.notes[3].pitch, "-");
        assert_eq!(score.notes[3].start, 1.5);
        assert_eq!(score.notes[4].start, 2.5);
        assert_eq!(score.notes[5].start, 9.0);
        assert!(parse("repeat 0\nnote C4 1\n").is_err());
    }

    #[test]
    fn vibrato_parses() {
        let score = parse("vibrato 5 20\n").unwrap();
        assert_eq!(
            score.vibrato,
            Vibrato {
                rate: 5.0,
                cents: 20.0
            }
        );
    }

    #[test]
    fn per_note_wave_override() {
        let score =
            parse("wave sine\nnote C4 1\ninst saw\nnote E4 1\ninst default\nnote G4 1\n").unwrap();
        assert_eq!(score.notes[0].wave, Wave::Sine);
        assert_eq!(score.notes[1].wave, Wave::Saw);
        assert_eq!(score.notes[2].wave, Wave::Sine);
        assert!(parse("inst nonsense\nnote C4 1\n").is_err());
    }

    #[test]
    fn per_note_vibrato_override() {
        let score = parse("vibrato 5 20\nnote C4 1\nvib 8 50\nnote E4 1\nvib default\nnote G4 1\n")
            .unwrap();
        assert_eq!(score.notes[0].vibrato.rate, 5.0);
        assert_eq!(score.notes[1].vibrato.rate, 8.0);
        assert_eq!(score.notes[1].vibrato.cents, 50.0);
        assert_eq!(score.notes[2].vibrato, score.vibrato);
        assert!(parse("vib 5 2000\nnote C4 1\n").is_err());
        assert!(parse("vib 200 20\nnote C4 1\n").is_err());
    }

    #[test]
    fn preset_sets_and_restores_note_envelope() {
        let score = parse("preset pad\nnote C4 1\npreset default\nnote E4 1\n").unwrap();
        assert_eq!(score.notes[0].envelope, Envelope::preset("pad").unwrap());
        assert_eq!(score.notes[1].envelope, score.envelope);
        assert!(parse("preset bogus\nnote C4 1\n").is_err());
    }

    #[test]
    fn per_note_envelope_override() {
        let score =
            parse("attack 0.1\nnote C4 1\nenv 0 0 1 0\nnote E4 1\nenv default\nnote G4 1\n")
                .unwrap();
        assert_eq!(score.notes[0].envelope.attack, 0.1);
        assert_eq!(score.notes[1].envelope.attack, 0.0);
        assert_eq!(score.notes[1].envelope.sustain, 1.0);
        assert_eq!(score.notes[2].envelope.attack, 0.1);
        assert!(parse("env 0 0 2 0\nnote C4 1\n").is_err());
    }

    #[test]
    fn pan_parses_and_is_bounded() {
        let score = parse("pan -0.5\nnote C4 1\nat 2 E4 1 0.8 0.7\n").unwrap();
        assert_eq!(score.notes[0].pan, -0.5);
        assert_eq!(score.notes[1].pan, 0.7);
        assert!(parse("pan 2\nnote C4 1\n").is_err());
    }

    #[test]
    fn velocity_and_rest() {
        let score = parse("note C4 1 0.5\nrest 0.5\nat 3 E4 1 0.8\n").unwrap();
        assert_eq!(score.notes[0].velocity, 0.5);
        assert_eq!(score.notes[1].pitch, "-");
        assert_eq!(score.notes[1].start, 1.0);
        assert_eq!(score.notes[1].beats, 0.5);
        assert_eq!(score.notes[2].start, 3.0);
        assert_eq!(score.notes[2].velocity, 0.8);
        assert!(parse("note C4 1 1.5\n").is_err());
    }
}
