//! Seeded, bounded timing and velocity jitter for a more natural performance.

use crate::score::Score;

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

const MIX: u64 = 0x9e37_79b9_7f4a_7c15;

pub fn humanize(score: &Score, seed: u64, amount: f64) -> Result<Score, String> {
    if !amount.is_finite() || !(0.0..=1.0).contains(&amount) {
        return Err("amount must be between 0 and 1".into());
    }
    let mut result = score.clone();
    if amount == 0.0 {
        return Ok(result);
    }

    for (index, note) in result.notes.iter_mut().enumerate() {
        // Rests always sit at the running cursor in the score format, so their
        // start cannot be serialized independently. Leave them untouched.
        if note.pitch == "-" {
            continue;
        }
        let mut rng = Lcg(seed ^ (index as u64).wrapping_mul(MIX));
        let timing = ((rng.next() % 201) as f64 / 100.0 - 1.0) * 0.06 * amount;
        let velocity = ((rng.next() % 201) as f64 / 100.0 - 1.0) * 0.15 * amount;
        note.start = (note.start + timing).max(0.0);
        note.velocity = (note.velocity + velocity).clamp(0.0, 1.0);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    fn sample() -> Score {
        score::parse("tempo 120\nnote C4 1 0.8\nnote E4 0.5 0.7\nrest 1\n").unwrap()
    }

    #[test]
    fn zero_amount_is_an_identity() {
        assert_eq!(humanize(&sample(), 42, 0.0).unwrap(), sample());
    }

    #[test]
    fn same_seed_is_deterministic() {
        assert_eq!(
            humanize(&sample(), 42, 0.6).unwrap(),
            humanize(&sample(), 42, 0.6).unwrap()
        );
    }

    #[test]
    fn different_seeds_differ() {
        assert_ne!(
            humanize(&sample(), 42, 0.6).unwrap(),
            humanize(&sample(), 43, 0.6).unwrap()
        );
    }

    #[test]
    fn stays_in_bounds_and_roundtrips() {
        let result = humanize(&sample(), 7, 1.0).unwrap();
        assert!(result.notes.iter().all(|n| n.start >= 0.0));
        assert!(result
            .notes
            .iter()
            .all(|n| (0.0..=1.0).contains(&n.velocity)));
        assert_eq!(score::parse(&score::serialize(&result)).unwrap(), result);
    }

    #[test]
    fn rejects_bad_amounts() {
        assert!(humanize(&sample(), 1, -0.1).is_err());
        assert!(humanize(&sample(), 1, 1.5).is_err());
    }
}
