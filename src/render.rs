//! Mix notes into stereo f64 frames and encode 16-bit PCM WAV bytes.
//!
//! Panning is equal-power: `L = cos((pan+1)*pi/4)`, `R = sin((pan+1)*pi/4)`.

use crate::notes;
use crate::score::Score;

/// A stereo sample frame: `(left, right)`.
pub type Frame = (f64, f64);

/// Render a score into normalised frames in `[-1, 1]`.
pub fn render(score: &Score, sample_rate: u32) -> Vec<Frame> {
    let sr = sample_rate as f64;
    let beat = 60.0 / score.tempo;
    let tail = if score.notes.is_empty() {
        score.envelope.tail()
    } else {
        score
            .notes
            .iter()
            .map(|n| n.envelope.tail())
            .fold(0.0_f64, f64::max)
    };

    let mut end_beats: f64 = 0.0;
    for note in &score.notes {
        end_beats = end_beats.max(note.start + note.beats);
    }
    let total = ((end_beats * beat + tail) * sr).ceil() as usize;
    let mut buffer = vec![(0.0_f64, 0.0_f64); total];

    for (index, note) in score.notes.iter().enumerate() {
        let Some(freq) = notes::frequency(&note.pitch) else {
            continue;
        };
        let duration = note.beats * beat;
        let start = (note.start * beat * sr) as usize;
        let count = ((duration + tail) * sr) as usize;
        let seed = index as u64;
        let angle = (note.pan + 1.0) * std::f64::consts::FRAC_PI_4;
        let (left_gain, right_gain) = (angle.cos(), angle.sin());

        for i in 0..count {
            let at = start + i;
            if at >= buffer.len() {
                break;
            }
            let t = i as f64 / sr;
            let amp = note.envelope.amplitude(t, duration);
            if amp == 0.0 {
                continue;
            }
            let detune =
                note.vibrato.cents * (std::f64::consts::TAU * note.vibrato.rate * t).sin() / 1200.0;
            let phase = (freq * t * 2.0_f64.powf(detune)) % 1.0;
            let value = score.wave.sample(phase, seed) * amp * note.velocity;
            buffer[at].0 += value * left_gain;
            buffer[at].1 += value * right_gain;
        }
    }

    let gain = score.gain.clamp(0.0, 1.0);
    for frame in &mut buffer {
        frame.0 = (frame.0 * gain).tanh();
        frame.1 = (frame.1 * gain).tanh();
    }
    buffer
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_i16(out: &mut Vec<u8>, value: i16) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Encode frames as stereo 16-bit PCM WAV (44-byte RIFF header + data).
pub fn wav_bytes(frames: &[Frame], sample_rate: u32) -> Vec<u8> {
    let data_len = (frames.len() * 4) as u32;
    let mut out = Vec::with_capacity(44 + frames.len() * 4);

    out.extend_from_slice(b"RIFF");
    push_u32(&mut out, 36 + data_len);
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    push_u32(&mut out, 16); // fmt chunk size
    push_u16(&mut out, 1); // PCM
    push_u16(&mut out, 2); // stereo
    push_u32(&mut out, sample_rate);
    push_u32(&mut out, sample_rate * 4); // byte rate
    push_u16(&mut out, 4); // block align
    push_u16(&mut out, 16); // bits per sample
    out.extend_from_slice(b"data");
    push_u32(&mut out, data_len);

    for &(left, right) in frames {
        push_i16(&mut out, (left.clamp(-1.0, 1.0) * 32767.0).round() as i16);
        push_i16(&mut out, (right.clamp(-1.0, 1.0) * 32767.0).round() as i16);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    #[test]
    fn wav_header_is_well_formed() {
        let frames = vec![(0.0, 0.0), (0.5, -0.5), (1.0, -1.0)];
        let wav = wav_bytes(&frames, 44100);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(wav.len(), 44 + 12);
        assert_eq!(u16::from_le_bytes([wav[22], wav[23]]), 2);
        assert_eq!(u16::from_le_bytes([wav[32], wav[33]]), 4);
        assert_eq!(u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]), 12);
        assert_eq!(
            u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]),
            44100
        );
    }

    #[test]
    fn render_is_deterministic_and_wave_sensitive() {
        let a = score::parse("tempo 120\nwave sine\nnote A4 1\n").unwrap();
        let b = score::parse("tempo 120\nwave sine\nnote A4 1\n").unwrap();
        let c = score::parse("tempo 120\nwave saw\nnote A4 1\n").unwrap();
        let wa = wav_bytes(&render(&a, 8000), 8000);
        let wb = wav_bytes(&render(&b, 8000), 8000);
        let wc = wav_bytes(&render(&c, 8000), 8000);
        assert_eq!(wa, wb);
        assert_ne!(wa, wc);
    }

    #[test]
    fn pan_changes_channel_balance() {
        let left = score::parse("wave sine\npan -1\nnote A4 1\n").unwrap();
        let right = score::parse("wave sine\npan 1\nnote A4 1\n").unwrap();
        let l = render(&left, 8000);
        let r = render(&right, 8000);
        assert!(l.iter().map(|f| f.0.abs()).fold(0.0, f64::max) > 0.0);
        assert!(l.iter().all(|f| f.1.abs() < 1e-12));
        assert!(r.iter().all(|f| f.0.abs() < 1e-12));
        assert!(r.iter().map(|f| f.1.abs()).fold(0.0, f64::max) > 0.0);
    }

    #[test]
    fn silence_is_zero() {
        let empty = score::parse("tempo 120\n").unwrap();
        let frames = render(&empty, 8000);
        assert!(frames.iter().all(|f| f.0 == 0.0 && f.1 == 0.0));
    }
}
