use std::ops::RangeInclusive;

use lapex_input::TokenRule;
use lapex_lexer::{Dfa, LexerCodeGen};
use lapex_parser::grammar::Grammar;
use lapex_parser::ll_parser::{ParserCodeGen, ParserTable};

fn main() {
    let path = "example/test1.lapex";
    let file_contents = std::fs::read(path).unwrap();
    let rules = lapex_input::parse_lapex_file(&file_contents).unwrap();
    let (alphabet, dfa) = lapex_lexer::generate_dfa(rules.tokens());
    generate_cpp_lexer(rules.tokens(), &alphabet, &dfa);
    let grammar = Grammar::from_rule_set(&rules).unwrap();
    println!("{}", grammar);
    let parser_table = lapex_parser::ll_parser::generate_table(&grammar).unwrap();
    println!("{:?}", parser_table);
    generate_cpp_parser(&grammar, &parser_table);
}

fn generate_cpp_parser(grammar: &Grammar, table: &ParserTable) {
    let cpp_codegen = lapex_cpp_codegen::CppTableParserCodeGen::new();
    if cpp_codegen.has_header() {
        let mut lexer_h = std::fs::File::create("parser.h").unwrap();
        cpp_codegen
            .generate_header(grammar, table, &mut lexer_h)
            .unwrap();
    }
    let mut lexer_cpp = std::fs::File::create("parser.cpp").unwrap();
    cpp_codegen
        .generate_source(grammar, table, &mut lexer_cpp)
        .unwrap();
}

fn generate_cpp_lexer(tokens: &[TokenRule], alphabet: &[RangeInclusive<u32>], dfa: &Dfa) {
    let cpp_codegen = lapex_cpp_codegen::CppLexerCodeGen::new();
    if cpp_codegen.has_header() {
        let mut lexer_h = std::fs::File::create("lexer.h").unwrap();
        cpp_codegen
            .generate_header(tokens, alphabet, dfa, &mut lexer_h)
            .unwrap();
    }
    let mut lexer_cpp = std::fs::File::create("lexer.cpp").unwrap();
    cpp_codegen
        .generate_source(tokens, alphabet, dfa, &mut lexer_cpp)
        .unwrap();
}
