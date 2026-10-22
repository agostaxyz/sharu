// SPDX-License-Identifier: AGPL-3.0-or-later
//! Default viewer nicknames: two CamelCase words, no digits (e.g. `LemonOtter`).

use rand::Rng;

const FRUITS: &[&str] = &[
    "Almond",
    "Apple",
    "Apricot",
    "Avocado",
    "Banana",
    "Berry",
    "Blueberry",
    "Cashew",
    "Cherry",
    "Coconut",
    "Cranberry",
    "Currant",
    "Date",
    "Fig",
    "Grape",
    "Guava",
    "Hazel",
    "Kiwi",
    "Lemon",
    "Lime",
    "Lychee",
    "Mango",
    "Melon",
    "Mulberry",
    "Olive",
    "Orange",
    "Papaya",
    "Peach",
    "Pear",
    "Pineapple",
    "Plum",
    "Pomelo",
    "Quince",
    "Raspberry",
    "Tangerine",
];

const ANIMALS: &[&str] = &[
    "Badger", "Beaver", "Bison", "Condor", "Crane", "Dolphin", "Eagle", "Falcon", "Ferret",
    "Finch", "Fox", "Gecko", "Heron", "Ibis", "Jaguar", "Koala", "Lemur", "Lynx", "Magpie",
    "Marmot", "Newt", "Ocelot", "Okapi", "Otter", "Owl", "Panda", "Parrot", "Pelican", "Puffin",
    "Quail", "Raven", "Robin", "Seal", "Sparrow", "Swan", "Tapir", "Tiger", "Toucan", "Turtle",
    "Walrus", "Wombat", "Yak", "Zebra",
];

pub fn default_nickname<R: Rng + ?Sized>(rng: &mut R) -> String {
    format!(
        "{}{}",
        FRUITS[rng.random_range(0..FRUITS.len())],
        ANIMALS[rng.random_range(0..ANIMALS.len())]
    )
}

/// Accepts 2-24 letters/digits/`_`/`-` (Unicode letters allowed). Letters-only
/// means no whitespace, control or bidi-override characters can slip in.
pub fn sanitize(input: &str) -> Option<String> {
    let s = input.trim();
    let n = s.chars().count();
    if !(2..=24).contains(&n) {
        return None;
    }
    s.chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        .then(|| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_two_camel_words_without_digits() {
        let mut rng = rand::rng();
        for _ in 0..200 {
            let n = default_nickname(&mut rng);
            assert!(sanitize(&n).is_some());
            assert!(n.chars().all(|c| c.is_ascii_alphabetic()));
            assert_eq!(n.chars().filter(|c| c.is_ascii_uppercase()).count(), 2);
        }
    }

    #[test]
    fn sanitize_rejects_bad_input() {
        assert!(sanitize("a").is_none());
        assert!(sanitize("has space").is_none());
        assert!(sanitize("evil\u{202e}name").is_none());
        assert!(sanitize("<script>").is_none());
        assert!(sanitize(&"x".repeat(25)).is_none());
        assert_eq!(sanitize("  Zoë_1 ").as_deref(), Some("Zoë_1"));
    }
}
