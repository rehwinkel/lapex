use std::{
    error::Error,
    fmt::Display,
    io::BufWriter,
    path::{Path, PathBuf},
};

use clap::ValueEnum;
use lapex_codegen::GeneratedCodeWriter;
use lapex_cpp_codegen::{CppLLParserCodeGen, CppLRParserCodeGen, CppLexerCodeGen};
use lapex_input::{LapexInputParser, SourcePos, SourceSpan};
use lapex_lexer::{LexerCodeGen, PrecedenceError};
use lapex_parser::{
    grammar::{Grammar, Symbol},
    ll_parser::LLParserCodeGen,
    lr_parser::{Conflict, LRParserCodeGen},
};
use lapex_rust_codegen::{RustLLParserCodeGen, RustLRParserCodeGen, RustLexerCodeGen};
use owo_colors::OwoColorize;

#[derive(Debug)]
pub enum Severity {
    Error,
}

impl Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Error => write!(f, "{}", "error".bright_red().bold()),
        }
    }
}

#[derive(Debug)]
pub struct Location {
    pos: SourcePos,
    file: PathBuf,
    text: String,
}
impl Location {
    fn from_span(span: SourceSpan, file: &Path, contents: &str) -> Option<Location> {
        Some(Location {
            pos: span.start,
            file: file.to_path_buf(),
            text: span.substring(contents)?.to_string(),
        })
    }
}

#[derive(Debug)]
pub struct LapexError {
    severity: Severity,
    error: LapexErrorType,
}

#[derive(Debug)]
pub enum LapexErrorType {
    ShiftReduce {
        symbol_name: String,
        location: Location,
        item_text: String,
    },
    Precedence {
        rules: Vec<(Location, String)>,
    },
    ReduceReduce {
        items: Vec<(Location, String)>,
    },
    IO {
        file: PathBuf,
        error: std::io::Error,
    },
}

impl LapexError {
    fn conflicts(
        file: &Path,
        contents: &str,
        conflicts: &[Conflict<1>],
        grammar: &Grammar,
    ) -> Vec<Self> {
        conflicts
            .iter()
            .map(|c| match c {
                Conflict::ShiftReduce {
                    item_to_reduce,
                    shift_symbol,
                } => {
                    let symbol_name = match shift_symbol {
                        Symbol::Terminal(token_id) => grammar.get_token_name(*token_id).to_string(),
                        _ => grammar.get_symbol_name(shift_symbol),
                    };
                    LapexError {
                        severity: Severity::Error,
                        error: LapexErrorType::ShiftReduce {
                            symbol_name,
                            location: Location::from_span(
                                item_to_reduce.production().span,
                                file,
                                contents,
                            )
                            .unwrap(),
                            item_text: format!("{}", item_to_reduce.display(grammar)),
                        },
                    }
                }
                Conflict::ReduceReduce { items } => LapexError {
                    severity: Severity::Error,
                    error: LapexErrorType::ReduceReduce {
                        items: items
                            .iter()
                            .map(|item| {
                                let item_text = format!("{}", item.display(grammar));
                                let location =
                                    Location::from_span(item.production().span, file, contents)
                                        .unwrap();
                                (location, item_text)
                            })
                            .collect(),
                    },
                },
            })
            .collect()
    }

    fn io(file: PathBuf, error: std::io::Error) -> Vec<LapexError> {
        vec![LapexError {
            severity: Severity::Error,
            error: LapexErrorType::IO { error, file },
        }]
    }

    fn precedence(file: &Path, contents: &str, error: PrecedenceError) -> Vec<LapexError> {
        vec![LapexError {
            severity: Severity::Error,
            error: LapexErrorType::Precedence {
                rules: error
                    .rules
                    .into_iter()
                    .map(|r| {
                        (
                            Location::from_span(r.span, file, contents).unwrap(),
                            r.inner,
                        )
                    })
                    .collect(),
            },
        }]
    }
}

impl LapexErrorType {
    fn message(&self) -> &'static str {
        match self {
            LapexErrorType::ShiftReduce { .. } => "shift-reduce conflict in grammar",
            LapexErrorType::ReduceReduce { .. } => "reduce-reduce conflict in grammar",
            LapexErrorType::Precedence { .. } => "conflicting token precedences in grammar",
            LapexErrorType::IO { .. } => "failed to read grammar file",
        }
    }
}

impl Display for LapexErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LapexErrorType::ShiftReduce {
                symbol_name,
                location,
                item_text,
            } => write_section(
                location,
                format_args!(
                    "Could shift token\n\t{}\nOr reduce item\n\t{}",
                    symbol_name.bold(),
                    item_text.bold()
                ),
                f,
            ),
            LapexErrorType::Precedence { rules } => {
                for (i, (location, rule)) in rules.iter().enumerate() {
                    write_section(
                        location,
                        format_args!("Token has identical precedence:\n\t{}", rule.bold()),
                        f,
                    )?;
                    if i + 1 < rules.len() {
                        writeln!(f)?;
                    }
                }
                Ok(())
            }
            LapexErrorType::ReduceReduce { items } => {
                for (i, (location, item_text)) in items.iter().enumerate() {
                    write_section(
                        location,
                        format_args!("Could reduce this item:\n\t{}", item_text.bold()),
                        f,
                    )?;
                    if i + 1 < items.len() {
                        writeln!(f)?;
                    }
                }
                Ok(())
            }
            LapexErrorType::IO { error, file } => {
                write!(f, "     file: {}\n     reason: {}", file.display(), error)
            }
        }
    }
}

impl Error for LapexError {}

fn write_section<D: Display>(
    location: &Location,
    contents: D,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(
        f,
        " {} {}:{}:{}",
        "-->".blue().bold(),
        location.file.display(),
        location.pos.line,
        location.pos.col
    )?;
    let formatted = format!(
        "{}\n{}\n\n{}",
        location.text.as_str(),
        "~".repeat(location.text.len()).bright_red().bold(),
        contents
    );
    let lines_iter_padded = std::iter::once("").chain(formatted.lines().chain(std::iter::once("")));
    let lines: Vec<String> = lines_iter_padded
        .map(|l| format!("  {}  {}", "|".blue().bold(), l))
        .collect();
    write!(f, "{}", lines.join("\n"))
}

impl Display for LapexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}: {}", self.severity, self.error.message())?;
        write!(f, "{}", self.error)
    }
}

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
) -> Result<(), Vec<LapexError>>
where
    L: LexerCodeGen,
    LR: LRParserCodeGen,
    LL: LLParserCodeGen,
    F: LanguageFactory<L, LR, LL>,
    I: LapexInputParser,
{
    let lexer_codegen = language.lexer();
    let ll_codegen = language.ll_parser();
    let lr_codegen = language.lr_parser();

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
            let parser_table = match lapex_parser::lr_parser::generate_table::<0>(&grammar) {
                Ok(val) => val,
                Err(_conflicts) => todo!(),
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
            let parser_table = match lapex_parser::lr_parser::generate_table::<1>(&grammar) {
                Ok(val) => val,
                Err(conflicts) => {
                    return Err(LapexError::conflicts(
                        grammar_path,
                        file_contents.as_str(),
                        &conflicts,
                        &grammar,
                    )
                    .into());
                }
            };
            if generate_table {
                gen.generate_code("table", |output| {
                    lapex_parser::lr_parser::output_table(&grammar, &parser_table, output)
                })
                .expect("TODO");
            }
            lr_codegen.generate_code(&grammar, &parser_table, &mut gen);
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
