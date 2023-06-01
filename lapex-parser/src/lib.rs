use std::collections::{HashMap, HashSet};

use lapex_input::{EntryRule, ProductionRule, TokenRule};

mod grammar;
use grammar::{Grammar, GrammarError, Symbol};

fn compute_follow_sets(
    first_sets: &HashMap<Symbol, HashSet<Symbol>>,
    grammar: &Grammar,
) -> HashMap<Symbol, HashSet<Symbol>> {
    let mut follow_sets: HashMap<Symbol, HashSet<Symbol>> = grammar
        .non_terminals()
        .map(|nt| (nt, HashSet::new()))
        .collect();
    loop {
        let mut changed = false;
        for rule in grammar.rules() {
            for window in rule.rhs().windows(2) {
                println!("{:?}", window);
            }
        }
        if !changed {
            break;
        }
    }
    println!("{:?}", follow_sets);
    follow_sets
}

fn compute_first_sets(grammar: &Grammar) -> HashMap<Symbol, HashSet<Symbol>> {
    let mut first_sets: HashMap<Symbol, HashSet<Symbol>> = grammar
        .non_terminals()
        .map(|nt| (nt, HashSet::new()))
        .collect();
    loop {
        let mut changed = false;
        for rule in grammar.rules() {
            // unwrap because rule can never be empty
            for (n, nth_symbol) in rule.rhs().iter().enumerate() {
                match nth_symbol {
                    Symbol::Epsilon => {
                        if first_sets
                            .get_mut(&rule.lhs())
                            .expect("first set missing for rule")
                            .insert(Symbol::Epsilon)
                        {
                            changed = true;
                        }
                    }
                    Symbol::NonTerminal(_) => {
                        let mut has_epsilon = true;
                        let nth_first_set: &HashSet<Symbol> = first_sets
                            .get(nth_symbol)
                            .expect("nonterminal doesn't have rule");
                        if !nth_first_set.contains(&Symbol::Epsilon) {
                            has_epsilon = false;
                        }

                        let first_symbols: Vec<Symbol> = nth_first_set
                            .iter()
                            .filter(|s| s != &&Symbol::Epsilon)
                            .map(|s| *s)
                            .collect();
                        let rule_first_set = first_sets
                            .get_mut(&rule.lhs())
                            .expect("first set missing for rule");
                        for nt_first in first_symbols {
                            if rule_first_set.insert(nt_first) {
                                changed = true;
                            }
                        }
                        // if it has epsilon and is the last symbol
                        if has_epsilon && (n + 1 == rule.rhs().len()) {
                            if rule_first_set.insert(Symbol::Epsilon) {
                                changed = true;
                            }
                        }
                        if !has_epsilon {
                            break;
                        }
                    }
                    Symbol::Terminal(_) => {
                        if first_sets
                            .get_mut(&rule.lhs())
                            .expect("first set missing for rule")
                            .insert(*nth_symbol)
                        {
                            changed = true;
                            break;
                        }
                    }
                }
            }
        }
        if !changed {
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
    let follow_sets = compute_follow_sets(&first_sets, &grammar);
    Ok(())
}
