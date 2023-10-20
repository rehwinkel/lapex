use std::{collections::BTreeSet, iter::Peekable};

use lapex_automaton::{Nfa, StateId};

use lapex_input::{Characters, Pattern, Spanned, TokenPattern, TokenRule};

use crate::alphabet::Alphabet;

fn chain_pattern_iterator<'rules, 'p, I>(
    alphabet: &Alphabet,
    nfa: &mut Nfa<&'rules Spanned<TokenRule<'rules>>, usize>,
    mut patterns: Peekable<I>,
    start: StateId,
    end: StateId,
) -> Vec<StateId>
where
    I: Iterator<Item = &'p Pattern>,
{
    if patterns.peek().is_none() {
        panic!("iterator is empty");
    }
    let mut intermediates = Vec::new();
    let mut inner_start = start;
    while let Some(p) = patterns.next() {
        if !patterns.peek().is_none() {
            let inner_end = nfa.add_intermediate_state();
            intermediates.push(inner_end);
            build_nfa_from_pattern(inner_start, inner_end, alphabet, nfa, p);
            inner_start = inner_end;
        } else {
            build_nfa_from_pattern(inner_start, end, alphabet, nfa, p);
        }
    }
    intermediates
}

fn chain_pattern_times<'rules, 'p>(
    alphabet: &Alphabet,
    nfa: &mut Nfa<&'rules Spanned<TokenRule<'rules>>, usize>,
    times: usize,
    pattern: &Pattern,
    start: StateId,
    end: StateId,
) -> Vec<StateId> {
    chain_pattern_iterator(
        alphabet,
        nfa,
        (0..times).into_iter().map(|_i| pattern).peekable(),
        start,
        end,
    )
}

fn build_nfa_from_pattern<'rules>(
    start: StateId,
    end: StateId,
    alphabet: &Alphabet,
    nfa: &mut Nfa<&'rules Spanned<TokenRule<'rules>>, usize>,
    pattern: &Pattern,
) -> Option<()> {
    match &pattern {
        Pattern::Sequence { elements } => {
            if !elements.is_empty() {
                chain_pattern_iterator(alphabet, nfa, elements.into_iter().peekable(), start, end);
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
        Pattern::Repetition { min, max, inner } => {
            let inner_start = nfa.add_intermediate_state();
            let inner_end = nfa.add_intermediate_state();
            nfa.add_epsilon_transition(start, inner_start);

            match (min, max) {
                (0, None) => {
                    build_nfa_from_pattern(inner_start, inner_end, alphabet, nfa, inner);
                    nfa.add_epsilon_transition(start, end);
                    nfa.add_epsilon_transition(inner_end, inner_start);
                    nfa.add_epsilon_transition(inner_end, end);
                }
                (n, max) => {
                    let mut intermediates = if *n == 0 {
                        nfa.add_epsilon_transition(inner_start, inner_end);
                        Vec::new()
                    } else {
                        chain_pattern_times(
                            alphabet,
                            nfa,
                            *n as usize,
                            inner,
                            inner_start,
                            inner_end,
                        )
                    };
                    match max {
                        None => {
                            let previous = intermediates.pop().unwrap_or(inner_start);
                            nfa.add_epsilon_transition(inner_end, previous);
                            nfa.add_epsilon_transition(inner_end, end);
                        }
                        Some(max) => {
                            let additional_until_max = max - min;
                            let max_start = nfa.add_intermediate_state();
                            nfa.add_epsilon_transition(inner_end, max_start);
                            let max_end = nfa.add_intermediate_state();
                            let mut max_intermediates = chain_pattern_times(
                                alphabet,
                                nfa,
                                additional_until_max as usize,
                                inner,
                                max_start,
                                max_end,
                            );
                            max_intermediates.push(max_start);
                            max_intermediates.push(max_end);
                            for mi in max_intermediates {
                                nfa.add_epsilon_transition(mi, end);
                            }
                        }
                    }
                }
            }
        }
        Pattern::CharSet {
            chars: chars_vec,
            negated,
        } => {
            let mut indices = BTreeSet::new();
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
    rules: &'rules [Spanned<TokenRule>],
) -> (StateId, Nfa<&'rules Spanned<TokenRule<'rules>>, usize>) {
    let mut nfa: Nfa<&'rules Spanned<TokenRule<'rules>>, usize> = Nfa::new();

    let start = nfa.add_intermediate_state();
    for rule in rules {
        let rule_start = nfa.add_intermediate_state();
        let rule_end = nfa.add_accepting_state(&rule);
        nfa.add_epsilon_transition(start, rule_start);
        match &rule.inner.pattern {
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

#[cfg(test)]
mod tests;
