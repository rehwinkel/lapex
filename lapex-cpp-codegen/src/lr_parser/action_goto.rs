use std::io::{Error, Write};

use lapex_parser::{
    grammar::{Rule, Symbol},
    lr_parser::TableEntry,
};

use super::CodeWriter;

impl<'parser> CodeWriter<'parser> {
    fn write_goto_cases<'a, I: Iterator<Item = (Symbol, Option<&'a TableEntry<'a>>)>>(
        &self,
        states: I,
        output: &mut dyn Write,
    ) -> Result<(), Error> {
        for (symbol, entry) in states {
            if let Some(entry) = entry {
                let body_needed = self.write_goto_case_header(entry, output, symbol)?;
                if body_needed {
                    self.write_goto_case_body(entry, output)?;
                }
            }
        }
        writeln!(output, "default:")?;
        writeln!(
            output,
            "// Entered state where top of stack doesn't produce a valid transition."
        )?;
        writeln!(output, "std::terminate();")?;
        Ok(())
    }

    fn write_goto_case_header(
        &self,
        entry: &TableEntry<'_>,
        output: &mut dyn Write,
        symbol: Symbol,
    ) -> Result<bool, Error> {
        Ok(match entry {
            TableEntry::Shift { target: _ } | TableEntry::Accept => {
                write!(output, "case ")?;

                if let Symbol::NonTerminal(_) = symbol {
                    write!(output, "NonTerminalType::")?;
                    self.write_non_terminal_enum_name(symbol, output)?;
                } else if let Symbol::Terminal(terminal_index) = symbol {
                    write!(
                        output,
                        "lexer::TokenType::TK_{}",
                        self.grammar.get_token_name(terminal_index)
                    )?;
                }
                writeln!(output, ":")?;
                true
            }
            _ => false,
        })
    }

    fn write_goto_case_body(
        &self,
        entry: &TableEntry<'_>,
        output: &mut dyn Write,
    ) -> Result<(), Error> {
        writeln!(output, "{{")?;
        match entry {
            TableEntry::Shift { target } => {
                writeln!(output, "Transition transition{{{}, false}};", target)?;
                writeln!(output, "return transition;")?;
            }
            TableEntry::Accept => {
                writeln!(output, "Transition transition{{0, true}};")?;
                writeln!(output, "return transition;")?;
            }
            _ => (),
        }
        writeln!(output, "}}")
    }

    fn write_action_cases<'a, I: Iterator<Item = (Symbol, Option<&'a TableEntry<'a>>)>>(
        &self,
        states: I,
        output: &mut dyn Write,
    ) -> Result<(), Error> {
        let mut expected_symbols = Vec::new();
        for (symbol, entry) in states {
            if let Some(entry) = entry {
                let entry_created = self.write_action_case_header(entry, output, symbol)?;
                if entry_created {
                    match symbol {
                        Symbol::Terminal(token_index) => expected_symbols.push(Some(token_index)),
                        Symbol::End => expected_symbols.push(None),
                        _ => (),
                    }
                    self.write_action_case_body(entry, output)?;
                }
            }
        }
        writeln!(output, "default:")?;
        let token_names: Vec<String> = expected_symbols
            .into_iter()
            .map(|tk| {
                if let Some(token_id) = tk {
                    self.grammar.get_token_name(token_id)
                } else {
                    "<EOF>"
                }
            })
            .map(|s| format!("'{}'", s))
            .collect();
        writeln!(
            output,
            "throw_unexpected_token_error(\"{}\", lookahead_token);",
            token_names.join(", ")
        )?;
        Ok(())
    }

    fn write_action_case_body(
        &self,
        entry: &TableEntry<'_>,
        output: &mut dyn Write,
    ) -> Result<(), Error> {
        writeln!(output, "{{")?;
        match entry {
            TableEntry::Shift { target: _ } => {
                writeln!(output, "Action act{{ActionType::Shift, 0}};")?;
                writeln!(output, "return act;")?;
            }
            TableEntry::Reduce { rule } => {
                let rule_ptr = (*rule) as *const Rule;
                let rule_index = self.rule_index_map.get(&rule_ptr).unwrap();
                writeln!(output, "Action act{{ActionType::Reduce, {}}};", rule_index)?;
                writeln!(output, "return act;")?;
            }
            _ => (),
        }
        writeln!(output, "}}")
    }

    fn write_action_case_header(
        &self,
        entry: &TableEntry<'_>,
        output: &mut dyn Write,
        symbol: Symbol,
    ) -> Result<bool, Error> {
        Ok(match entry {
            TableEntry::Shift { target: _ } | TableEntry::Reduce { rule: _ } => {
                write!(output, "case ")?;

                match symbol {
                    Symbol::NonTerminal(_) => {
                        write!(output, "NonTerminalType::")?;
                        self.write_non_terminal_enum_name(symbol, output)?;
                    }
                    Symbol::Terminal(terminal_index) => {
                        write!(
                            output,
                            "lexer::TokenType::TK_{}",
                            self.grammar.get_token_name(terminal_index)
                        )?;
                    }
                    Symbol::End => {
                        write!(output, "lexer::TokenType::TK_EOF",)?;
                    }
                    _ => (),
                }
                writeln!(output, ":")?;
                true
            }
            _ => false,
        })
    }

    pub fn write_goto_table(&self, output: &mut dyn Write) -> Result<(), std::io::Error> {
        writeln!(output, "switch (state) {{")?;
        for state in 0..self.parser_table.states() {
            if self.parser_table.state_has_shift(state, self.grammar) {
                writeln!(output, "case {}: {{", state)?;
                writeln!(
                    output,
                    "if (current_symbol.kind == SymbolKind::Terminal) {{"
                )?;
                writeln!(
                    output,
                    "switch (static_cast<lexer::TokenType>(current_symbol.identifier)) {{"
                )?;
                self.write_goto_cases(
                    self.parser_table.iter_state_terminals(state, self.grammar),
                    output,
                )?;
                writeln!(output, "}}")?;
                writeln!(
                    output,
                    "}} else if (current_symbol.kind == SymbolKind::NonTerminal) {{"
                )?;
                writeln!(
                    output,
                    "switch (static_cast<parser::NonTerminalType>(current_symbol.identifier)) {{"
                )?;
                self.write_goto_cases(
                    self.parser_table
                        .iter_state_non_terminals(state, self.grammar),
                    output,
                )?;
                writeln!(output, "}}")?;
                writeln!(output, "}} else {{")?;
                writeln!(
                    output,
                    "// There was a state atop the stack when there should have been a symbol."
                )?;
                writeln!(output, "std::terminate();")?;
                writeln!(output, "}}")?;
                writeln!(output, "}}")?;
                writeln!(output, "break;")?;
            }
        }
        writeln!(output, "default:")?;
        writeln!(output, "// Encountered a parser state that does not exist.")?;
        writeln!(output, "std::terminate();")?;
        writeln!(output, "}}")?;
        Ok(())
    }

    pub fn write_action_table(&self, output: &mut dyn Write) -> Result<(), std::io::Error> {
        writeln!(output, "switch (state) {{")?;
        for state in 0..self.parser_table.states() {
            writeln!(output, "case {}: {{", state)?;
            writeln!(output, "switch (lookahead_token) {{")?;
            self.write_action_cases(
                self.parser_table.iter_state_terminals(state, self.grammar),
                output,
            )?;
            writeln!(output, "}}")?;
            writeln!(output, "}}")?;
            writeln!(output, "break;")?;
        }
        writeln!(output, "default:")?;
        writeln!(output, "// Encountered a parser state that does not exist.")?;
        writeln!(output, "std::terminate();")?;
        writeln!(output, "}}")?;
        Ok(())
    }
}
