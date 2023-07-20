use std::io::{Error, Write};

use lapex_codegen::{GeneratedCodeWriter, Template};
use lapex_parser::grammar::{Grammar, Symbol};
use lapex_parser::ll_parser::{self, LLParserTable};

use crate::CppLLParserCodeGen;

struct CodeWriter<'parser> {
    grammar: &'parser Grammar<'parser>,
    parser_table: &'parser LLParserTable,
    parser_header_template: Template<'static>,
    parser_impl_header_template: Template<'static>,
    parser_impl_template: Template<'static>,
    visitor_header_template: Template<'static>,
}

impl<'parser> CodeWriter<'parser> {
    pub fn new(
        grammar: &'parser Grammar,
        parser_table: &'parser LLParserTable,
    ) -> CodeWriter<'parser> {
        let parser_header_template = Template::new(include_str!("parser_header.tpl"));
        let parser_impl_header_template = Template::new(include_str!("parser_impl_header.tpl"));
        let parser_impl_template = Template::new(include_str!("parser_impl.tpl"));
        let visitor_header_template = Template::new(include_str!("parser_visitor_header.tpl"));
        CodeWriter {
            grammar,
            parser_table,
            parser_header_template,
            parser_impl_header_template,
            parser_impl_template,
            visitor_header_template,
        }
    }

    fn write_visitor_methods(&self, output: &mut dyn Write) -> Result<(), Error> {
        for non_terminal in self.grammar.non_terminals() {
            if let Some(name) = self.grammar.is_named_non_terminal(non_terminal) {
                writeln!(output, "virtual void enter_{}() = 0;", name)?;
                writeln!(output, "virtual void exit_{}() = 0;", name)?;
            }
        }
        Ok(())
    }

    fn write_non_terminal_visitor_call(
        &self,
        is_exit: bool,
        output: &mut dyn Write,
    ) -> Result<(), Error> {
        writeln!(output, "switch (non_terminal) {{")?;
        for non_terminal in self.grammar.non_terminals() {
            if let Some(name) = self.grammar.is_named_non_terminal(non_terminal) {
                write!(output, "case NonTerminalType::")?;
                self.write_non_terminal_enum_name(non_terminal, output)?;
                writeln!(output, ":")?;
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

    fn write_non_terminal_enum_name(
        &self,
        non_terminal: Symbol,
        output: &mut dyn Write,
    ) -> Result<(), Error> {
        if let Some(name) = self.grammar.is_named_non_terminal(non_terminal) {
            write!(output, "NT_{}", name.to_uppercase())?;
        } else {
            if let Symbol::NonTerminal(non_terminal_index) = non_terminal {
                write!(output, "NT_ANON{}", non_terminal_index)?;
            } else {
                unreachable!()
            }
        }
        Ok(())
    }

    fn write_non_terminal_enum_variants(&self, output: &mut dyn Write) -> Result<(), Error> {
        for non_terminal in self.grammar.non_terminals() {
            self.write_non_terminal_enum_name(non_terminal, output)?;
            writeln!(output, ",")?;
        }
        Ok(())
    }

    fn write_push_symbol_sequence(
        &self,
        symbols: &[Symbol],
        output: &mut dyn Write,
    ) -> Result<(), Error> {
        for (i, symbol) in symbols.iter().rev().enumerate() {
            match symbol {
                Symbol::NonTerminal(_) => {
                    write!(
                        output,
                        "Symbol sym{}{{SymbolKind::NonTerminal, static_cast<uint32_t>(NonTerminalType::",
                        i
                    )?;
                    self.write_non_terminal_enum_name(*symbol, output)?;
                    writeln!(output, ")}};")?;
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

    fn write_parser_table_error<'a, I>(
        &self,
        non_terminal_name: Option<&'a str>,
        allowed_tokens: I,
        output: &mut dyn Write,
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

    fn write_table_switch(&self, output: &mut dyn Write) -> Result<(), Error> {
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

    fn write_visitor_header(&self, output: &mut dyn Write) -> Result<(), Error> {
        let mut writer = self.visitor_header_template.writer();
        writer.substitute("visitor_methods", |w| self.write_visitor_methods(w));
        writer.write(output)
    }

    fn write_header(&self, output: &mut dyn Write) -> Result<(), Error> {
        self.parser_header_template.writer().write(output)
    }

    fn write_impl_header(&self, output: &mut dyn Write) -> Result<(), Error> {
        let entry_symbol = if let entry @ Symbol::NonTerminal(_) = self.grammar.entry_point() {
            entry
        } else {
            panic!("entry point cannot be something other than non-terminal");
        };

        let mut writer = self.parser_impl_header_template.writer();
        writer.substitute("visitor_enter_switch", |w| {
            self.write_non_terminal_visitor_call(false, w)
        });
        writer.substitute("visitor_exit_switch", |w| {
            self.write_non_terminal_visitor_call(true, w)
        });
        writer.substitute("grammar_entry_non_terminal", |w| {
            write!(w, "NonTerminalType::")?;
            self.write_non_terminal_enum_name(*entry_symbol, w)?;
            Ok(())
        });
        writer.substitute("non_terminal_enum_variants", |w| {
            self.write_non_terminal_enum_variants(w)
        });

        writer.write(output)
    }

    fn write_impl(&self, output: &mut dyn Write) -> Result<(), Error> {
        let mut writer = self.parser_impl_template.writer();
        writer.substitute("parser_table_switch", |w| self.write_table_switch(w));
        writer.write(output)
    }
}

impl ll_parser::LLParserCodeGen for CppLLParserCodeGen {
    fn generate_code(
        &self,
        grammar: &Grammar,
        parser_table: &LLParserTable,
        gen: &mut GeneratedCodeWriter,
    ) {
        let code_writer = CodeWriter::new(grammar, parser_table);
        gen.generate_code("parser.h", |output| code_writer.write_header(output))
            .unwrap();
        gen.generate_code("parser.cpp", |output| code_writer.write_impl(output))
            .unwrap();
        gen.generate_code("parser_impl.h", |output| {
            code_writer.write_impl_header(output)
        })
        .unwrap();
        gen.generate_code("visitor.h", |output| {
            code_writer.write_visitor_header(output)
        })
        .unwrap();
    }
}
