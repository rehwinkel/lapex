use std::{fmt::Display, io::BufWriter, path::Path};

use clap::ValueEnum;
use lapex_codegen::GeneratedCodeWriter;
use lapex_cpp_codegen::{CppLLParserCodeGen, CppLRParserCodeGen, CppLexerCodeGen};
use lapex_input::LapexInputParser;
use lapex_lexer::LexerCodeGen;
use lapex_parser::{grammar::Grammar, ll_parser::LLParserCodeGen, lr_parser::LRParserCodeGen};
use lapex_rust_codegen::{RustLLParserCodeGen, RustLRParserCodeGen, RustLexerCodeGen};

#[derive(Debug, Clone, ValueEnum)]
pub enum ParsingAlgorithm {
    LL1,
    LR0,
    LR1,
}

impl Display for ParsingAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ParsingAlgorithm::LL1 => "ll1",
                ParsingAlgorithm::LR0 => "lr0",
                ParsingAlgorithm::LR1 => "lr1",
            }
        )
    }
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Language {
    Rust,
    Cpp,
}

trait LanguageFactory<Lexer, LR, LL> {
    fn lexer(&self) -> Lexer;
    fn lr_parser(&self) -> LR;
    fn ll_parser(&self) -> LL;
}

struct CppLanguageFactory;

impl LanguageFactory<CppLexerCodeGen, CppLRParserCodeGen, CppLLParserCodeGen>
    for CppLanguageFactory
{
    fn lexer(&self) -> CppLexerCodeGen {
        CppLexerCodeGen::new()
    }

    fn lr_parser(&self) -> CppLRParserCodeGen {
        CppLRParserCodeGen::new()
    }

    fn ll_parser(&self) -> CppLLParserCodeGen {
        CppLLParserCodeGen::new()
    }
}

struct RustLanguageFactory;

impl LanguageFactory<RustLexerCodeGen, RustLRParserCodeGen, RustLLParserCodeGen>
    for RustLanguageFactory
{
    fn lexer(&self) -> RustLexerCodeGen {
        RustLexerCodeGen::new()
    }

    fn lr_parser(&self) -> RustLRParserCodeGen {
        RustLRParserCodeGen::new()
    }

    fn ll_parser(&self) -> RustLLParserCodeGen {
        RustLLParserCodeGen::new()
    }
}

fn generate_lexer_and_parser<L, LR, LL, F, I>(
    generate_lexer: bool,
    algorithm: ParsingAlgorithm,
    generate_table: bool,
    grammar_path: &Path,
    target_path: &Path,
    language: F,
    input_parser: I,
) where
    L: LexerCodeGen,
    LR: LRParserCodeGen,
    LL: LLParserCodeGen,
    F: LanguageFactory<L, LR, LL>,
    I: LapexInputParser,
{
    let lexer_codegen = language.lexer();
    let ll_codegen = language.ll_parser();
    let lr_codegen = language.lr_parser();

    let file_contents = std::fs::read_to_string(grammar_path).unwrap();
    let rules = input_parser.parse_lapex(file_contents.as_str()).unwrap();
    let mut gen = GeneratedCodeWriter::with_default(|name| {
        let file = std::fs::File::create(target_path.join(name)).unwrap();
        BufWriter::new(file)
    });
    lexer_codegen.generate_tokens(rules.tokens(), &mut gen);

    if generate_lexer {
        let alphabet = lapex_lexer::generate_alphabet(rules.tokens());
        let (nfa_entrypoint, nfa) = lapex_lexer::generate_nfa(&alphabet, rules.tokens());
        let dfa = lapex_lexer::apply_precedence_to_dfa(nfa.powerset_construction(nfa_entrypoint))
            .unwrap();

        lexer_codegen.generate_lexer(rules.tokens(), &alphabet.get_ranges(), &dfa, &mut gen);
    }

    let grammar = Grammar::from_rule_set(&rules).unwrap();
    match algorithm {
        ParsingAlgorithm::LL1 => {
            let parser_table = lapex_parser::ll_parser::generate_table(&grammar).unwrap();
            ll_codegen.generate_code(&grammar, &parser_table, &mut gen);
        }
        ParsingAlgorithm::LR0 => {
            let parser_table = lapex_parser::lr_parser::generate_table::<0>(&grammar).unwrap();
            if generate_table {
                gen.generate_code("table", |output| {
                    lapex_parser::lr_parser::output_table(&grammar, &parser_table, output)
                })
                .unwrap();
            }
            lr_codegen.generate_code(&grammar, &parser_table, &mut gen);
        }
        ParsingAlgorithm::LR1 => {
            let parser_table = lapex_parser::lr_parser::generate_table::<1>(&grammar).unwrap();
            if generate_table {
                gen.generate_code("table", |output| {
                    lapex_parser::lr_parser::output_table(&grammar, &parser_table, output)
                })
                .unwrap();
            }
            lr_codegen.generate_code(&grammar, &parser_table, &mut gen);
        }
    };
}

pub fn generate<I>(
    generate_lexer: bool,
    algorithm: ParsingAlgorithm,
    generate_table: bool,
    grammar_path: &Path,
    target_path: &Path,
    language: Language,
    input_parser: I,
) where
    I: LapexInputParser,
{
    match language {
        Language::Cpp => generate_lexer_and_parser(
            generate_lexer,
            algorithm,
            generate_table,
            grammar_path,
            target_path,
            CppLanguageFactory {},
            input_parser,
        ),
        Language::Rust => generate_lexer_and_parser(
            generate_lexer,
            algorithm,
            generate_table,
            grammar_path,
            target_path,
            RustLanguageFactory {},
            input_parser,
        ),
    }
}
