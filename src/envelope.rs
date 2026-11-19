//! ADSR amplitude envelope. All times are in seconds.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Envelope {
    pub attack: f64,
    pub decay: f64,
    pub sustain: f64,
    pub release: f64,
}

impl Default for Envelope {
    fn default() -> Self {
        Envelope {
            attack: 0.01,
            decay: 0.08,
            sustain: 0.65,
            release: 0.15,
        }
    }
}

impl Envelope {
    /// Named articulation presets, in help order.
    pub const PRESETS: [&'static str; 5] = ["pluck", "stab", "pad", "organ", "bow"];

    /// Look up a named envelope preset.
    pub fn preset(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "pluck" => Some(Envelope {
                attack: 0.002,
                decay: 0.12,
                sustain: 0.0,
                release: 0.08,
            }),
            "stab" => Some(Envelope {
                attack: 0.005,
                decay: 0.08,
                sustain: 0.1,
                release: 0.05,
            }),
            "pad" => Some(Envelope {
                attack: 0.35,
                decay: 0.4,
                sustain: 0.75,
                release: 0.9,
            }),
            "organ" => Some(Envelope {
                attack: 0.0,
                decay: 0.0,
                sustain: 1.0,
                release: 0.05,
            }),
            "bow" => Some(Envelope {
                attack: 0.18,
                decay: 0.12,
                sustain: 0.7,
                release: 0.35,
            }),
            _ => None,
        }
    }

    /// Amplitude at time `t` for a note held for `duration` seconds.
    pub fn amplitude(&self, t: f64, duration: f64) -> f64 {
        if t < 0.0 {
            return 0.0;
        }
        if self.attack > 0.0 && t < self.attack {
            return t / self.attack;
        }
        let level = if self.decay > 0.0 && t < self.attack + self.decay {
            1.0 - (1.0 - self.sustain) * ((t - self.attack) / self.decay)
        } else {
            self.sustain
        };
        if t <= duration {
            return level.max(0.0);
        }
        if self.release <= 0.0 {
            return 0.0;
        }
        let r = (t - duration) / self.release;
        if r >= 1.0 {
            0.0
        } else {
            self.sustain * (1.0 - r)
        }
    }

    /// Extra seconds a note contributes after its nominal end.
    pub fn tail(&self) -> f64 {
        self.release.max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_roundtrip_and_bounds() {
        for name in Envelope::PRESETS {
            let preset = Envelope::preset(name).unwrap_or_else(|| panic!("missing {name}"));
            assert!(preset.attack >= 0.0 && preset.decay >= 0.0 && preset.release >= 0.0);
            assert!((0.0..=1.0).contains(&preset.sustain));
        }
        assert_eq!(Envelope::preset("PAD"), Envelope::preset("pad"));
        assert_eq!(Envelope::preset("bogus"), None);
    }

    #[test]
    fn shape() {
        let e = Envelope {
            attack: 0.1,
            decay: 0.2,
            sustain: 0.5,
            release: 0.4,
        };
        assert_eq!(e.amplitude(-1.0, 1.0), 0.0);
        assert!(e.amplitude(0.0, 1.0) < 0.01);
        assert!(e.amplitude(0.1, 1.0) > 0.99);
        assert!((e.amplitude(0.3, 1.0) - 0.5).abs() < 1e-9);
        assert!((e.amplitude(0.7, 1.0) - 0.5).abs() < 1e-9);
        assert!((e.amplitude(1.2, 1.0) - 0.25).abs() < 1e-9);
        assert_eq!(e.amplitude(1.5, 1.0), 0.0);
    }

    #[test]
    fn zero_release_stops_hard() {
        let e = Envelope {
            attack: 0.0,
            decay: 0.0,
            sustain: 1.0,
            release: 0.0,
        };
        assert_eq!(e.tail(), 0.0);
        assert_eq!(e.amplitude(0.5, 0.25), 0.0);
        assert_eq!(e.amplitude(0.1, 0.25), 1.0);
    }
}
