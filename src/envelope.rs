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
