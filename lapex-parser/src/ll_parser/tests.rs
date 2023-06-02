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
    let _target_table = ParserTable::new();
    println!("{:?}", table);
    todo!("compare")
}
