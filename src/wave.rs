//! Deterministic waveform generators. `phase` is always in `[0, 1)`.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wave {
    Sine,
    Square,
    Triangle,
    Saw,
    Noise,
    /// Narrow 25% duty-cycle pulse.
    Pulse,
    /// Additive sine harmonics (fundamental + 2nd + 3rd).
    Organ,
    /// Sine with a decaying shape inside each cycle.
    Pluck,
}

impl Wave {
    pub const ALL: [Wave; 8] = [
        Wave::Sine,
        Wave::Square,
        Wave::Triangle,
        Wave::Saw,
        Wave::Noise,
        Wave::Pulse,
        Wave::Organ,
        Wave::Pluck,
    ];

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "sine" => Some(Wave::Sine),
            "square" => Some(Wave::Square),
            "triangle" => Some(Wave::Triangle),
            "saw" => Some(Wave::Saw),
            "noise" => Some(Wave::Noise),
            "pulse" => Some(Wave::Pulse),
            "organ" => Some(Wave::Organ),
            "pluck" => Some(Wave::Pluck),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Wave::Sine => "sine",
            Wave::Square => "square",
            Wave::Triangle => "triangle",
            Wave::Saw => "saw",
            Wave::Noise => "noise",
            Wave::Pulse => "pulse",
            Wave::Organ => "organ",
            Wave::Pluck => "pluck",
        }
    }

    /// One sample in `[-1, 1]`. `seed` only affects Noise, and makes it
    /// reproducible: the same seed always yields the same sample sequence.
    pub fn sample(self, phase: f64, seed: u64) -> f64 {
        use std::f64::consts::TAU;
        match self {
            Wave::Sine => (TAU * phase).sin(),
            Wave::Square => {
                if phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            Wave::Triangle => 1.0 - 4.0 * (phase - 0.5).abs(),
            Wave::Saw => 2.0 * phase - 1.0,
            Wave::Noise => {
                let n = (phase * 4_294_967_296.0) as u64 ^ seed;
                let x = n
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                ((x >> 32) as f64 / u32::MAX as f64) * 2.0 - 1.0
            }
            Wave::Pulse => {
                if phase < 0.25 {
                    1.0
                } else {
                    -1.0
                }
            }
            Wave::Organ => {
                let sum = (TAU * phase).sin()
                    + 0.5 * (2.0 * TAU * phase).sin()
                    + 0.25 * (3.0 * TAU * phase).sin();
                sum / 1.75
            }
            Wave::Pluck => (TAU * phase).sin() * (1.0 - phase).powi(2),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_samples() {
        for wave in Wave::ALL {
            for i in 0..256 {
                let v = wave.sample(i as f64 / 256.0, 7);
                assert!((-1.0..=1.0).contains(&v), "{wave:?} produced {v}");
            }
        }
    }

    #[test]
    fn parse_and_name() {
        for w in Wave::ALL {
            assert_eq!(Wave::parse(w.name()), Some(w));
        }
        assert_eq!(Wave::parse("SINE"), Some(Wave::Sine));
        assert_eq!(Wave::parse("bogus"), None);
    }

    #[test]
    fn noise_is_reproducible() {
        assert_eq!(Wave::Noise.sample(0.25, 9), Wave::Noise.sample(0.25, 9));
        assert_ne!(Wave::Noise.sample(0.25, 9), Wave::Noise.sample(0.25, 10));
    }

    #[test]
    fn new_waves_have_distinct_timbres() {
        let base = Wave::Sine.sample(0.125, 0);
        for wave in [Wave::Pulse, Wave::Organ, Wave::Pluck] {
            assert!((wave.sample(0.125, 0) - base).abs() > 0.01, "{wave:?}");
        }
    }
}
