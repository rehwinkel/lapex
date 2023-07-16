use crate::grammar::{Grammar, Symbol};
use crate::ll_parser::LLParserError;

use super::{generate_table, LLParserTable};

#[test]
fn test_generate_table_valid() {
    let grammar = r#"
    token PLUS = "+";
    token MINUS = "-";
    token NUMBER = /[0-9]+/;

    entry sum;
    prod sum = NUMBER ((PLUS | MINUS) NUMBER)*;
    "#;
    let rules = lapex_input::parse_lapex_file(grammar.as_bytes()).unwrap();
    let grammar = Grammar::from_rule_set(&rules).unwrap();
    let table = generate_table(&grammar).unwrap();
    let mut target_table = LLParserTable::new();
    target_table
        .insert(
            Symbol::NonTerminal(0),
            Symbol::Terminal(2),
            vec![Symbol::Terminal(2), Symbol::NonTerminal(1)],
        )
        .unwrap();
    target_table
        .insert(Symbol::NonTerminal(1), Symbol::End, vec![Symbol::Epsilon])
        .unwrap();
    target_table
        .insert(
            Symbol::NonTerminal(1),
            Symbol::Terminal(0),
            vec![
                Symbol::NonTerminal(2),
                Symbol::Terminal(2),
                Symbol::NonTerminal(1),
            ],
        )
        .unwrap();
    target_table
        .insert(
            Symbol::NonTerminal(1),
            Symbol::Terminal(1),
            vec![
                Symbol::NonTerminal(2),
                Symbol::Terminal(2),
                Symbol::NonTerminal(1),
            ],
        )
        .unwrap();
    target_table
        .insert(
            Symbol::NonTerminal(2),
            Symbol::Terminal(0),
            vec![Symbol::Terminal(0)],
        )
        .unwrap();
    target_table
        .insert(
            Symbol::NonTerminal(2),
            Symbol::Terminal(1),
            vec![Symbol::Terminal(1)],
        )
        .unwrap();
    assert_eq!(table, target_table);
}

#[test]
fn test_generate_table_first_conflict() {
    let grammar = r#"
    token A = "a";
    token B = "b";
    token C = "c";

    entry x;
    prod x = (A B | A C);
    "#;
    let rules = lapex_input::parse_lapex_file(grammar.as_bytes()).unwrap();
    let grammar = Grammar::from_rule_set(&rules).unwrap();
    assert_eq!(
        generate_table(&grammar),
        Err(LLParserError::ParserTableConflict {
            non_terminal: Symbol::NonTerminal(1),
            terminal: Symbol::Terminal(0),
            production: vec![Symbol::Terminal(0), Symbol::Terminal(2)],
            existing_production: vec![Symbol::Terminal(0), Symbol::Terminal(1)],
        })
    );
}

#[test]
fn test_generate_table_first_follow_conflict() {
    let grammar = r#"
    token A = "a";
    token B = "b";
    token C = "c";

    entry x;
    prod x = y z;
    prod y = (A B)?;
    prod z = A C;
    "#;
    let rules = lapex_input::parse_lapex_file(grammar.as_bytes()).unwrap();
    let grammar = Grammar::from_rule_set(&rules).unwrap();
    assert_eq!(
        generate_table(&grammar),
        Err(LLParserError::ParserTableConflict {
            non_terminal: Symbol::NonTerminal(3),
            terminal: Symbol::Terminal(0),
            production: vec![Symbol::Epsilon],
            existing_production: vec![Symbol::Terminal(0), Symbol::Terminal(1)],
        })
    );
}

#[test]
fn test_generate_table_follow_conflict() {
    let grammar = r#"
    token A = "a";
    token B = "b";
    token C = "c";

    entry x;
    prod x = (y | z) C;
    prod y = (A)?;
    prod z = (B)?;
    "#;
    let rules = lapex_input::parse_lapex_file(grammar.as_bytes()).unwrap();
    let grammar = Grammar::from_rule_set(&rules).unwrap();
    assert_eq!(
        generate_table(&grammar),
        Err(LLParserError::ParserTableConflict {
            non_terminal: Symbol::NonTerminal(1),
            terminal: Symbol::Terminal(2),
            production: vec![Symbol::NonTerminal(3)],
            existing_production: vec![Symbol::NonTerminal(2)],
        })
    );
}
