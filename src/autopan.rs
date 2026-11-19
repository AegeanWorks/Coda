//! Deterministic equal-power stereo auto-pan driven by a sine LFO.

use crate::render::Frame;

const CENTRE: f64 = std::f64::consts::FRAC_1_SQRT_2;

/// Sweep the stereo image with `pan = depth * sin(2*pi*rate*t)`.
/// Gains are normalised so depth 0 leaves the buffer byte-identical.
pub fn apply_autopan(frames: &mut [Frame], sample_rate: u32, rate: f64, depth: f64) {
    if depth <= 0.0 || !depth.is_finite() || rate <= 0.0 || !rate.is_finite() {
        return;
    }
    let depth = depth.clamp(0.0, 1.0);
    let step = std::f64::consts::TAU * rate / sample_rate as f64;
    for (index, frame) in frames.iter_mut().enumerate() {
        let pan = depth * (step * index as f64).sin();
        let angle = (pan + 1.0) * std::f64::consts::FRAC_PI_4;
        let left = (angle.cos() / CENTRE).min(2.0_f64.sqrt());
        let right = (angle.sin() / CENTRE).min(2.0_f64.sqrt());
        frame.0 = (frame.0 * left).clamp(-1.0, 1.0);
        frame.1 = (frame.1 * right).clamp(-1.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_depth_is_an_identity() {
        let mut frames = vec![(0.5, -0.5); 8];
        let before = frames.clone();
        apply_autopan(&mut frames, 8, 1.0, 0.0);
        assert_eq!(frames, before);
    }

    #[test]
    fn pan_at_start_is_centre() {
        let mut frames = vec![(1.0, 1.0); 4];
        apply_autopan(&mut frames, 8, 1.0, 1.0);
        assert!((frames[0].0 - 1.0).abs() < 1e-12);
        assert!((frames[0].1 - 1.0).abs() < 1e-12);
    }

    #[test]
    fn sine_lfo_changes_balance_and_stays_bounded() {
        let mut frames = vec![(0.5, 0.5); 64];
        apply_autopan(&mut frames, 64, 1.0, 1.0);
        assert!(frames.iter().any(|f| f.0 > f.1), "should sweep left");
        assert!(frames.iter().any(|f| f.1 > f.0), "should sweep right");
        assert!(frames.iter().all(|f| f.0.abs() <= 1.0 && f.1.abs() <= 1.0));
    }

    #[test]
    fn deterministic() {
        let mut a = vec![(0.4, -0.3); 32];
        let mut b = a.clone();
        apply_autopan(&mut a, 32, 2.0, 0.6);
        apply_autopan(&mut b, 32, 2.0, 0.6);
        assert_eq!(a, b);
    }
}
