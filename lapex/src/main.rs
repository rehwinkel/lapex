fn main() {
    let path = "example/test3.lapex";
    let file_contents = std::fs::read(path).unwrap();
    let rules = lapex_input::parse_lapex_file(&file_contents).unwrap();
    let (_alphabet, _dfa) = lapex_lexer::generate_dfa(rules.tokens());
    let parser_table = lapex_parser::ll_parser::generate_table(&rules).unwrap();
    println!("{:?}", parser_table);
    // println!("{:?}", parser_table.debug(&token_rules, &prod_rules));

    /*
    let (alphabet, dfa) = lapex_lexer::generate_dfa(&token_rules);
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
