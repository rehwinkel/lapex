use std::{
    error::Error,
    fmt::Display,
    path::{Path, PathBuf},
};

use lapex_input::{SourcePos, SourceSpan};
use lapex_lexer::PrecedenceError;
use lapex_parser::{
    grammar::{Grammar, Symbol},
    lr_parser::Conflict,
};
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
enum LapexErrorType {
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
    pub fn conflicts(
        file: &Path,
        contents: &str,
        conflicts: &[Conflict],
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

    pub fn io(file: PathBuf, error: std::io::Error) -> Vec<LapexError> {
        vec![LapexError {
            severity: Severity::Error,
            error: LapexErrorType::IO { error, file },
        }]
    }

    pub fn precedence(file: &Path, contents: &str, error: PrecedenceError) -> Vec<LapexError> {
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
