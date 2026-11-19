//! Seeded, bounded variation of an existing score.
//!
//! Variation is a pure function of `(score, seed, amount)`. `amount = 0` is an
//! exact identity; larger amounts make stronger but still bounded changes.

use crate::notes;
use crate::score::Score;
use crate::wave::Wave;

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
}

const INTERVALS: [i32; 8] = [0, 2, 3, 5, 7, -2, -3, -5];
const MIX: u64 = 0x9e37_79b9_7f4a_7c15;

pub fn vary(score: &Score, seed: u64, amount: f64) -> Score {
    let amount = amount.clamp(0.0, 1.0);
    let mut result = score.clone();
    if amount == 0.0 {
        return result;
    }

    let mut global = Lcg(seed);
    let key_shift =
        (INTERVALS[(global.next() as usize) % INTERVALS.len()] as f64 * amount).round() as i32;
    let note_chance = (15.0 * amount) as u64;
    let wave_chance = (30.0 * amount) as u64;

    for (index, note) in result.notes.iter_mut().enumerate() {
        if note.pitch == "-" {
            continue;
        }
        let mut local = Lcg(seed ^ (index as u64).wrapping_mul(MIX));

        if let Some(number) = notes::midi(&note.pitch) {
            let ornament = if local.next() % 100 < note_chance {
                (local.next() % 3) as i32 - 1
            } else {
                0
            };
            note.pitch = notes::name(number + key_shift + ornament);
        }

        let velocity_delta = ((local.next() % 21) as f64 / 100.0 - 0.1) * amount;
        note.velocity = (note.velocity + velocity_delta).clamp(0.0, 1.0);

        let pan_delta = ((local.next() % 61) as f64 / 100.0 - 0.3) * amount;
        note.pan = (note.pan + pan_delta).clamp(-1.0, 1.0);

        if local.next() % 100 < wave_chance {
            note.wave = Wave::ALL[(local.next() as usize) % Wave::ALL.len()];
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::{parse, serialize};

    fn sample() -> Score {
        parse("tempo 120\nwave sine\nnote C4 1 0.8\nat 2 E4 0.5 0.6\nrest 1\n").unwrap()
    }

    #[test]
    fn zero_amount_is_an_identity() {
        assert_eq!(vary(&sample(), 42, 0.0), sample());
        assert_eq!(serialize(&vary(&sample(), 42, 0.0)), serialize(&sample()));
    }

    #[test]
    fn same_seed_is_deterministic() {
        assert_eq!(vary(&sample(), 42, 0.7), vary(&sample(), 42, 0.7));
        assert_eq!(
            serialize(&vary(&sample(), 42, 0.7)),
            serialize(&vary(&sample(), 42, 0.7))
        );
    }

    #[test]
    fn different_seeds_differ() {
        assert_ne!(
            serialize(&vary(&sample(), 42, 0.7)),
            serialize(&vary(&sample(), 43, 0.7))
        );
    }

    #[test]
    fn varied_fields_stay_valid() {
        let varied = vary(&sample(), 7, 1.0);
        assert!(varied
            .notes
            .iter()
            .all(|n| n.pitch == "-" || notes::frequency(&n.pitch).is_some()));
        assert!(varied
            .notes
            .iter()
            .all(|n| (0.0..=1.0).contains(&n.velocity)));
        assert!(varied.notes.iter().all(|n| (-1.0..=1.0).contains(&n.pan)));
        assert_eq!(parse(&serialize(&varied)).unwrap(), varied);
    }

    #[test]
    fn rests_are_untouched() {
        let original = sample();
        let varied = vary(&original, 9, 1.0);
        assert_eq!(varied.notes[2], original.notes[2]);
    }
}
