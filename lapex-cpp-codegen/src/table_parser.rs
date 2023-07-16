use std::io::{Error, Write};

use lapex_parser::grammar::{Grammar, Symbol};
use lapex_parser::ll_parser;
use lapex_parser::ll_parser::LLParserTable;

use crate::CppTableParserCodeGen;

impl CppTableParserCodeGen {
    pub fn new() -> Self {
        CppTableParserCodeGen {}
    }

    fn write_visitor_class<W: Write>(grammar: &Grammar, output: &mut W) -> Result<(), Error> {
        writeln!(output, "template <class T>")?;
        writeln!(output, "class Visitor {{")?;
        writeln!(output, "public:")?;
        writeln!(
            output,
            "virtual void token(lexer::TokenType tk_type, T data) = 0;"
        )?;
        for non_terminal in grammar.non_terminals() {
            if let Some(name) = grammar.is_named_non_terminal(non_terminal) {
                writeln!(output, "virtual void enter_{}() = 0;", name)?;
                writeln!(output, "virtual void exit_{}() = 0;", name)?;
            }
        }
        writeln!(output, "}};")
    }

    fn write_non_terminal_visitor_call<W: Write>(
        grammar: &Grammar,
        is_exit: bool,
        output: &mut W,
    ) -> Result<(), Error> {
        writeln!(output, "switch (non_terminal) {{")?;
        for non_terminal in grammar.non_terminals() {
            if let Some(name) = grammar.is_named_non_terminal(non_terminal) {
                let index = (if let Symbol::NonTerminal(non_terminal_index) = non_terminal {
                    Some(non_terminal_index)
                } else {
                    None
                })
                .unwrap();
                writeln!(output, "case {}:", index)?;
                if is_exit {
                    writeln!(output, "visitor.exit_{}();", name)?;
                } else {
                    writeln!(output, "visitor.enter_{}();", name)?;
                }
                writeln!(output, "break;")?;
            }
        }
        writeln!(output, "}}")
    }

    fn write_push_symbol_sequence<W: Write>(
        grammar: &Grammar,
        symbols: &[Symbol],
        output: &mut W,
    ) -> Result<(), Error> {
        for (i, symbol) in symbols.iter().rev().enumerate() {
            match symbol {
                Symbol::NonTerminal(non_terminal_index) => {
                    writeln!(
                        output,
                        "Symbol sym{}{{false, false, {}}};",
                        i, non_terminal_index
                    )?;
                    writeln!(output, "parse_stack.push(sym{});", i)?;
                }
                Symbol::Terminal(terminal_index) => {
                    writeln!(
                        output,
                        "Symbol sym{}{{true, false, static_cast<uint32_t>(lexer::TokenType::TK_{})}};",
                        i,
                        grammar.get_token_name(*terminal_index)
                    )?;
                    writeln!(output, "parse_stack.push(sym{});", i)?;
                }
                Symbol::Epsilon => {
                    writeln!(output, "// epsilon; push nothing to stack")?;
                }
                _ => (), //panic!("token not yet handled: {:?}", symbol),
            }
        }
        Ok(())
    }

    fn write_parser_table_error<'a, W: Write, I>(
        non_terminal_name: Option<&'a str>,
        allowed_tokens: I,
        output: &mut W,
    ) -> Result<(), Error>
    where
        I: Iterator<Item = &'a str>,
    {
        let allowed_tokens_list = allowed_tokens.collect::<Vec<&str>>().join(", ");
        let message = if let Some(production_name) = non_terminal_name {
            format!(
                "Encountered unknown lookahead for production '{}'. Expected one of: {}",
                production_name, allowed_tokens_list
            )
        } else {
            format!(
                "Encountered unknown lookahead for anonymous production. Expected one of: {}",
                allowed_tokens_list
            )
        };
        writeln!(output, "throw std::runtime_error(\"{}\");", message)
    }

    fn write_table_switch<W: Write>(
        grammar: &Grammar,
        parser_table: &LLParserTable,
        output: &mut W,
    ) -> Result<(), Error> {
        writeln!(output, "switch(non_terminal.identifier) {{")?;
        for non_terminal in grammar.non_terminals() {
            let non_terminal_index = if let Symbol::NonTerminal(i) = non_terminal {
                i
            } else {
                unreachable!()
            };
            writeln!(output, "case {}: {{", non_terminal_index)?;
            writeln!(output, "switch (lookahead) {{")?;
            for (terminal, token_name) in grammar.terminals_with_names() {
                let entry = parser_table.get_production(non_terminal, &terminal);
                if let Some(symbols) = entry {
                    writeln!(output, "case lexer::TokenType::TK_{}: {{", token_name)?;
                    CppTableParserCodeGen::write_push_symbol_sequence(grammar, symbols, output)?;
                    writeln!(output, "break;")?;
                    writeln!(output, "}}")?;
                }
            }
            writeln!(output, "default:")?;
            CppTableParserCodeGen::write_parser_table_error(
                grammar.get_production_name(&non_terminal),
                grammar
                    .terminals_with_names()
                    .filter(|(symbol, _)| {
                        parser_table.get_production(non_terminal, symbol).is_some()
                    })
                    .map(|(_, name)| name),
                output,
            )?;
            writeln!(output, "}}")?;
            writeln!(output, "break;")?;
            writeln!(output, "}}")?;
        }
        writeln!(output, "}}")
    }
}

impl ll_parser::TableParserCodeGen for CppTableParserCodeGen {
    fn has_header(&self) -> bool {
        true
    }

    fn generate_header<W: Write>(
        &self,
        grammar: &Grammar,
        _parser_table: &LLParserTable,
        output: &mut W,
    ) -> Result<(), Error> {
        let mut visitor_code = Vec::new();
        CppTableParserCodeGen::write_visitor_class(grammar, &mut visitor_code)?;

        let mut enter_switch_code = Vec::new();
        let mut exit_switch_code = Vec::new();
        CppTableParserCodeGen::write_non_terminal_visitor_call(
            grammar,
            false,
            &mut enter_switch_code,
        )?;
        CppTableParserCodeGen::write_non_terminal_visitor_call(
            grammar,
            true,
            &mut exit_switch_code,
        )?;

        let entry_index = if let Symbol::NonTerminal(non_terminal_index) = grammar.entry_point() {
            non_terminal_index
        } else {
            panic!("entry point cannot be something other than non-terminal");
        };

        let parser_h_template = include_str!("parser.h");
        let replaced_template = parser_h_template
            .replace(
                "/*INSERT_VISITOR*/",
                std::str::from_utf8(&visitor_code).unwrap(),
            )
            .replace(
                "/*EXIT_SWITCH*/",
                std::str::from_utf8(&exit_switch_code).unwrap(),
            )
            .replace(
                "/*ENTER_SWITCH*/",
                std::str::from_utf8(&enter_switch_code).unwrap(),
            )
            .replace("/*INSERT_ENTRY*/", &format!("{}", entry_index));
        write!(output, "{}", replaced_template)
    }

    fn generate_source<W: Write>(
        &self,
        grammar: &Grammar,
        parser_table: &LLParserTable,
        output: &mut W,
    ) -> Result<(), Error> {
        let mut table_switch_code = Vec::new();
        CppTableParserCodeGen::write_table_switch(grammar, parser_table, &mut table_switch_code)?;

        let parser_cpp_template = include_str!("parser.cpp");
        let replaced_template = parser_cpp_template.replace(
            "/*INSERT_TABLE*/",
            std::str::from_utf8(&table_switch_code).unwrap(),
        );
        write!(output, "{}", replaced_template)
    }
}
