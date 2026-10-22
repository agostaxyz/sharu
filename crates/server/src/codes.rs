// SPDX-License-Identifier: AGPL-3.0-or-later
//! Session codes such as `Caesar-23-Pyramid-Orchid`: 2-4 capitalised words
//! plus one two-digit number placed after a word (never first).

use std::collections::BTreeSet;
use std::sync::OnceLock;

use rand::Rng;

const RAW: &str = include_str!("wordlist.txt");

pub fn words() -> &'static [&'static str] {
    static WORDS: OnceLock<Vec<&'static str>> = OnceLock::new();
    WORDS.get_or_init(|| {
        let set: BTreeSet<&'static str> = RAW
            .lines()
            .map(str::trim)
            .filter(|w| w.len() >= 4 && w.bytes().all(|b| b.is_ascii_lowercase()))
            .collect();
        set.into_iter().collect()
    })
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

pub fn generate_code<R: Rng + ?Sized>(rng: &mut R, n_words: usize) -> String {
    let n = n_words.clamp(2, 4);
    let list = words();
    let number_after = rng.random_range(1..=n);
    let mut parts: Vec<String> = Vec::with_capacity(n + 1);
    for i in 1..=n {
        parts.push(capitalize(list[rng.random_range(0..list.len())]));
        if i == number_after {
            parts.push(rng.random_range(10..=99u32).to_string());
        }
    }
    parts.join("-")
}

/// Approximate guessing entropy of a freshly generated code, in bits.
pub fn entropy_bits(n_words: usize) -> f64 {
    let n = n_words.clamp(2, 4) as f64;
    n * (words().len() as f64).log2() + 90f64.log2() + n.log2()
}

fn is_number(part: &str) -> bool {
    part.len() == 2 && part.bytes().all(|b| b.is_ascii_digit())
}

fn is_word(part: &str) -> bool {
    (4..=16).contains(&part.len()) && part.bytes().all(|b| b.is_ascii_alphabetic())
}

/// Validates the shape and returns the canonical (lower-case) lookup key.
/// Shared test vectors: `crates/proto/fixtures/code_vectors.json`.
pub fn normalize_code(input: &str) -> Option<String> {
    let parts: Vec<&str> = input.trim().split('-').collect();
    if !(3..=5).contains(&parts.len()) || is_number(parts[0]) {
        return None;
    }
    let mut numbers = 0;
    for part in &parts {
        if is_number(part) {
            numbers += 1;
        } else if !is_word(part) {
            return None;
        }
    }
    (numbers == 1).then(|| parts.join("-").to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn wordlist_is_clean() {
        let list = words();
        assert!(list.len() >= 400, "wordlist too small: {}", list.len());
        assert!(list.iter().all(|w| w.len() >= 4));
    }

    #[test]
    fn generated_codes_are_valid_and_varied() {
        let mut rng = rand::rng();
        let mut seen = BTreeSet::new();
        for n in 2..=4 {
            for _ in 0..200 {
                let code = generate_code(&mut rng, n);
                assert!(normalize_code(&code).is_some(), "invalid code {code}");
                seen.insert(code);
            }
        }
        assert!(seen.len() > 500);
    }

    #[test]
    fn number_never_comes_first() {
        let mut rng = rand::rng();
        for _ in 0..500 {
            let code = generate_code(&mut rng, 3);
            assert!(!code.chars().next().unwrap().is_ascii_digit());
        }
    }

    #[test]
    fn shared_vectors() {
        let v: Value = serde_json::from_str(include_str!(
            "../../proto/fixtures/code_vectors.json"
        ))
        .unwrap();
        for ok in v["valid"].as_array().unwrap() {
            assert!(normalize_code(ok.as_str().unwrap()).is_some(), "{ok}");
        }
        for bad in v["invalid"].as_array().unwrap() {
            assert!(normalize_code(bad.as_str().unwrap()).is_none(), "{bad}");
        }
    }

    #[test]
    fn lookup_is_case_insensitive() {
        assert_eq!(
            normalize_code("CAESAR-23-pyramid"),
            normalize_code("caesar-23-Pyramid")
        );
    }
}
