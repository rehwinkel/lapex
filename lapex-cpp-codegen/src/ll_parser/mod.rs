use std::io::{Error, Write};
use std::path::Path;

use lapex_codegen::GeneratedCodeWriter;
use lapex_parser::grammar::{Grammar, Symbol};
use lapex_parser::ll_parser::{self, LLParserTable};
use serde::Serialize;

use crate::CppLLParserCodeGen;

#[derive(Serialize)]
struct ImplHeaderContext {
    visitor_enter_switch: String,
    visitor_exit_switch: String,
    grammar_entry_non_terminal: String,
    non_terminal_enum_variants: String,
}

#[derive(Serialize)]
struct ImplContext {
    parser_table_switch: String,
}

#[derive(Serialize)]
struct VisitorContext {
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

    fn write_non_terminal_enum_name<W: Write>(
        &self,
        non_terminal: Symbol,
        output: &mut W,
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

    fn write_non_terminal_enum_variants<W: Write>(&self, output: &mut W) -> Result<(), Error> {
        for non_terminal in self.grammar.non_terminals() {
            self.write_non_terminal_enum_name(non_terminal, output)?;
            writeln!(output, ",")?;
        }
        Ok(())
    }

    fn write_push_symbol_sequence<W: Write>(
        &self,
        symbols: &[Symbol],
        output: &mut W,
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

    fn write_visitor_header<W: Write + ?Sized>(&self, output: &mut W) -> Result<(), Error> {
        let mut visitor_methods = Vec::new();
        self.write_visitor_methods(&mut visitor_methods)?;

        let context = VisitorContext {
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

    fn write_header<W: Write + ?Sized>(&self, output: &mut W) -> Result<(), Error> {
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
        self.write_non_terminal_visitor_call(false, &mut enter_switch_code)?;

        let mut exit_switch_code = Vec::new();
        self.write_non_terminal_visitor_call(true, &mut exit_switch_code)?;

        let mut non_terminal_enum_variants = Vec::new();
        self.write_non_terminal_enum_variants(&mut non_terminal_enum_variants)?;

        let entry_symbol = if let entry @ Symbol::NonTerminal(_) = self.grammar.entry_point() {
            entry
        } else {
            panic!("entry point cannot be something other than non-terminal");
        };
        let mut entry_symbol_name = Vec::new();
        write!(entry_symbol_name, "NonTerminalType::")?;
        self.write_non_terminal_enum_name(*entry_symbol, &mut entry_symbol_name)?;

        let context = ImplHeaderContext {
            visitor_enter_switch: String::from_utf8(enter_switch_code).unwrap(),
            visitor_exit_switch: String::from_utf8(exit_switch_code).unwrap(),
            non_terminal_enum_variants: String::from_utf8(non_terminal_enum_variants).unwrap(),
            grammar_entry_non_terminal: String::from_utf8(entry_symbol_name).unwrap(),
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

        let context = ImplContext {
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
