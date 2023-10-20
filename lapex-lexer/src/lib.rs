use std::collections::BTreeMap;

pub use codegen::*;

mod alphabet;
mod codegen;
mod nfa;
pub use alphabet::generate_alphabet;
use lapex_automaton::{AutomatonState, Dfa};
use lapex_input::{Spanned, TokenRule};
pub use nfa::generate_nfa;

#[derive(Debug)]
pub struct PrecedenceError {
    pub rules: Vec<Spanned<String>>,
    pub precedence: usize,
}

fn resolve_precedence<'rules>(
    rules: &Vec<&'rules Spanned<TokenRule<'rules>>>,
) -> Result<&'rules TokenRule<'rules>, PrecedenceError> {
    assert!(!rules.is_empty());
    let mut sorted_rules: Vec<(&Spanned<TokenRule>, usize)> =
        rules.iter().map(|r| (*r, r.inner.precedence())).collect();
    sorted_rules.sort_by_key(|r| std::cmp::Reverse(r.1));
    let highest_precedence = sorted_rules[0].1;
    let rules_with_matching_prec: Vec<&Spanned<TokenRule>> = sorted_rules
        .iter()
        .filter(|(_r, p)| *p == highest_precedence)
        .map(|(r, _p)| *r)
        .collect();
    if rules_with_matching_prec.len() > 1 {
        return Err(PrecedenceError {
            rules: rules_with_matching_prec
                .iter()
                .map(|r| Spanned::new(r.span, r.inner.name.to_string()))
                .collect(),
            precedence: highest_precedence,
        });
    }
    Ok(&rules_with_matching_prec[0].inner)
}

pub fn apply_precedence_to_dfa<'rules>(
    dfa: Dfa<Vec<&'rules Spanned<TokenRule<'rules>>>, usize>,
) -> Result<Dfa<&'rules TokenRule<'rules>, usize>, PrecedenceError> {
    let mut resulting_dfa = Dfa::new();
    let mut state_mapping = BTreeMap::new();
    for (idx, state) in dfa.states() {
        match state {
            AutomatonState::Accepting(accepted) => {
                let rule = resolve_precedence(accepted)?;
                let new_idx = resulting_dfa.add_accepting_state(rule);
                state_mapping.insert(idx, new_idx);
            }
            AutomatonState::Intermediate(_) => {
                let new_idx = resulting_dfa.add_intermediate_state();
                state_mapping.insert(idx, new_idx);
            }
        }
    }
    for (old_idx, new_idx) in &state_mapping {
        for (weight, old_target_idx) in dfa.transitions_from(*old_idx) {
            resulting_dfa.add_transition(
                *new_idx,
                *state_mapping.get(&old_target_idx).unwrap(),
                *weight,
            );
        }
    }
    Ok(resulting_dfa)
}
