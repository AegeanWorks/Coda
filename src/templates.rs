//! Built-in starting points, stored as ordinary score source so the existing
//! parser and serializer are the only implementation of the format.

/// (name, source) pairs in listing order.
pub const TEMPLATES: [(&str, &str); 5] = [
    (
        "waltz",
        "tempo 120\nwave triangle\nnote C4 1\nat 1 E4 0.5\nat 1.5 G4 0.5\nat 2 C4 1\nat 3 G3 1\nat 4 E4 0.5\nat 4.5 C4 0.5\nat 5 G3 1\n",
    ),
    (
        "march",
        "tempo 120\nwave square\ngain 0.25\nnote C3 0.5\nnote G3 0.5\nnote C4 1\nnote G3 0.5\nnote E4 0.5\nnote G3 1\n",
    ),
    (
        "arp",
        "tempo 132\nwave saw\ngain 0.22\nnote C4 0.25\nnote E4 0.25\nnote G4 0.25\nnote B4 0.25\nnote C5 0.25\nnote B4 0.25\nnote G4 0.25\nnote E4 0.25\n",
    ),
    (
        "lullaby",
        "tempo 72\nwave sine\nattack 0.08\nrelease 0.4\nnote E4 1\nnote G4 0.5\nnote A4 0.5\nnote G4 1\nnote E4 1\nrest 1\n",
    ),
    (
        "drone",
        "tempo 60\nwave triangle\npreset pad\nnote C3 4\nat 0 G3 4\nat 0 C4 4\nat 2 E4 2\nat 4 C3 4\n",
    ),
];

pub fn names() -> Vec<&'static str> {
    TEMPLATES.iter().map(|(name, _)| *name).collect()
}

pub fn source(name: &str) -> Option<&'static str> {
    TEMPLATES
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map(|(_, source)| *source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score;

    #[test]
    fn names_are_stable() {
        assert_eq!(names(), vec!["waltz", "march", "arp", "lullaby", "drone"]);
        assert!(source("missing").is_none());
    }

    #[test]
    fn every_template_parses_and_roundtrips() {
        for name in names() {
            let parsed =
                score::parse(source(name).unwrap()).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(!parsed.notes.is_empty(), "{name}");
            assert_eq!(
                score::parse(&score::serialize(&parsed)).unwrap(),
                parsed,
                "{name}"
            );
        }
    }

    #[test]
    fn template_tempos_are_distinct_values() {
        let tempos: Vec<f64> = names()
            .iter()
            .map(|name| score::parse(source(name).unwrap()).unwrap().tempo)
            .collect();
        assert_eq!(tempos, vec![120.0, 120.0, 132.0, 72.0, 60.0]);
    }
}
