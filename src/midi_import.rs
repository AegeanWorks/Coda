//! Import the single-track, type-0 MIDI subset that Coda exports.
//!
//! MIDI carries no waveform, pan or envelope information, so imported notes
//! use the score defaults for those fields. Tempo, pitches, durations and
//! velocities are recovered from meta and note events.

use crate::envelope::Envelope;
use crate::notes;
use crate::score::{Note, Score, Vibrato};
use crate::wave::Wave;
use std::collections::BTreeMap;

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Reader { data, pos: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self
            .pos
            .checked_add(count)
            .ok_or_else(|| "MIDI offset overflow".to_string())?;
        let slice = self
            .data
            .get(self.pos..end)
            .ok_or_else(|| "truncated MIDI data".to_string())?;
        self.pos = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    fn u16be(&mut self) -> Result<u16, String> {
        let bytes = self.take(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn u32be(&mut self) -> Result<u32, String> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn vlq(&mut self) -> Result<u32, String> {
        let mut value = 0_u32;
        for _ in 0..4 {
            let byte = self.u8()?;
            value = value
                .checked_shl(7)
                .ok_or_else(|| "MIDI delta time overflow".to_string())?
                | (byte & 0x7f) as u32;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err("invalid MIDI variable-length quantity".into())
    }
}

struct TrackData {
    notes: Vec<Note>,
    tempo: Option<f64>,
}

fn parse_track(data: &[u8], ticks_per_quarter: u16) -> Result<TrackData, String> {
    let mut reader = Reader::new(data);
    let mut running: Option<u8> = None;
    let mut tick = 0_u32;
    let mut tempo: Option<f64> = None;
    let mut open: BTreeMap<(u8, u8), (u32, u8)> = BTreeMap::new();
    let mut notes = Vec::new();

    while reader.pos < data.len() {
        tick = tick
            .checked_add(reader.vlq()?)
            .ok_or_else(|| "MIDI tick overflow".to_string())?;
        let mut status = reader.u8()?;
        if status & 0x80 == 0 {
            // Running status: the byte just read is the first data byte.
            status =
                running.ok_or_else(|| "running status without a prior status byte".to_string())?;
            reader.pos -= 1;
        } else {
            running = Some(status);
        }

        match status & 0xf0 {
            0x80 => {
                let channel = status & 0x0f;
                let pitch = reader.u8()?;
                let _velocity = reader.u8()?;
                if let Some((start, velocity)) = open.remove(&(channel, pitch)) {
                    notes.push(make_note(pitch, start, tick, velocity, ticks_per_quarter));
                }
            }
            0x90 => {
                let channel = status & 0x0f;
                let pitch = reader.u8()?;
                let velocity = reader.u8()?;
                if velocity == 0 {
                    if let Some((start, vel)) = open.remove(&(channel, pitch)) {
                        notes.push(make_note(pitch, start, tick, vel, ticks_per_quarter));
                    }
                } else {
                    open.insert((channel, pitch), (tick, velocity));
                }
            }
            0xa0 | 0xb0 | 0xe0 => {
                reader.take(2)?;
            }
            0xc0 | 0xd0 => {
                reader.take(1)?;
            }
            0xf0 => {
                let kind = reader.u8()?;
                let length = reader.vlq()? as usize;
                let payload = reader.take(length)?;
                if kind == 0x51 && payload.len() == 3 {
                    let micros = ((payload[0] as u32) << 16)
                        | ((payload[1] as u32) << 8)
                        | payload[2] as u32;
                    if micros > 0 {
                        tempo = Some(60_000_000.0 / micros as f64);
                    }
                }
                if kind == 0x2f {
                    break;
                }
            }
            _ => return Err(format!("unsupported MIDI status {status:#04x}")),
        }
    }

    Ok(TrackData { notes, tempo })
}

fn make_note(pitch: u8, start: u32, end: u32, velocity: u8, ticks_per_quarter: u16) -> Note {
    let scale = ticks_per_quarter as f64;
    Note {
        pitch: notes::name(pitch as i32),
        start: start as f64 / scale,
        beats: (end.saturating_sub(start)) as f64 / scale,
        velocity: ((velocity.saturating_sub(1)) as f64 / 126.0).clamp(0.0, 1.0),
        pan: 0.0,
        envelope: Envelope::default(),
        vibrato: Vibrato {
            rate: 0.0,
            cents: 0.0,
        },
        wave: Wave::Triangle,
    }
}

pub fn import_midi(bytes: &[u8]) -> Result<Score, String> {
    let mut reader = Reader::new(bytes);
    if reader.take(4)? != b"MThd" {
        return Err("not a Standard MIDI File: missing MThd".into());
    }
    let header_length = reader.u32be()?;
    if header_length < 6 {
        return Err("invalid MIDI header length".into());
    }
    let format = reader.u16be()?;
    if format > 1 {
        return Err(format!(
            "only format 0 and 1 MIDI is supported, got {format}"
        ));
    }
    let declared_tracks = reader.u16be()?;
    if declared_tracks == 0 {
        return Err("MIDI file declares no tracks".into());
    }
    let ticks_per_quarter = reader.u16be()?;
    if ticks_per_quarter == 0 {
        return Err("MIDI division must be positive".into());
    }
    reader.take(header_length as usize - 6)?;

    let mut notes = Vec::new();
    let mut tempo = None;
    let mut found_tracks = 0_u16;
    while reader.pos < bytes.len() {
        let id = reader.take(4)?;
        let length = reader.u32be()? as usize;
        let chunk = reader.take(length)?;
        if id == b"MTrk" {
            found_tracks += 1;
            let track = parse_track(chunk, ticks_per_quarter)?;
            if tempo.is_none() {
                tempo = track.tempo;
            }
            notes.extend(track.notes);
        }
    }
    if found_tracks == 0 {
        return Err("MIDI file has no MTrk chunk".into());
    }
    if found_tracks != declared_tracks {
        return Err(format!(
            "MIDI header declares {declared_tracks} tracks but {found_tracks} were found"
        ));
    }

    notes.sort_by_key(|note| {
        (
            (note.start * ticks_per_quarter as f64).round() as i64,
            note.pitch.clone(),
        )
    });
    Ok(Score {
        tempo: tempo.unwrap_or(120.0),
        wave: Wave::Triangle,
        gain: 0.3,
        envelope: Envelope::default(),
        vibrato: Vibrato {
            rate: 0.0,
            cents: 0.0,
        },
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi::midi_bytes;
    use crate::score;

    #[test]
    fn rejects_non_midi_bytes() {
        assert!(import_midi(b"RIFFnotmidi").is_err());
        assert!(import_midi(b"MThd").is_err());
    }

    #[test]
    fn rejects_unsupported_formats() {
        let bytes = midi_bytes(&score::parse("note C4 1\n").unwrap());
        let mut modified = bytes.clone();
        modified[9] = 2; // high byte of the format field
        assert!(import_midi(&modified)
            .unwrap_err()
            .contains("format 0 and 1"));
    }

    fn format_one_file(tracks: &[Vec<u8>]) -> Vec<u8> {
        let mut bytes: Vec<u8> = vec![b'M', b'T', b'h', b'd', 0, 0, 0, 6, 0, 1];
        bytes.extend_from_slice(&(tracks.len() as u16).to_be_bytes());
        bytes.extend_from_slice(&480_u16.to_be_bytes());
        for track in tracks {
            bytes.extend_from_slice(b"MTrk");
            bytes.extend_from_slice(&(track.len() as u32).to_be_bytes());
            bytes.extend_from_slice(track);
        }
        bytes
    }

    #[test]
    fn merges_format_one_tracks() {
        let conductor: Vec<u8> = vec![
            0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20, // tempo 120
            0x00, 0xff, 0x2f, 0x00,
        ];
        let first: Vec<u8> = vec![
            0x00, 0x90, 60, 100, // C4 on at tick 0
            0x83, 0x60, 0x80, 60, 0, // C4 off at tick 480
            0x00, 0xff, 0x2f, 0x00,
        ];
        let second: Vec<u8> = vec![
            0x83, 0x60, 0x90, 64, 100, // E4 on at tick 480
            0x83, 0x60, 0x80, 64, 0, // E4 off at tick 960
            0x00, 0xff, 0x2f, 0x00,
        ];

        let imported = import_midi(&format_one_file(&[conductor, first, second])).unwrap();
        assert_eq!(imported.tempo, 120.0);
        assert_eq!(imported.notes.len(), 2);
        assert_eq!(imported.notes[0].pitch, "C4");
        assert_eq!(imported.notes[0].start, 0.0);
        assert_eq!(imported.notes[0].beats, 1.0);
        assert_eq!(imported.notes[1].pitch, "E4");
        assert_eq!(imported.notes[1].start, 1.0);
        assert_eq!(imported.notes[1].beats, 1.0);
        assert_eq!(
            score::parse(&score::serialize(&imported)).unwrap(),
            imported
        );
    }

    #[test]
    fn rejects_track_count_mismatch() {
        let track: Vec<u8> = vec![0x00, 0xff, 0x2f, 0x00];
        let bytes = format_one_file(&[track]);
        let mut modified = bytes.clone();
        modified[10] = 0; // declared tracks high byte -> 0, low byte stays 1; change low below
        modified[11] = 2; // declare two tracks, one chunk present
        assert!(import_midi(&modified).unwrap_err().contains("declares"));
    }

    #[test]
    fn roundtrips_a_simple_score() {
        let source = score::parse("tempo 120\nnote C4 1 1.0\nat 2 G4 0.5 0.5\n").unwrap();
        let imported = import_midi(&midi_bytes(&source)).unwrap();
        assert_eq!(imported.tempo, 120.0);
        assert_eq!(imported.notes.len(), 2);
        assert_eq!(imported.notes[0].pitch, "C4");
        assert_eq!(imported.notes[0].start, 0.0);
        assert_eq!(imported.notes[0].beats, 1.0);
        assert_eq!(imported.notes[0].velocity, 1.0);
        assert_eq!(imported.notes[1].pitch, "G4");
        assert_eq!(imported.notes[1].start, 2.0);
        assert_eq!(imported.notes[1].beats, 0.5);
        assert_eq!(imported.notes[1].velocity, 0.5);
    }

    #[test]
    fn imported_score_renders_to_identical_bytes() {
        // MIDI carries no waveform, gain or envelope, so use score defaults.
        let source = score::parse("tempo 120\nnote C4 1\nat 2 G4 0.5\n").unwrap();
        let imported = import_midi(&midi_bytes(&source)).unwrap();
        assert_eq!(
            crate::render::wav_bytes(&crate::render::render(&imported, 8_000), 8_000),
            crate::render::wav_bytes(&crate::render::render(&source, 8_000), 8_000)
        );
    }

    #[test]
    fn handles_running_status() {
        let track: Vec<u8> = vec![
            0x00, 0x90, 60, 100, // note on C4 at tick 0
            0x01, 62, 100, // running status: note on D4 at tick 1
            0x01, 0x80, 60, 0, // note off C4 at tick 2
            0x01, 62, 0, // running status: note off D4 at tick 3
            0x00, 0xff, 0x2f, 0x00, // end of track
        ];
        let mut bytes: Vec<u8> = vec![
            b'M', b'T', b'h', b'd', 0, 0, 0, 6, 0, 0, 0, 1, 1, 0xe0, b'M', b'T', b'r', b'k',
        ];
        bytes.extend_from_slice(&(track.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&track);

        let score = import_midi(&bytes).unwrap();
        assert_eq!(score.notes.len(), 2);
        assert_eq!(score.notes[0].pitch, "C4");
        assert_eq!(score.notes[1].pitch, "D4");
    }
}
