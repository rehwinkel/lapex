use std::{collections::HashMap, error::Error, fmt::Display};

pub use codegen::*;

mod alphabet;
mod codegen;
mod nfa;
pub use alphabet::generate_alphabet;
use lapex_automaton::{AutomatonState, Dfa};
use lapex_input::TokenRule;
pub use nfa::generate_nfa;

#[derive(Debug)]
pub struct PrecedenceError<'rules> {
    rules: Vec<&'rules TokenRule<'rules>>,
}

impl<'rules> Error for PrecedenceError<'rules> {}
impl<'rules> Display for PrecedenceError<'rules> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Encountered a precedence conflict while scanning DFA. {}\n{:#?}",
            "Following rules have same precedence:", self.rules
        )
    }
}

fn resolve_precedence<'rules>(
    rules: &Vec<&'rules TokenRule<'rules>>,
) -> Result<&'rules TokenRule<'rules>, PrecedenceError<'rules>> {
    assert!(!rules.is_empty());
    let mut sorted_rules: Vec<(&TokenRule, usize)> =
        rules.iter().map(|r| (*r, r.precedence())).collect();
    sorted_rules.sort_by_key(|r| std::cmp::Reverse(r.1));
    let highest_precedence = sorted_rules[0].1;
    let rules_with_matching_prec: Vec<&TokenRule> = sorted_rules
        .iter()
        .filter(|(_r, p)| *p == highest_precedence)
        .map(|(r, _p)| *r)
        .collect();
    if rules_with_matching_prec.len() > 1 {
        return Err(PrecedenceError {
            rules: rules_with_matching_prec,
        });
    }
    Ok(rules_with_matching_prec[0])
}

pub fn apply_precedence_to_dfa<'rules>(
    dfa: Dfa<Vec<&'rules TokenRule<'rules>>, usize>,
) -> Result<Dfa<Vec<&'rules TokenRule<'rules>>, usize>, PrecedenceError> {
    let mut resulting_dfa = Dfa::new();
    let mut state_mapping = HashMap::new();
    for (idx, state) in dfa.states() {
        match state {
            AutomatonState::Accepting(accepted) => {
                let rule = resolve_precedence(accepted)?;
                let new_idx = resulting_dfa.add_accepting_state(vec![rule]);
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
