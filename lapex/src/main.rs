use std::fmt::Display;
use std::path::Path;

use clap::{arg, command, Parser, ValueEnum};
use lapex_codegen::GeneratedCodeWriter;
use lapex_lexer::LexerCodeGen;
use lapex_parser::grammar::Grammar;
use lapex_parser::ll_parser::LLParserCodeGen;
use lapex_parser::lr_parser::LRParserCodeGen;

#[derive(Debug, Clone, ValueEnum)]
enum ParsingAlgorithm {
    LL1,
    LR0,
}

impl Display for ParsingAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ParsingAlgorithm::LL1 => "ll1",
                ParsingAlgorithm::LR0 => "lr0",
            }
        )
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CommandLine {
    #[arg(required = true)]
    grammar: String,
    #[arg(long, help = "Do not generate a the lexer")]
    no_lexer: bool,
    #[arg(short, long, help = "The parser algorithm to use", default_value_t = ParsingAlgorithm::LL1)]
    algorithm: ParsingAlgorithm,
    #[arg(
        long,
        help = "The target path to write the generated code to",
        default_value_t = String::from("./generated/")
    )]
    target: String,
}

fn main() {
    let cli = CommandLine::parse();
    let path = &cli.grammar;
    let target_path = Path::new(&cli.target);

    let file_contents = std::fs::read(path).unwrap();
    let rules = lapex_input::parse_lapex_file(&file_contents).unwrap();

    let cpp_codegen = lapex_cpp_codegen::CppLexerCodeGen::new();
    let mut gen = GeneratedCodeWriter::with_default(|name| {
        std::fs::File::create(target_path.join(name)).unwrap()
    });
    cpp_codegen.generate_tokens(rules.tokens(), &mut gen);

    if !cli.no_lexer {
        let alphabet = lapex_lexer::generate_alphabet(rules.tokens());
        let (nfa_entrypoint, nfa) = lapex_lexer::generate_nfa(&alphabet, rules.tokens());
        let dfa = nfa.powerset_construction(nfa_entrypoint);

        cpp_codegen.generate_lexer(rules.tokens(), &alphabet.get_ranges(), &dfa, &mut gen);
    }

    let grammar = Grammar::from_rule_set(&rules).unwrap();
    match cli.algorithm {
        ParsingAlgorithm::LL1 => {
            let parser_table = lapex_parser::ll_parser::generate_table(&grammar).unwrap();
            let cpp_codegen = lapex_cpp_codegen::CppLLParserCodeGen::new();
            cpp_codegen.generate_code(&grammar, &parser_table, &mut gen);
        }
        ParsingAlgorithm::LR0 => {
            let parser_table = lapex_parser::lr_parser::generate_table(&grammar).unwrap();
            let cpp_codegen = lapex_cpp_codegen::CppLRParserCodeGen::new();
            cpp_codegen.generate_code(&grammar, &parser_table, &mut gen);
        }
    };
}
