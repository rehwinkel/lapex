use lapex_parser::grammar::{Grammar, Symbol};

pub struct RustLexerCodeGen {}

impl RustLexerCodeGen {
    pub fn new() -> Self {
        RustLexerCodeGen {}
    }
}

impl Default for RustLexerCodeGen {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RustLLParserCodeGen {}

impl RustLLParserCodeGen {
    pub fn new() -> Self {
        RustLLParserCodeGen {}
    }
}

impl Default for RustLLParserCodeGen {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RustLRParserCodeGen {}

impl RustLRParserCodeGen {
    pub fn new() -> Self {
        RustLRParserCodeGen {}
    }
}

impl Default for RustLRParserCodeGen {
    fn default() -> Self {
        Self::new()
    }
}

fn get_token_enum_name(name: &str) -> String {
    format!("Tk{}", convert_snake_to_upper_camel(name))
}

fn get_non_terminal_enum_name(grammar: &Grammar, non_terminal: Symbol) -> String {
    if let Some(name) = grammar.get_production_name(&non_terminal) {
        format!("Nt{}", convert_snake_to_upper_camel(name))
    } else {
        if let Symbol::NonTerminal(non_terminal_index) = non_terminal {
            format!("NtAnon{}", non_terminal_index)
        } else {
            unreachable!()
        }
    }
}

fn convert_snake_to_upper_camel(name: &str) -> String {
    name.split('_')
        .map(|s| {
            let (head, tail) = s.split_at(1);
            format!("{}{}", head.to_ascii_uppercase(), tail.to_ascii_lowercase())
        })
        .collect::<Vec<String>>()
        .join("")
}

mod lexer;
mod ll_parser;
mod lr_parser;
