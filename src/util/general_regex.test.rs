use super::{GenRegExp, Pattern};

fn symbol(bits: &str) -> Vec<u64> {
    let mut symbol = vec![0];
    for bit in bits.bytes() {
        assert!(bit.is_ascii_lowercase());
        symbol[0] |= 1 << (bit - b'a');
    }
    symbol
}

fn word(input: &str) -> Vec<Vec<u64>> {
    input.split_whitespace().map(symbol).collect()
}

fn check(pattern: Pattern, matching: &[&str], rejected: &[&str]) {
    let mut regexp = GenRegExp::new(pattern);
    for input in matching {
        assert!(regexp.matches(&word(input)), "{input:?} should match");
    }
    for input in rejected {
        assert!(!regexp.matches(&word(input)), "{input:?} should not match");
    }
}

fn contains(bit: usize) -> Pattern {
    Pattern::sequence(vec![Pattern::universal(), Pattern::bit(bit), Pattern::universal()])
}

#[test]
fn general_regex() {
    check(Pattern::epsilon(), &[""], &["a", "a b"]);
    check(Pattern::bit(0), &["a", "ab"], &["", "b", "a b"]);
    check(Pattern::not_bit(0), &["b"], &["", "a", "ab", "b c"]);
    check(Pattern::any_symbol(), &["a", "ab"], &["", "a b"]);
    check(Pattern::universal(), &["", "a", "a b c"], &[]);

    check(Pattern::sequence(vec![Pattern::bit(0), Pattern::bit(1)]), &["a b", "ab b"], &["", "a", "b a", "a c"]);
    check(Pattern::choice(vec![Pattern::bit(0), Pattern::bit(1)]), &["a", "b", "ab"], &["", "c", "a b"]);
    check(Pattern::bit(0).repeat(), &["", "a", "a a", "ab a"], &["b", "a b"]);
    check(Pattern::not_bit(0).repeat(), &["", "b", "b c"], &["a", "b a"]);

    // Every required bit may occur in any order, or on the same input symbol.
    check(Pattern::intersection(vec![contains(0), contains(1)]), &["a b", "b a", "ab", "x a x b x"], &["", "a", "b", "a x"]);
}

#[test]
fn shared_prefix_transitions() {
    // A derivative can be continued along different postfixes without recalculating their shared prefix.
    let mut regexp = GenRegExp::new(Pattern::sequence(vec![Pattern::bit(0), Pattern::choice(vec![Pattern::bit(1), Pattern::bit(2)])]));
    let prefix = regexp.transition(regexp.start_state(), &symbol("a"));
    let with_b = regexp.transition(prefix, &symbol("b"));
    let with_c = regexp.transition(prefix, &symbol("c"));
    let with_d = regexp.transition(prefix, &symbol("d"));
    assert!(regexp.is_match(with_b));
    assert!(regexp.is_match(with_c));
    assert!(!regexp.is_match(with_d));
}

#[test]
fn multi_word_bitsets() {
    // Input bitsets can span several u64 words.
    let mut regexp = GenRegExp::new(Pattern::sequence(vec![Pattern::bit(70), Pattern::bit(1)]));
    assert!(regexp.matches(&[vec![0, 1 << 6], vec![1 << 1]]));
    assert!(!regexp.matches(&[vec![1 << 6], vec![1 << 1]]));

    // Repeating a known transition reuses the same DFA states.
    let states = regexp.state_count();
    assert!(regexp.matches(&[vec![0, 1 << 6], vec![1 << 1]]));
    assert_eq!(regexp.state_count(), states);
}
