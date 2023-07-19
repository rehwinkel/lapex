use std::io::{Error, Write};
use std::path::Path;

use lapex_codegen::GeneratedCode;
use lapex_parser::grammar::{Grammar, Symbol};
use lapex_parser::ll_parser::{self, LLParserTable};
use serde::Serialize;

use crate::CppLLParserCodeGen;

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

struct CodeWriter<'parser> {
    grammar: &'parser Grammar<'parser>,
    parser_table: &'parser LLParserTable,
    template: tinytemplate::TinyTemplate<'static>,
}

impl<'parser> CodeWriter<'parser> {
    pub fn new(
        grammar: &'parser Grammar,
        parser_table: &'parser LLParserTable,
    ) -> CodeWriter<'parser> {
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
        CodeWriter {
            grammar,
            parser_table,
            template,
        }
    }

    fn write_visitor_methods<W: Write>(&self, output: &mut W) -> Result<(), Error> {
        for non_terminal in self.grammar.non_terminals() {
            if let Some(name) = self.grammar.is_named_non_terminal(non_terminal) {
                writeln!(output, "virtual void enter_{}() = 0;", name)?;
                writeln!(output, "virtual void exit_{}() = 0;", name)?;
            }
        }
        Ok(())
    }

    fn write_non_terminal_visitor_call<W: Write>(
        &self,
        is_exit: bool,
        output: &mut W,
    ) -> Result<(), Error> {
        writeln!(output, "switch (non_terminal) {{")?;
        for non_terminal in self.grammar.non_terminals() {
            if let Some(name) = self.grammar.is_named_non_terminal(non_terminal) {
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
        &self,
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
                       self. grammar.get_token_name(*terminal_index)
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
        &self,
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

    fn write_table_switch<W: Write>(&self, output: &mut W) -> Result<(), Error> {
        writeln!(output, "switch(non_terminal.identifier) {{")?;
        for non_terminal in self.grammar.non_terminals() {
            let non_terminal_index = if let Symbol::NonTerminal(i) = non_terminal {
                i
            } else {
                unreachable!()
            };
            writeln!(output, "case {}: {{", non_terminal_index)?;
            writeln!(output, "switch (lookahead) {{")?;
            for (terminal, token_name) in self.grammar.terminals_with_names() {
                let entry = self.parser_table.get_production(non_terminal, &terminal);
                if let Some(symbols) = entry {
                    writeln!(output, "case lexer::TokenType::TK_{}: {{", token_name)?;
                    self.write_push_symbol_sequence(symbols, output)?;
                    writeln!(output, "break;")?;
                    writeln!(output, "}}")?;
                }
            }
            writeln!(output, "default:")?;
            self.write_parser_table_error(
                self.grammar.get_production_name(&non_terminal),
                self.grammar
                    .terminals_with_names()
                    .filter(|(symbol, _)| {
                        self.parser_table
                            .get_production(non_terminal, symbol)
                            .is_some()
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

    fn write_visitor_header<W: Write + ?Sized>(
        &self,
        output: &mut W,
    ) -> Result<(), std::io::Error> {
        let mut visitor_methods = Vec::new();
        self.write_visitor_methods(&mut visitor_methods)?;

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

    fn write_header<W: Write + ?Sized>(&self, output: &mut W) -> Result<(), std::io::Error> {
        writeln!(
            output,
            "{}",
            self.template
                .render("parser_header", &())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
        )
    }

    fn write_impl_header<W: Write + ?Sized>(&self, output: &mut W) -> Result<(), Error> {
        let mut visitor_code = Vec::new();
        self.write_visitor_methods(&mut visitor_code)?;

        let mut enter_switch_code = Vec::new();
        let mut exit_switch_code = Vec::new();
        self.write_non_terminal_visitor_call(false, &mut enter_switch_code)?;
        self.write_non_terminal_visitor_call(true, &mut exit_switch_code)?;

        let entry_index =
            if let Symbol::NonTerminal(non_terminal_index) = self.grammar.entry_point() {
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

    fn write_impl<W: Write + ?Sized>(&self, output: &mut W) -> Result<(), Error> {
        let mut parser_table_switch = Vec::new();
        self.write_table_switch(&mut parser_table_switch)?;

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

impl ll_parser::LLParserCodeGen for CppLLParserCodeGen {
    fn generate_code(
        &self,
        grammar: &Grammar,
        parser_table: &LLParserTable,
    ) -> lapex_codegen::GeneratedCode {
        let code_writer = CodeWriter::new(grammar, parser_table);
        let mut generators = GeneratedCode::new();
        generators
            .add_generated_code(Path::new("parser.h"), |output| {
                code_writer.write_header(output)
            })
            .unwrap();
        generators
            .add_generated_code(Path::new("parser.cpp"), |output| {
                code_writer.write_impl(output)
            })
            .unwrap();
        generators
            .add_generated_code(Path::new("parser_impl.h"), |output| {
                code_writer.write_impl_header(output)
            })
            .unwrap();
        generators
            .add_generated_code(Path::new("visitor.h"), |output| {
                code_writer.write_visitor_header(output)
            })
            .unwrap();
        generators
    }
}
