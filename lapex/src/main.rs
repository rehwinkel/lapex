use lapex_lexer::LexerCodeGen;

fn main() {
    let path = "example/test1.lapex";
    let file_contents = std::fs::read(path).unwrap();
    let (remaining, rules) = lapex_input::parse_lapex(&file_contents).unwrap();
    assert!(
        remaining.is_empty(),
        "Didn't finish parsing file: \n {}",
        std::str::from_utf8(remaining).unwrap()
    );
    let mut token_rules = Vec::new();
    let mut prod_rules = Vec::new();
    let mut entry_rules = Vec::new();
    for rule in rules {
        match rule {
            lapex_input::Rule::TokenRule(tr) => token_rules.push(tr),
            lapex_input::Rule::ProductionRule(pr) => prod_rules.push(pr),
            lapex_input::Rule::EntryRule(er) => entry_rules.push(er),
        }
    }
    assert_eq!(entry_rules.len(), 1);
    let entry_rule = entry_rules.remove(0);
    let (_alphabet, _dfa) = lapex_lexer::generate_dfa(&token_rules);
    let parser_table = lapex_parser::generate_table(&entry_rule, &token_rules, &prod_rules);
    // TODO: missing nonterminals and terminals (name mapping hashmap?)
    println!("{:?}", parser_table.debug(&token_rules, &prod_rules));

    /*
    let cpp_codegen = lapex_lexer::CppLexerCodeGen::new();
    if cpp_codegen.has_header() {
        let mut lexer_h = std::fs::File::create("lexer.h").unwrap();
        cpp_codegen
            .generate_header(&token_rules, &alphabet, &dfa, &mut lexer_h)
            .unwrap();
    }
    let mut lexer_cpp = std::fs::File::create("lexer.cpp").unwrap();
    cpp_codegen
        .generate_source(&token_rules, &alphabet, &dfa, &mut lexer_cpp)
        .unwrap();
        */
}
