use std::ops::RangeInclusive;
use std::path::Path;

use clap::{arg, command, Parser};
use lapex_automaton::Dfa;
use lapex_input::TokenRule;
use lapex_lexer::LexerCodeGen;
use lapex_parser::grammar::Grammar;
use lapex_parser::ll_parser::{LLParserTable, TableParserCodeGen};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CommandLine {
    #[arg(required = true)]
    grammar: String,
}

fn main() {
    let cli = CommandLine::parse();
    let path = &cli.grammar;
    let target_path = "generated";
    let file_contents = std::fs::read(path).unwrap();
    let rules = lapex_input::parse_lapex_file(&file_contents).unwrap();
    let alphabet = lapex_lexer::generate_alphabet(rules.tokens());
    let (nfa_entrypoint, nfa) = lapex_lexer::generate_nfa(&alphabet, rules.tokens());
    let dfa = nfa.powerset_construction(nfa_entrypoint);
    generate_cpp_lexer(
        rules.tokens(),
        &alphabet.get_ranges(),
        &dfa,
        Path::new(target_path),
    );
    let grammar = Grammar::from_rule_set(&rules).unwrap();
    println!("{}", grammar);
    let parser_table = lapex_parser::ll_parser::generate_table(&grammar).unwrap();
    println!("{:?}", parser_table);
    generate_cpp_parser(&grammar, &parser_table, Path::new(target_path));
}

fn generate_cpp_parser(grammar: &Grammar, table: &LLParserTable, target_path: &Path) {
    let cpp_codegen = lapex_cpp_codegen::CppTableParserCodeGen::new();
    if cpp_codegen.has_header() {
        let mut lexer_h = std::fs::File::create(target_path.join("parser.h")).unwrap();
        cpp_codegen
            .generate_header(grammar, table, &mut lexer_h)
            .unwrap();
    }
    let mut lexer_cpp = std::fs::File::create(target_path.join("parser.cpp")).unwrap();
    cpp_codegen
        .generate_source(grammar, table, &mut lexer_cpp)
        .unwrap();
}

fn generate_cpp_lexer(
    tokens: &[TokenRule],
    alphabet: &[RangeInclusive<u32>],
    dfa: &Dfa<Vec<String>, usize>,
    target_path: &Path,
) {
    let cpp_codegen = lapex_cpp_codegen::CppLexerCodeGen::new();
    if cpp_codegen.has_header() {
        let mut lexer_h = std::fs::File::create(target_path.join("lexer.h")).unwrap();
        cpp_codegen
            .generate_header(tokens, alphabet, dfa, &mut lexer_h)
            .unwrap();
    }
    let mut lexer_cpp = std::fs::File::create(target_path.join("lexer.cpp")).unwrap();
    cpp_codegen
        .generate_source(tokens, alphabet, dfa, &mut lexer_cpp)
        .unwrap();
}
