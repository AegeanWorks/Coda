//! A simple bounded feedback delay (comb echo) over stereo frames.

use crate::render::Frame;

/// Add a delayed copy of the signal back into itself.
///
/// `feedback` is clamped below 1.0 so the recursion cannot grow without bound;
/// a zero or negative delay leaves the buffer unchanged.
pub fn apply_echo(frames: &mut [Frame], sample_rate: u32, delay: f64, feedback: f64) {
    if delay <= 0.0 || !delay.is_finite() {
        return;
    }
    let delay_samples = (delay * sample_rate as f64).round() as usize;
    if delay_samples == 0 || delay_samples >= frames.len() {
        return;
    }
    let feedback = feedback.clamp(0.0, 0.95);
    if feedback == 0.0 {
        return;
    }

    for index in delay_samples..frames.len() {
        let previous = frames[index - delay_samples];
        frames[index].0 = (frames[index].0 + previous.0 * feedback).clamp(-1.0, 1.0);
        frames[index].1 = (frames[index].1 + previous.1 * feedback).clamp(-1.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_delay_is_an_identity() {
        let mut frames = vec![(0.5, -0.5); 8];
        let before = frames.clone();
        apply_echo(&mut frames, 8, 0.0, 0.5);
        assert_eq!(frames, before);
    }

    #[test]
    fn impulse_creates_a_delayed_tail() {
        let mut frames = vec![(0.0, 0.0); 10];
        frames[0] = (1.0, 0.5);
        apply_echo(&mut frames, 10, 0.3, 0.5); // 3 samples delay
        assert_eq!(frames[0], (1.0, 0.5));
        assert_eq!(frames[1], (0.0, 0.0));
        assert_eq!(frames[2], (0.0, 0.0));
        assert!((frames[3].0 - 0.5).abs() < 1e-12);
        assert!((frames[3].1 - 0.25).abs() < 1e-12);
        assert!((frames[6].0 - 0.25).abs() < 1e-12);
    }

    #[test]
    fn feedback_is_clamped() {
        let mut frames = vec![(0.0, 0.0); 20];
        frames[0] = (1.0, 1.0);
        apply_echo(&mut frames, 10, 0.1, 99.0);
        assert!(frames.iter().all(|f| f.0.abs() <= 1.0 && f.1.abs() <= 1.0));
    }

    #[test]
    fn long_delay_does_nothing() {
        let mut frames = vec![(0.5, 0.5); 4];
        let before = frames.clone();
        apply_echo(&mut frames, 8, 5.0, 0.5); // 40 samples > buffer
        assert_eq!(frames, before);
    }
}
