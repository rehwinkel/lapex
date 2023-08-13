use lapex_automaton::{AutomatonState, NfaEdge};
use lapex_input::{Characters, Pattern, TokenPattern, TokenRule};
use petgraph::{prelude::DiGraph, Graph};

use crate::{generate_alphabet, generate_nfa};

#[test]
fn test_repetition_option() {
    let rules = [TokenRule {
        name: "test",
        pattern: TokenPattern::Pattern {
            pattern: Pattern::Repetition {
                min: 0,
                max: Some(1),
                inner: Box::new(Pattern::Char {
                    chars: Characters::Single('a'),
                }),
            },
        },
    }];
    let alphabet = generate_alphabet(&rules);
    let (_entry, nfa) = generate_nfa(&alphabet, &rules);
    println!("{:?}", alphabet);
    println!("{:?}", nfa);
}

#[test]
fn test_repetition_bounded() {
    let rules = [TokenRule {
        name: "test",
        pattern: TokenPattern::Pattern {
            pattern: Pattern::Repetition {
                min: 3,
                max: Some(5),
                inner: Box::new(Pattern::Char {
                    chars: Characters::Single('a'),
                }),
            },
        },
    }];
    let alphabet = generate_alphabet(&rules);
    let (_entry, nfa) = generate_nfa(&alphabet, &rules);
    println!("{:?}", alphabet);
    println!("{:?}", nfa);
}

#[test]
fn test_repetition_unbounded() {
    let rules = [TokenRule {
        name: "test",
        pattern: TokenPattern::Pattern {
            pattern: Pattern::Repetition {
                min: 0,
                max: None,
                inner: Box::new(Pattern::Char {
                    chars: Characters::Single('a'),
                }),
            },
        },
    }];
    let alphabet = generate_alphabet(&rules);
    let (_entry, nfa) = generate_nfa(&alphabet, &rules);
    println!("{:?}", alphabet);
    println!("{:?}", nfa);
}

#[test]
fn test_repetition_lower_bounded() {
    let rules = [TokenRule {
        name: "test",
        pattern: TokenPattern::Pattern {
            pattern: Pattern::Repetition {
                min: 3,
                max: None,
                inner: Box::new(Pattern::Char {
                    chars: Characters::Single('a'),
                }),
            },
        },
    }];
    let alphabet = generate_alphabet(&rules);
    let (_entry, nfa) = generate_nfa(&alphabet, &rules);
    println!("{:?}", alphabet);
    println!("{:?}", nfa);
}
