use std::collections::HashSet;

use lapex_automaton::{Nfa, StateId};

use lapex_input::{Characters, Pattern, TokenPattern, TokenRule};

use crate::alphabet::Alphabet;

fn build_nfa_from_pattern<'rules>(
    start: StateId,
    end: StateId,
    alphabet: &Alphabet,
    nfa: &mut Nfa<&'rules TokenRule<'rules>, usize>,
    pattern: &Pattern,
) -> Option<()> {
    match &pattern {
        Pattern::Sequence { elements } => {
            if !elements.is_empty() {
                let mut start = start;
                for pat in &elements[..elements.len() - 1] {
                    let end = nfa.add_intermediate_state();
                    build_nfa_from_pattern(start, end, alphabet, nfa, pat);
                    start = end;
                }
                build_nfa_from_pattern(start, end, alphabet, nfa, elements.last().unwrap());
            }
        }
        Pattern::Alternative { elements } => {
            for elem in elements {
                let inner_start = nfa.add_intermediate_state();
                let inner_end = nfa.add_intermediate_state();
                build_nfa_from_pattern(inner_start, inner_end, alphabet, nfa, elem);
                nfa.add_epsilon_transition(start, inner_start);
                nfa.add_epsilon_transition(inner_end, end);
            }
        }
        Pattern::Optional { inner } => {
            let inner_start = nfa.add_intermediate_state();
            let inner_end = nfa.add_intermediate_state();
            build_nfa_from_pattern(inner_start, inner_end, alphabet, nfa, inner);
            nfa.add_epsilon_transition(start, end);
            nfa.add_epsilon_transition(start, inner_start);
            nfa.add_epsilon_transition(inner_end, end);
        }
        Pattern::OneOrMany { inner } => {
            let inner_start = nfa.add_intermediate_state();
            let inner_end = nfa.add_intermediate_state();
            build_nfa_from_pattern(inner_start, inner_end, alphabet, nfa, inner);
            nfa.add_epsilon_transition(start, inner_start);
            nfa.add_epsilon_transition(inner_end, end);
            nfa.add_epsilon_transition(inner_end, inner_start);
        }
        Pattern::ZeroOrMany { inner } => {
            let inner_start = nfa.add_intermediate_state();
            let inner_end = nfa.add_intermediate_state();
            build_nfa_from_pattern(inner_start, inner_end, alphabet, nfa, inner);
            nfa.add_epsilon_transition(start, end);
            nfa.add_epsilon_transition(start, inner_start);
            nfa.add_epsilon_transition(inner_end, end);
            nfa.add_epsilon_transition(inner_end, inner_start);
        }
        Pattern::CharSet {
            chars: chars_vec,
            negated,
        } => {
            let mut indices = HashSet::new();
            for chars in chars_vec {
                match chars {
                    Characters::Single(ch) => {
                        let index = alphabet.find_range(*ch as u32)?;
                        indices.insert(index);
                    }
                    Characters::Range(rng_start, rng_end) => {
                        let index_start = alphabet.find_range(*rng_start as u32)?;
                        let index_end = alphabet.find_range(*rng_end as u32)?;
                        for i in index_start..=index_end {
                            indices.insert(i);
                        }
                    }
                }
            }
            if *negated {
                for i in 0..alphabet.get_ranges().len() {
                    if !indices.contains(&i) {
                        nfa.add_transition(start, end, i);
                    }
                }
            } else {
                for i in indices {
                    nfa.add_transition(start, end, i);
                }
            }
        }
        Pattern::Char { chars } => match chars {
            Characters::Single(ch) => {
                let index = alphabet.find_range(*ch as u32)?;
                nfa.add_transition(start, end, index);
            }
            Characters::Range(rng_start, rng_end) => {
                let index_start = alphabet.find_range(*rng_start as u32)?;
                let index_end = alphabet.find_range(*rng_end as u32)?;
                for i in index_start..=index_end {
                    nfa.add_transition(start, end, i);
                }
            }
        },
    }
    Some(())
}

pub fn generate_nfa<'rules>(
    alphabet: &Alphabet,
    rules: &'rules [TokenRule],
) -> (StateId, Nfa<&'rules TokenRule<'rules>, usize>) {
    let mut nfa = Nfa::new();

    let start = nfa.add_intermediate_state();
    for rule in rules {
        let rule_start = nfa.add_intermediate_state();
        let rule_end = nfa.add_accepting_state(rule);
        nfa.add_epsilon_transition(start, rule_start);
        match rule.pattern() {
            TokenPattern::Literal { characters } => build_nfa_from_pattern(
                rule_start,
                rule_end,
                alphabet,
                &mut nfa,
                &Pattern::from_chars(characters),
            ),
            TokenPattern::Pattern { pattern } => {
                build_nfa_from_pattern(rule_start, rule_end, alphabet, &mut nfa, pattern)
            }
        };
    }
    (start, nfa)
}
