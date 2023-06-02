use std::collections::{HashMap, HashSet};
use std::num::NonZeroU32;

use crate::grammar::{Grammar, GrammarError, Symbol};
use lapex_input::RuleSet;

fn get_follow_symbols_of_remainder(
    lhs: Option<Symbol>,
    remainder: &[Symbol],
    first_sets: &HashMap<Symbol, HashSet<Symbol>>,
    follow_sets: &HashMap<Symbol, HashSet<Symbol>>,
) -> HashSet<Symbol> {
    let mut result_set = HashSet::new();
    let remainder_first_set = get_first_symbols_of_sequence(remainder, first_sets);
    let remainder_first_has_epsilon = remainder_first_set.contains(&Symbol::Epsilon);
    let should_add_lhs_follow_set = remainder_first_has_epsilon || remainder.is_empty();
    if should_add_lhs_follow_set {
        let follow_set_of_lhs = follow_sets.get(&lhs.unwrap()).unwrap().clone();
        result_set.extend(follow_set_of_lhs);
    }
    for remainder_first_symbol in remainder_first_set {
        if remainder_first_symbol != Symbol::Epsilon {
            result_set.insert(remainder_first_symbol);
        }
    }

    result_set
}

fn compute_follow_sets(
    grammar: &Grammar,
    first_sets: &HashMap<Symbol, HashSet<Symbol>>,
) -> HashMap<Symbol, HashSet<Symbol>> {
    // init empty first sets
    let mut follow_sets = HashMap::new();
    for nt in grammar.non_terminals() {
        follow_sets.insert(nt, HashSet::new());
    }
    // repeat until no more changes occur
    let terminated_entry_point_rhs = vec![*grammar.entry_point(), Symbol::End];
    loop {
        let grammar_rules = grammar.rules().iter().map(|r| (Some(r.lhs()), r.rhs()));
        let all_rules = std::iter::once((None, &terminated_entry_point_rhs)).chain(grammar_rules);
        let mut inserted_any = false;
        for rule in all_rules {
            let lhs = rule.0;
            let sequence = rule.1;
            for i in 0..sequence.len() {
                let symbol = &sequence[i];
                if let Symbol::NonTerminal(_) = symbol {
                    let remainder = &sequence[i + 1..];
                    let follow_symbols_for_remainder =
                        get_follow_symbols_of_remainder(lhs, remainder, &first_sets, &follow_sets);
                    let follow_set_of_nt = follow_sets.get_mut(symbol).unwrap();
                    for follow_symbol in follow_symbols_for_remainder {
                        let was_inserted = follow_set_of_nt.insert(follow_symbol);
                        inserted_any = inserted_any || was_inserted;
                    }
                }
            }
        }
        if !inserted_any {
            break;
        }
    }

    follow_sets
}

fn get_first_symbols_of_sequence(
    sequence: &[Symbol],
    first_sets: &HashMap<Symbol, HashSet<Symbol>>,
) -> HashSet<Symbol> {
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
            Symbol::End | Symbol::Terminal(_) => {
                result_set.insert(symbol);
                return result_set;
            }
            Symbol::Epsilon | Symbol::NonTerminal(_) => {
                let first_set_for_symbol = if symbol == Symbol::Epsilon {
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

#[derive(Debug)]
pub struct ParserTable {
    table: HashMap<(u32, Option<NonZeroU32>), Vec<Symbol>>,
}

impl ParserTable {
    fn new() -> Self {
        ParserTable {
            table: HashMap::new(),
        }
    }

    fn insert(
        &mut self,
        non_terminal: Symbol,
        terminal: Symbol,
        production: Vec<Symbol>,
    ) -> Result<(), ()> {
        if let Symbol::NonTerminal(non_terminal_index) = non_terminal {
            match terminal {
                Symbol::Terminal(terminal_index) => {
                    let prev_entry = self.table.insert(
                        (non_terminal_index, NonZeroU32::new(terminal_index + 1)),
                        production,
                    );
                    if prev_entry.is_none() {
                        return Ok(());
                    }
                }
                Symbol::End => {
                    let prev_entry = self.table.insert((non_terminal_index, None), production);
                    if prev_entry.is_none() {
                        return Ok(());
                    }
                }
                _ => (),
            }
        }
        Err(())
    }
}

pub fn generate_table(rule_set: &RuleSet) -> Result<ParserTable, GrammarError> {
    let grammar = Grammar::from_rule_set(rule_set)?;
    let first_sets = compute_first_sets(&grammar);
    let follow_sets = compute_follow_sets(&grammar, &first_sets);
    let mut parser_table = ParserTable::new();
    for rule in grammar.rules() {
        let first_set_of_rhs = get_first_symbols_of_sequence(rule.rhs(), &first_sets);
        for symbol in first_set_of_rhs.iter() {
            match symbol {
                Symbol::End | Symbol::Terminal(_) => {
                    parser_table
                        .insert(rule.lhs(), *symbol, rule.rhs().clone())
                        .expect("parser table conflict");
                }
                _ => (),
            }
        }
        if first_set_of_rhs.contains(&Symbol::Epsilon) {
            let follow_set_of_lhs = follow_sets.get(&rule.lhs()).unwrap();
            for symbol in follow_set_of_lhs.iter() {
                match symbol {
                    Symbol::End | Symbol::Terminal(_) => {
                        parser_table
                            .insert(rule.lhs(), *symbol, rule.rhs().clone())
                            .expect("parser table conflict");
                    }
                    _ => (),
                }
            }
        }
    }
    Ok(parser_table)
}

#[cfg(test)]
mod tests;
