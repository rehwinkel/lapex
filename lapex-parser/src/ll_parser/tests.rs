use crate::grammar::Symbol;

use super::{generate_table, ParserTable};

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
    let table = generate_table(&rules).unwrap();
    let mut target_table = ParserTable::new();
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
