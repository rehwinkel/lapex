use std::collections::{HashMap, HashSet};

use grammar::{Grammar, GrammarError, Symbol};
use lapex_input::{EntryRule, ProductionRule, TokenRule};

mod grammar;

fn compute_follow_sets(
    first_sets: &HashMap<Symbol, HashSet<Symbol>>,
    grammar: &Grammar,
) -> HashMap<Symbol, HashSet<Symbol>> {
    todo!()
}

fn get_first_symbols_of_sequence(sequence: &Vec<Symbol>, first_sets: &HashMap<Symbol, HashSet<Symbol>>) -> HashSet<Symbol> {
    let epsilon_first_set = {
        let mut new_set = HashSet::new();
        new_set.insert(Symbol::Epsilon);
        new_set
    };

    let mut result_set = HashSet::new();
    for i in 0..sequence.len() {
        let symbol = sequence[i];
        let is_last = i + 1 == sequence.len();
        match symbol {
            Symbol::Terminal(_) => {
                result_set.insert(symbol);
                return result_set;
            }
            Symbol::Epsilon |
            Symbol::NonTerminal(_) => {
                let mut first_set_for_symbol = if symbol == Symbol::Epsilon {
                    &epsilon_first_set
                } else {
                    first_sets.get(&symbol).unwrap()
                };
                let has_epsilon = first_set_for_symbol.contains(&Symbol::Epsilon);
                for first_symbol in first_set_for_symbol {
                    if first_symbol != &Symbol::Epsilon {
                        result_set.insert(*first_symbol);
                    }
                }
                if !has_epsilon {
                    break;
                } else {
                    if is_last {
                        result_set.insert(Symbol::Epsilon);
                    }
                }
            }
        }
    }
    result_set
}

fn compute_first_sets(grammar: &Grammar) -> HashMap<Symbol, HashSet<Symbol>> {
    // init empty first sets
    let mut first_sets = HashMap::new();
    for nt in grammar.non_terminals() {
        first_sets.insert(nt, HashSet::new());
    }
    // repeat until no more changes occur
    loop {
        let mut inserted_any = false;
        for rule in grammar.rules() {
            let first_for_rhs = get_first_symbols_of_sequence(rule.rhs(), &first_sets);
            let first_set_of_lhs = first_sets.get_mut(&rule.lhs()).unwrap();
            for symbol in first_for_rhs {
                let was_inserted = first_set_of_lhs.insert(symbol);
                inserted_any = inserted_any || was_inserted;
            }
        }
        // if nothing new was added, we are done
        if !inserted_any {
            break;
        }
    }

    first_sets
}

pub fn generate_table(
    entry: &EntryRule,
    tokens: &[TokenRule],
    rules: &[ProductionRule],
) -> Result<(), GrammarError> {
    let grammar = Grammar::from_rules(entry, tokens, rules)?;
    println!("{}", grammar);
    let first_sets = compute_first_sets(&grammar);
    println!("{:?}", first_sets);
    let follow_sets = compute_follow_sets(&first_sets, &grammar);
    Ok(())
}
