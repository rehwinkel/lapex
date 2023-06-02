use std::ops::RangeInclusive;

use lapex_input::TokenRule;
use lapex_lexer::{Dfa, LexerCodeGen};

fn main() {
    let path = "example/test3.lapex";
    let file_contents = std::fs::read(path).unwrap();
    let rules = lapex_input::parse_lapex_file(&file_contents).unwrap();
    let (alphabet, dfa) = lapex_lexer::generate_dfa(rules.tokens());
    generate_cpp_lexer(rules.tokens(), &alphabet, &dfa);
    let parser_table = lapex_parser::ll_parser::generate_table(&rules).unwrap();
    println!("{:?}", parser_table);
}

fn generate_cpp_lexer(tokens: &[TokenRule], alphabet: &[RangeInclusive<u32>], dfa: &Dfa) {
    let cpp_codegen = lapex_cpp_codegen::CppLexerCodeGen::new();
    if cpp_codegen.has_header() {
        let mut lexer_h = std::fs::File::create("lexer.h").unwrap();
        cpp_codegen
            .generate_header(tokens, &alphabet, &dfa, &mut lexer_h)
            .unwrap();
    }
    let mut lexer_cpp = std::fs::File::create("lexer.cpp").unwrap();
    cpp_codegen
        .generate_source(tokens, &alphabet, &dfa, &mut lexer_cpp)
        .unwrap();
}
