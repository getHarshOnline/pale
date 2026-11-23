//! Compares hook matching with an independent finite-language reference implementation.

use super::star_pattern_matches;

fn reference(pattern: &[char], value: &[char]) -> bool {
    let mut reachable = vec![false; value.len() + 1];
    reachable[0] = true;
    for token in pattern {
        let mut next = vec![false; value.len() + 1];
        if *token == '*' {
            let mut prefix_reachable = false;
            for (index, slot) in next.iter_mut().enumerate() {
                prefix_reachable |= reachable[index];
                *slot = prefix_reachable;
            }
        } else {
            for index in 0..value.len() {
                next[index + 1] = reachable[index] && *token == value[index];
            }
        }
        reachable = next;
    }
    reachable[value.len()]
}

fn words(alphabet: &[char], max_len: usize) -> Vec<String> {
    let mut words = vec![String::new()];
    for _ in 0..max_len {
        let prior = words.clone();
        words.extend(prior.iter().flat_map(|prefix| {
            alphabet.iter().map(move |character| {
                let mut word = prefix.clone();
                word.push(*character);
                word
            })
        }));
    }
    words.sort();
    words.dedup();
    words
}

#[test]
fn star_language_matches_an_independent_reference_exhaustively() {
    let patterns = words(&['a', 'b', '*'], 4);
    let values = words(&['a', 'b'], 4);
    for pattern in &patterns {
        for value in &values {
            assert_eq!(
                star_pattern_matches(pattern, value),
                reference(
                    &pattern.chars().collect::<Vec<_>>(),
                    &value.chars().collect::<Vec<_>>()
                ),
                "pattern={pattern:?}, value={value:?}"
            );
        }
    }
}

#[test]
fn non_star_metacharacters_are_literal_and_unicode_is_scalar_safe() {
    for literal in ["?", "[a]", "{a,b}", r"a\\b", "雪"] {
        assert!(star_pattern_matches(literal, literal));
        assert!(!star_pattern_matches(literal, "different"));
    }
    assert!(star_pattern_matches("*雪*", "前雪後"));
    assert!(star_pattern_matches("a**b", "ab"));
}

#[test]
fn repeated_stars_and_long_unicode_inputs_remain_iterative() {
    let pattern = format!("{}雪", "*".repeat(4_089));
    let value = format!("{}雪", "前".repeat(8_192));
    assert!(star_pattern_matches(&pattern, &value));
    assert!(!star_pattern_matches(&pattern, &"前".repeat(8_192)));
}
