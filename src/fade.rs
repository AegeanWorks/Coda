//! Linear fade-in and fade-out over rendered stereo frames.

use crate::render::Frame;

/// Apply fades in place. Fades are clamped to at most half the buffer so the
/// two envelopes never overlap and the centre of the piece stays untouched.
pub fn apply_fades(frames: &mut [Frame], sample_rate: u32, fade_in: f64, fade_out: f64) {
    if frames.is_empty() {
        return;
    }
    let seconds = frames.len() as f64 / sample_rate as f64;
    let limit = seconds / 2.0;
    let in_samples = (fade_in.max(0.0).min(limit) * sample_rate as f64).round() as usize;
    let out_samples = (fade_out.max(0.0).min(limit) * sample_rate as f64).round() as usize;
    let last = frames.len() - 1;

    for (index, frame) in frames.iter_mut().enumerate() {
        let in_gain = if index < in_samples {
            index as f64 / in_samples as f64
        } else {
            1.0
        };
        let out_gain = if index > last.saturating_sub(out_samples) {
            (last - index) as f64 / out_samples as f64
        } else {
            1.0
        };
        let gain = in_gain.min(out_gain);
        frame.0 *= gain;
        frame.1 *= gain;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_fades_change_nothing() {
        let mut frames = vec![(0.5, -0.5); 8];
        let before = frames.clone();
        apply_fades(&mut frames, 8, 0.0, 0.0);
        assert_eq!(frames, before);
    }

    #[test]
    fn linear_envelopes() {
        let mut frames = vec![(1.0, 1.0); 10];
        apply_fades(&mut frames, 10, 0.2, 0.2); // 2 samples each side
        assert_eq!(frames[0], (0.0, 0.0));
        assert_eq!(frames[1], (0.5, 0.5));
        assert_eq!(frames[2], (1.0, 1.0));
        assert_eq!(frames[7], (1.0, 1.0));
        assert_eq!(frames[8], (0.5, 0.5));
        assert_eq!(frames[9], (0.0, 0.0));
    }

    #[test]
    fn long_fades_are_clamped_to_half() {
        let mut frames = vec![(1.0, 1.0); 10];
        apply_fades(&mut frames, 10, 99.0, 99.0);
        assert_eq!(frames[4], (0.8, 0.8));
        assert_eq!(frames[5], (0.8, 0.8));
        assert_eq!(frames[9], (0.0, 0.0));
    }

    #[test]
    fn empty_buffer_is_safe() {
        let mut frames: Vec<Frame> = Vec::new();
        apply_fades(&mut frames, 8_000, 1.0, 1.0);
        assert!(frames.is_empty());
    }
}
