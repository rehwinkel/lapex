use std::{fmt::Display, io::BufWriter, path::Path};

use clap::ValueEnum;
use errors::LapexError;
use lapex_codegen::GeneratedCodeWriter;
use lapex_cpp_codegen::{
    CppGLRParserCodeGen, CppLLParserCodeGen, CppLRParserCodeGen, CppLexerCodeGen,
};
use lapex_input::LapexInputParser;
use lapex_lexer::LexerCodeGen;
use lapex_parser::{
    grammar::Grammar,
    ll_parser::LLParserCodeGen,
    lr_parser::{GenerationResult, LRParserCodeGen},
};
use lapex_rust_codegen::{
    RustGLRParserCodeGen, RustLLParserCodeGen, RustLRParserCodeGen, RustLexerCodeGen,
};

mod errors;

#[derive(Debug, Clone, ValueEnum)]
pub enum ParsingAlgorithm {
    LL1,
    LR0,
    LR1,
    GLR,
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
                ParsingAlgorithm::GLR => "glr",
            }
        )
    }
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Language {
    Rust,
    Cpp,
}

trait LanguageFactory<Lexer, LR, LL, GLR> {
    fn lexer(&self) -> Lexer;
    fn lr_parser(&self) -> LR;
    fn glr_parser(&self) -> GLR;
    fn ll_parser(&self) -> LL;
}

struct CppLanguageFactory;

impl LanguageFactory<CppLexerCodeGen, CppLRParserCodeGen, CppLLParserCodeGen, CppGLRParserCodeGen>
    for CppLanguageFactory
{
    fn lexer(&self) -> CppLexerCodeGen {
        CppLexerCodeGen::new()
    }

    fn lr_parser(&self) -> CppLRParserCodeGen {
        CppLRParserCodeGen::new()
    }

    fn glr_parser(&self) -> CppGLRParserCodeGen {
        CppGLRParserCodeGen::new()
    }

    fn ll_parser(&self) -> CppLLParserCodeGen {
        CppLLParserCodeGen::new()
    }
}

struct RustLanguageFactory;

impl
    LanguageFactory<
        RustLexerCodeGen,
        RustLRParserCodeGen,
        RustLLParserCodeGen,
        RustGLRParserCodeGen,
    > for RustLanguageFactory
{
    fn lexer(&self) -> RustLexerCodeGen {
        RustLexerCodeGen::new()
    }

    fn lr_parser(&self) -> RustLRParserCodeGen {
        RustLRParserCodeGen::new()
    }

    fn glr_parser(&self) -> RustGLRParserCodeGen {
        RustGLRParserCodeGen::new()
    }

    fn ll_parser(&self) -> RustLLParserCodeGen {
        RustLLParserCodeGen::new()
    }
}

fn generate_lexer_and_parser<L, LR, LL, GLR, F, I>(
    generate_lexer: bool,
    algorithm: ParsingAlgorithm,
    generate_table: bool,
    grammar_path: &Path,
    target_path: &Path,
    language: F,
    input_parser: I,
) -> Result<(), Vec<LapexError>>
where
    L: LexerCodeGen,
    LR: LRParserCodeGen,
    LL: LLParserCodeGen,
    GLR: LRParserCodeGen,
    F: LanguageFactory<L, LR, LL, GLR>,
    I: LapexInputParser,
{
    let lexer_codegen = language.lexer();
    let ll_codegen = language.ll_parser();
    let lr_codegen = language.lr_parser();
    let glr_codegen = language.glr_parser();

    let file_contents = std::fs::read_to_string(grammar_path)
        .map_err(|e| LapexError::io(grammar_path.to_path_buf(), e))?;
    let rules = input_parser
        .parse_lapex(file_contents.as_str())
        .expect("TODO");
    let mut gen = GeneratedCodeWriter::with_default(|name| {
        let file = std::fs::File::create(target_path.join(name))?;
        Ok(BufWriter::new(file))
    });
    lexer_codegen.generate_tokens(&rules.token_rules, &mut gen);

    if generate_lexer {
        let alphabet = lapex_lexer::generate_alphabet(&rules.token_rules);
        let (nfa_entrypoint, nfa) = lapex_lexer::generate_nfa(&alphabet, &rules.token_rules);
        let dfa = lapex_lexer::apply_precedence_to_dfa(nfa.powerset_construction(nfa_entrypoint))
            .map_err(|e| LapexError::precedence(grammar_path, file_contents.as_str(), e))?;

        lexer_codegen.generate_lexer(&rules.token_rules, &alphabet.get_ranges(), &dfa, &mut gen);
    }

    let grammar = Grammar::from_rule_set(&rules).expect("TODO");
    match algorithm {
        ParsingAlgorithm::LL1 => {
            let parser_table = lapex_parser::ll_parser::generate_table(&grammar).expect("TODO");
            ll_codegen.generate_code(&grammar, &parser_table, &mut gen);
        }
        ParsingAlgorithm::LR0 => {
            let parser_table = match lapex_parser::lr_parser::generate_table::<0>(&grammar, false) {
                GenerationResult::NoConflicts(val) => val,
                GenerationResult::BadConflicts(conflicts) => {
                    return Err(LapexError::conflicts(
                        grammar_path,
                        file_contents.as_str(),
                        &conflicts,
                        &grammar,
                    )
                    .into());
                }
                _ => unreachable!(),
            };
            if generate_table {
                gen.generate_code("table", |output| {
                    lapex_parser::lr_parser::output_table(&grammar, &parser_table, output)
                })
                .expect("TODO");
            }
            lr_codegen.generate_code(&grammar, &parser_table, &mut gen);
        }
        ParsingAlgorithm::LR1 => {
            let parser_table = match lapex_parser::lr_parser::generate_table::<1>(&grammar, false) {
                GenerationResult::NoConflicts(val) => val,
                GenerationResult::BadConflicts(conflicts) => {
                    return Err(LapexError::conflicts(
                        grammar_path,
                        file_contents.as_str(),
                        &conflicts,
                        &grammar,
                    )
                    .into());
                }
                _ => unreachable!(),
            };
            if generate_table {
                gen.generate_code("table", |output| {
                    lapex_parser::lr_parser::output_table(&grammar, &parser_table, output)
                })
                .expect("TODO");
            }
            lr_codegen.generate_code(&grammar, &parser_table, &mut gen);
        }
        ParsingAlgorithm::GLR => {
            let parser_table = match lapex_parser::lr_parser::generate_table::<1>(&grammar, true) {
                GenerationResult::NoConflicts(table) => {
                    // TODO: info about using LR1 instead
                    table
                }
                GenerationResult::AllowedConflicts {
                    table,
                    conflicts: _conflicts,
                } => {
                    // TODO: info about conflicts
                    table
                }
                _ => unreachable!(),
            };
            if generate_table {
                gen.generate_code("table", |output| {
                    lapex_parser::lr_parser::output_table(&grammar, &parser_table, output)
                })
                .expect("TODO");
            }
            glr_codegen.generate_code(&grammar, &parser_table, &mut gen);
        }
    };
    Ok(())
}

pub fn generate<I>(
    generate_lexer: bool,
    algorithm: ParsingAlgorithm,
    generate_table: bool,
    grammar_path: &Path,
    target_path: &Path,
    language: Language,
    input_parser: I,
) -> Result<(), Vec<LapexError>>
where
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
