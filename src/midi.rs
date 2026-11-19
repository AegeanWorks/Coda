//! Dependency-free Standard MIDI File (type 0, single track) writer.
//!
//! Symbolic output sits next to the audio renderer: the same `Score` can be
//! compiled to WAV or to MIDI with no external crate and no clock.

use crate::notes;
use crate::score::Score;

/// MIDI ticks per quarter note.
pub const TICKS_PER_QUARTER: u16 = 480;

fn push_vlq(out: &mut Vec<u8>, value: u32) {
    let mut buffer = [0_u8; 4];
    let mut index = 3;
    let mut rest = value;
    buffer[index] = (rest & 0x7f) as u8;
    rest >>= 7;
    while rest > 0 {
        index -= 1;
        buffer[index] = ((rest & 0x7f) as u8) | 0x80;
        rest >>= 7;
    }
    out.extend_from_slice(&buffer[index..]);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Event {
    tick: u32,
    order: u8,
    bytes: [u8; 3],
}

/// Encode a score as a type-0 MIDI file.
pub fn midi_bytes(score: &Score) -> Vec<u8> {
    let ticks = TICKS_PER_QUARTER as f64;
    let mut events: Vec<Event> = Vec::new();

    let micros = (60_000_000.0 / score.tempo).round().max(1.0) as u32;

    let mut end_tick = 0_u32;
    for note in &score.notes {
        let Some(number) = notes::midi(&note.pitch) else {
            continue;
        };
        if !(0..=127).contains(&number) {
            continue;
        }
        let start = (note.start * ticks).round().max(0.0) as u32;
        let end = ((note.start + note.beats) * ticks)
            .round()
            .max(start as f64) as u32;
        end_tick = end_tick.max(end);
        let velocity = (note.velocity.clamp(0.0, 1.0) * 126.0).round() as u8 + 1;
        events.push(Event {
            tick: start,
            order: 1,
            bytes: [0x90, number as u8, velocity],
        });
        events.push(Event {
            tick: end,
            order: 2,
            bytes: [0x80, number as u8, 0],
        });
    }

    events.sort_by_key(|e| (e.tick, e.order, e.bytes));

    let mut track = Vec::new();
    push_vlq(&mut track, 0);
    track.extend_from_slice(&[0xff, 0x51, 0x03]);
    track.extend_from_slice(&micros.to_be_bytes()[1..]);

    let mut previous = 0_u32;
    for event in &events {
        push_vlq(&mut track, event.tick - previous);
        track.extend_from_slice(&event.bytes);
        previous = event.tick;
    }
    push_vlq(&mut track, end_tick - previous);
    track.extend_from_slice(&[0xff, 0x2f, 0x00]);

    let mut out = Vec::with_capacity(14 + 8 + track.len());
    out.extend_from_slice(b"MThd");
    out.extend_from_slice(&6_u32.to_be_bytes());
    out.extend_from_slice(&0_u16.to_be_bytes()); // format 0
    out.extend_from_slice(&1_u16.to_be_bytes()); // one track
    out.extend_from_slice(&TICKS_PER_QUARTER.to_be_bytes());
    out.extend_from_slice(b"MTrk");
    out.extend_from_slice(&(track.len() as u32).to_be_bytes());
    out.extend_from_slice(&track);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    #[test]
    fn variable_length_quantities() {
        let mut out = Vec::new();
        push_vlq(&mut out, 0);
        push_vlq(&mut out, 127);
        push_vlq(&mut out, 128);
        push_vlq(&mut out, 8_192);
        assert_eq!(out, vec![0x00, 0x7f, 0x81, 0x00, 0xc0, 0x00]);
    }

    #[test]
    fn header_and_tempo_are_correct() {
        let midi = midi_bytes(&score::parse("tempo 120\n").unwrap());
        assert_eq!(
            &midi[0..14],
            b"MThd\x00\x00\x00\x06\x00\x00\x00\x01\x01\xe0"
        );
        assert_eq!(&midi[14..18], b"MTrk");
        assert!(midi
            .windows(6)
            .any(|w| w == [0x00, 0xff, 0x51, 0x03, 0x07, 0xa1]));
        assert!(midi.ends_with(&[0x00, 0xff, 0x2f, 0x00]));
    }

    #[test]
    fn note_on_and_off_are_emitted() {
        let midi = midi_bytes(&score::parse("tempo 120\nnote C4 1 0.5\n").unwrap());
        assert!(midi.windows(4).any(|w| w == [0x00, 0x90, 60, 64]));
        assert!(midi.windows(3).any(|w| w == [0x80, 60, 0x00]));
    }

    #[test]
    fn rests_are_ignored_but_still_move_the_clock() {
        let plain = midi_bytes(&score::parse("note C4 1\n").unwrap());
        let rested = midi_bytes(&score::parse("rest 1\nnote C4 1\n").unwrap());
        assert_ne!(plain, rested);
    }

    #[test]
    fn output_is_deterministic() {
        let a = score::parse("tempo 96\nwave saw\nnote C4 1\nat 2 E4 1\n").unwrap();
        assert_eq!(midi_bytes(&a), midi_bytes(&a));
    }
}
