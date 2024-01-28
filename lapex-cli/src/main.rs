use std::path::Path;

use clap::{arg, command, Args, Parser, Subcommand};
use lapex::{generate, Language, ParsingAlgorithm};
use tempdir::TempDir;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CommandLine {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Generate a parser")]
    Generate(GenerateArgs),
    #[command(about = "Generate and test a parser on a source file")]
    Debug(DebugArgs),
}

#[derive(Args, Debug)]
struct GenerateArgs {
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
    #[arg(long,        help = "The target path to write the generated code to", default_value_t = String::from("./generated/"))]
    target: String,
}

#[derive(Args, Debug)]
struct DebugArgs {
    #[arg(required = true)]
    grammar: String,
    #[arg(required = true)]
    source: String,
    #[arg(short, long, help = "The parser algorithm to use", default_value_t = ParsingAlgorithm::GLR)]
    algorithm: ParsingAlgorithm,
}

fn main() {
    let cli = CommandLine::parse();
    match cli.command {
        Commands::Generate(cmd) => {
            let result = generate(
                !cmd.no_lexer,
                cmd.algorithm,
                cmd.table,
                Path::new(&cmd.grammar),
                Path::new(&cmd.target),
                cmd.language,
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
        Commands::Debug(cmd) => {
            let target_dir = TempDir::new("lapex_debug").unwrap();
            let project_path = target_dir.path().join("generated");
            let target_path = project_path.join("src");
            std::fs::create_dir_all(&target_path).unwrap();
            let source_path = Path::new(&cmd.source);
            let result = generate(
                true,
                cmd.algorithm,
                true,
                Path::new(&cmd.grammar),
                &target_path,
                Language::Rust,
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
                _ => {
                    assert!(
                        std::process::Command::new("cargo")
                            .current_dir(&project_path)
                            .arg("init")
                            .spawn()
                            .unwrap()
                            .wait()
                            .unwrap()
                            .success(),
                        "Failed to initialize cargo project"
                    );
                    std::fs::copy(source_path, project_path.join("input.txt")).unwrap();
                    std::fs::write(
                        target_path.join("main.rs"),
                        r#"
                        use lexer::Lexer;
                        use parser::{Parser, DebugVisitor};
                        use tokens::TokenType;
                        
                        mod lexer;
                        mod parser;
                        mod tokens;
                        
                        #[derive(Debug)]
                        struct DebugError;
                        impl std::error::Error for DebugError {}
                        impl std::fmt::Display for DebugError {
                            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                                write!(f, "DebugError")
                            }
                        }

                        fn main() {
                            let viz = DebugVisitor {};
                            let src = std::fs::read_to_string("input.txt").unwrap();
                            let mut lex = Lexer::new(src.as_str());
                            let mut par = Parser::new(
                                || {
                                    let tk = lex.next().unwrap();
                                    Ok::<(TokenType, ()), DebugError>((tk, ()))
                                },
                                viz,
                            );
                            par.parse().unwrap();
                        }                        
                        "#,
                    )
                    .unwrap();
                    let mut run_process = std::process::Command::new("cargo")
                        .current_dir(&project_path)
                        .arg("run")
                        .spawn()
                        .unwrap();
                    let exit_code = run_process.wait().unwrap();
                    if exit_code.success() {
                        println!("Successfully parsed {}", source_path.display());
                    } else {
                        eprintln!("Failed to parse {}", source_path.display());
                    }
                    target_dir.close().unwrap();
                }
            }
        }
    }
}
