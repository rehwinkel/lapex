use std::path::Path;

use clap::{arg, command, Parser};
use lapex::{generate, Language, ParsingAlgorithm};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CommandLine {
    #[arg(required = true)]
    grammar: String,
    #[arg(long, help = "Do not generate a lexer")]
    no_lexer: bool,
    #[arg(long, help = "Output the parser table")]
    table: bool,
    #[arg(short, long, help = "The parser algorithm to use", default_value_t = ParsingAlgorithm::LL1)]
    algorithm: ParsingAlgorithm,
    #[arg(short, long, help = "The language to generate code for")]
    language: Language,
    #[arg(
        long,
        help = "The target path to write the generated code to",
        default_value_t = String::from("./generated/")
    )]
    target: String,
}

fn main() {
    let cli = CommandLine::parse();
    let result = generate(
        !cli.no_lexer,
        cli.algorithm,
        cli.table,
        Path::new(&cli.grammar),
        Path::new(&cli.target),
        cli.language,
        lapex_input_gen::GeneratedLapexInputParser {},
    );
    match result {
        Err(errors) => {
            for (i, error) in errors.iter().enumerate() {
                eprintln!("{}", error);
                if i + 1 < errors.len() {
                    eprintln!();
                }
            }
        }
        _ => {}
    }
}
