use std::io::{Error, Write};

use lapex_parser::grammar::{Grammar, Symbol};
use lapex_parser::ll_parser;
use lapex_parser::ll_parser::LLParserTable;
use serde::Serialize;

use crate::CppTableParserCodeGen;

impl CppTableParserCodeGen {
    pub fn new() -> Self {
        let mut template = tinytemplate::TinyTemplate::new();
        template.set_default_formatter(&tinytemplate::format_unescaped);
        template
            .add_template("parser_header", include_str!("parser_header.tpl"))
            .unwrap();
        template
            .add_template("parser_impl_header", include_str!("parser_impl_header.tpl"))
            .unwrap();
        template
            .add_template("parser_impl", include_str!("parser_impl.tpl"))
            .unwrap();
        template
            .add_template(
                "parser_visitor_header",
                include_str!("parser_visitor_header.tpl"),
            )
            .unwrap();
        CppTableParserCodeGen { template }
    }

    fn write_visitor_methods<W: Write>(grammar: &Grammar, output: &mut W) -> Result<(), Error> {
        for non_terminal in grammar.non_terminals() {
            if let Some(name) = grammar.is_named_non_terminal(non_terminal) {
                writeln!(output, "virtual void enter_{}() = 0;", name)?;
                writeln!(output, "virtual void exit_{}() = 0;", name)?;
            }
        }
        Ok(())
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
                        "Symbol sym{}{{SymbolKind::NonTerminal, {}}};",
                        i, non_terminal_index
                    )?;
                    writeln!(output, "parse_stack.push(sym{});", i)?;
                }
                Symbol::Terminal(terminal_index) => {
                    writeln!(
                        output,
                        "Symbol sym{}{{SymbolKind::Terminal, static_cast<uint32_t>(lexer::TokenType::TK_{})}};",
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

#[derive(Serialize)]
struct ParserHeaderTemplateContext {}

#[derive(Serialize)]
struct ParserImplHeaderTemplateContext {
    visitor_enter_switch: String,
    visitor_exit_switch: String,
    grammar_entry_non_terminal: String,
}

#[derive(Serialize)]
struct ParserImplTemplateContext {
    parser_table_switch: String,
}

#[derive(Serialize)]
struct ParserVisitorHeaderTemplateContext {
    visitor_methods: String,
}

impl ll_parser::TableParserCodeGen for CppTableParserCodeGen {
    fn has_header(&self) -> bool {
        true
    }

    fn has_impl_header(&self) -> bool {
        true
    }

    fn generate_visitor_header<W: Write>(
        &self,
        grammar: &Grammar,
        _parser_table: &LLParserTable,
        output: &mut W,
    ) -> Result<(), std::io::Error> {
        let mut visitor_methods = Vec::new();
        CppTableParserCodeGen::write_visitor_methods(grammar, &mut visitor_methods)?;

        let context = ParserVisitorHeaderTemplateContext {
            visitor_methods: String::from_utf8(visitor_methods).unwrap(),
        };
        writeln!(
            output,
            "{}",
            self.template
                .render("parser_visitor_header", &context)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
        )
    }

    fn generate_header<W: Write>(
        &self,
        _grammar: &Grammar,
        _parser_table: &LLParserTable,
        output: &mut W,
    ) -> Result<(), std::io::Error> {
        let context = ParserHeaderTemplateContext {};
        writeln!(
            output,
            "{}",
            self.template
                .render("parser_header", &context)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
        )
    }

    fn generate_impl_header<W: Write>(
        &self,
        grammar: &Grammar,
        _parser_table: &LLParserTable,
        output: &mut W,
    ) -> Result<(), Error> {
        let mut visitor_code = Vec::new();
        CppTableParserCodeGen::write_visitor_methods(grammar, &mut visitor_code)?;

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

        let context = ParserImplHeaderTemplateContext {
            visitor_enter_switch: String::from_utf8(enter_switch_code).unwrap(),
            visitor_exit_switch: String::from_utf8(exit_switch_code).unwrap(),
            grammar_entry_non_terminal: format!("{}", entry_index),
        };

        writeln!(
            output,
            "{}",
            self.template
                .render("parser_impl_header", &context)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
        )
    }

    fn generate_source<W: Write>(
        &self,
        grammar: &Grammar,
        parser_table: &LLParserTable,
        output: &mut W,
    ) -> Result<(), Error> {
        let mut parser_table_switch = Vec::new();
        CppTableParserCodeGen::write_table_switch(grammar, parser_table, &mut parser_table_switch)?;

        let context = ParserImplTemplateContext {
            parser_table_switch: String::from_utf8(parser_table_switch).unwrap(),
        };
        writeln!(
            output,
            "{}",
            self.template
                .render("parser_impl", &context)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
        )
    }
}
