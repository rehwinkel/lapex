use std::{
    collections::HashMap,
    io::{Error, Write},
    num::NonZeroUsize,
    path::Path,
};

use lapex_codegen::GeneratedCode;
use lapex_parser::{
    grammar::{Grammar, Rule, Symbol},
    lr_parser::{ActionGotoTable, LRParserCodeGen, TableEntry},
};
use serde::Serialize;

use crate::CppLRParserCodeGen;

#[derive(Serialize)]
struct ImplHeaderContext {
    visitor_reduce_switch: String,
    entry_state: String,
    non_terminal_enum_variants: String,
}

#[derive(Serialize)]
struct ImplContext {
    action_table: String,
    goto_table: String,
    stack_reduce_table: String,
}

#[derive(Serialize)]
struct VisitorContext {
    visitor_methods: String,
}

struct CodeWriter<'parser> {
    grammar: &'parser Grammar<'parser>,
    parser_table: &'parser ActionGotoTable<'parser>,
    template: tinytemplate::TinyTemplate<'static>,
    rule_index_map: HashMap<*const Rule, NonZeroUsize>,
    rules_by_non_terminal: HashMap<Symbol, Vec<&'parser Rule>>,
}

impl<'parser> CodeWriter<'parser> {
    fn new(grammar: &'parser Grammar<'parser>, parser_table: &'parser ActionGotoTable) -> Self {
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

        let mut rules_by_non_terminal = HashMap::new();
        for rule in grammar.rules() {
            if let Some(non_terminal) = rule.lhs() {
                rules_by_non_terminal
                    .entry(non_terminal)
                    .or_insert(Vec::new())
                    .push(rule);
            }
        }
        let rule_index_map: HashMap<*const Rule, NonZeroUsize> = grammar
            .rules()
            .iter()
            .enumerate()
            .map(|(i, r)| (r as *const Rule, NonZeroUsize::new(i + 1).unwrap()))
            .collect();
        CodeWriter {
            grammar,
            parser_table,
            template,
            rule_index_map,
            rules_by_non_terminal,
        }
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

    fn write_goto_switch_cases<
        'a,
        W: Write,
        I: Iterator<Item = (Symbol, Option<&'a TableEntry<'a>>)>,
    >(
        &self,
        states: I,
        output: &mut W,
    ) -> Result<(), Error> {
        for (symbol, entry) in states {
            if let Some(entry) = entry {
                match entry {
                    TableEntry::Shift { target: _ } => {
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
                    }
                    _ => {}
                }
                match entry {
                    TableEntry::Shift { target } => {
                        writeln!(output, "return {};", target)?;
                    }
                    _ => (),
                }
            }
        }
        writeln!(output, "default:")?;
        writeln!(output, "throw std::runtime_error(\"Parsing error\"); ")?;
        Ok(())
    }

    fn write_action_switch_cases<
        'a,
        W: Write,
        I: Iterator<Item = (Symbol, Option<&'a TableEntry<'a>>)>,
    >(
        &self,
        states: I,
        output: &mut W,
    ) -> Result<(), Error> {
        for (symbol, entry) in states {
            if let Some(entry) = entry {
                match entry {
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
                    }
                    _ => {}
                }
                match entry {
                    TableEntry::Shift { target: _ } => {
                        writeln!(output, "return 0;")?;
                    }
                    TableEntry::Reduce { rule } => {
                        let rule_ptr = (*rule) as *const Rule;
                        let rule_index = self.rule_index_map.get(&rule_ptr);
                        writeln!(output, "return {};", rule_index.unwrap())?;
                    }
                    _ => (),
                }
            }
        }
        writeln!(output, "default:")?;
        writeln!(output, "throw std::runtime_error(\"Parsing error\"); ")?;
        Ok(())
    }

    fn write_goto_table<W: Write>(&self, output: &mut W) -> Result<(), std::io::Error> {
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
                self.write_goto_switch_cases(
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
                self.write_goto_switch_cases(
                    self.parser_table
                        .iter_state_non_terminals(state, self.grammar),
                    output,
                )?;
                writeln!(output, "}}")?;
                writeln!(output, "}} else {{")?;
                writeln!(output, "throw std::runtime_error(\"There was a state atop the stack when there should have been a symbol. This should never happen!\");")?;
                writeln!(output, "}}")?;
                writeln!(output, "}}")?;
                writeln!(output, "break;")?;
            }
        }
        writeln!(output, "default:")?;
        writeln!(output, "throw std::runtime_error(\"Encountered a parser state that does not exist. This should never happen!\"); ")?;
        writeln!(output, "}}")?;
        Ok(())
    }

    fn write_shift_or_reduce_table<W: Write>(&self, output: &mut W) -> Result<(), std::io::Error> {
        writeln!(output, "switch (state) {{")?;
        for state in 0..self.parser_table.states() {
            writeln!(output, "case {}: {{", state)?;
            writeln!(
                output,
                "if (lookahead_symbol.kind == SymbolKind::Terminal) {{"
            )?;
            writeln!(
                output,
                "switch (static_cast<lexer::TokenType>(lookahead_symbol.identifier)) {{"
            )?;
            self.write_action_switch_cases(
                self.parser_table.iter_state_terminals(state, self.grammar),
                output,
            )?;
            writeln!(output, "}}")?;
            writeln!(
                output,
                "}} else if (lookahead_symbol.kind == SymbolKind::NonTerminal) {{"
            )?;
            writeln!(
                output,
                "switch (static_cast<parser::NonTerminalType>(lookahead_symbol.identifier)) {{"
            )?;
            self.write_action_switch_cases(
                self.parser_table
                    .iter_state_non_terminals(state, self.grammar),
                output,
            )?;
            writeln!(output, "}}")?;
            writeln!(output, "}} else {{")?;
            writeln!(output, "throw std::runtime_error(\"There was a state atop the stack when there should have been a symbol. This should never happen!\");")?;
            writeln!(output, "}}")?;
            writeln!(output, "}}")?;
            writeln!(output, "break;")?;
        }
        writeln!(output, "default:")?;
        writeln!(output, "throw std::runtime_error(\"Encountered a parser state that does not exist. This should never happen!\"); ")?;
        writeln!(output, "}}")?;
        Ok(())
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

    fn write_visitor_methods<W: Write>(&self, output: &mut W) -> Result<(), std::io::Error> {
        for (non_terminal, rules) in &self.rules_by_non_terminal {
            let non_terminal_name = self.get_non_terminal_name(non_terminal);
            if rules.len() != 1 {
                for (i, _rule) in rules.iter().enumerate() {
                    writeln!(
                        output,
                        "virtual void reduce_{}_{}() = 0;",
                        non_terminal_name,
                        i + 1
                    )?;
                }
            } else {
                writeln!(output, "virtual void reduce_{}() = 0;", non_terminal_name)?;
            }
        }
        Ok(())
    }

    fn get_non_terminal_name(&self, non_terminal: &Symbol) -> String {
        let non_terminal_name = self
            .grammar
            .is_named_non_terminal(*non_terminal)
            .map(|s| String::from(s))
            .unwrap_or_else(|| {
                if let Symbol::NonTerminal(index) = non_terminal {
                    format!("anon{}", index)
                } else {
                    unreachable!()
                }
            });
        non_terminal_name
    }

    fn write_stack_reduce_table<W: Write>(&self, output: &mut W) -> Result<(), std::io::Error> {
        writeln!(output, "switch(rule) {{")?;
        for (rule, rule_index) in &self.rule_index_map {
            writeln!(output, "case {}: {{", rule_index)?;
            let rule = get_rule_from_pointer(rule);
            let symbols_to_reduce = rule.rhs().len();
            let is_accepting_rule = rule.lhs().unwrap() == *self.grammar.entry_point();
            writeln!(
                output,
                "for (size_t i = 0; i < {}; i++) {{",
                symbols_to_reduce
            )?;
            writeln!(output, "parse_stack.pop_back();")?;
            writeln!(output, "Symbol reduced_symbol = parse_stack.back();")?;
            writeln!(output, "rev_reduced_symbols.push_back(reduced_symbol);")?;
            writeln!(output, "parse_stack.pop_back();")?;
            writeln!(output, "}}")?;
            write!(output, "Symbol reduced_non_terminal{{SymbolKind::NonTerminal, static_cast<uint32_t>(NonTerminalType::")?;
            self.write_non_terminal_enum_name(rule.lhs().unwrap(), output)?;
            writeln!(output, ")}};")?;
            if is_accepting_rule {
                writeln!(output, "// accepting! remove state from parse stack")?;
                writeln!(output, "parse_stack.pop_back();")?;
            } else {
                writeln!(output, "parse_stack.push_back(reduced_non_terminal);")?;
            }
            writeln!(
                output,
                "return {};",
                if is_accepting_rule { "true" } else { "false" }
            )?;
            writeln!(output, "}}")?;
        }
        writeln!(output, "default:")?;
        writeln!(output, "throw std::runtime_error(\"Tried reducing non-existent rule. This should never happen!\");")?;
        writeln!(output, "}}")?;
        Ok(())
    }

    fn write_impl<W: Write + ?Sized>(&self, output: &mut W) -> Result<(), std::io::Error> {
        let mut action_table = Vec::new();
        self.write_shift_or_reduce_table(&mut action_table)?;

        let mut goto_table = Vec::new();
        self.write_goto_table(&mut goto_table)?;

        let mut stack_reduce_table = Vec::new();
        self.write_stack_reduce_table(&mut stack_reduce_table)?;

        let context = ImplContext {
            action_table: String::from_utf8(action_table).unwrap(),
            goto_table: String::from_utf8(goto_table).unwrap(),
            stack_reduce_table: String::from_utf8(stack_reduce_table).unwrap(),
        };

        writeln!(
            output,
            "{}",
            self.template
                .render("parser_impl", &context)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
        )
    }

    fn write_visitor_reduce_switch<W: Write>(&self, output: &mut W) -> Result<(), std::io::Error> {
        writeln!(output, "switch(rule) {{")?;
        for (rule, rule_index) in &self.rule_index_map {
            writeln!(output, "case {}: {{", rule_index)?;
            let rule = get_rule_from_pointer(rule);
            if let Some(non_terminal) = rule.lhs() {
                let rules_vec = self.rules_by_non_terminal.get(&non_terminal).unwrap();
                let non_terminal_name = self.get_non_terminal_name(&non_terminal);
                if rules_vec.len() == 1 {
                    writeln!(output, "visitor.reduce_{}();", &non_terminal_name)?;
                } else {
                    let rule_index_in_vec = rules_vec
                        .iter()
                        .position(|r| std::ptr::eq(*r, rule))
                        .unwrap();
                    writeln!(
                        output,
                        "visitor.reduce_{}_{}();",
                        &non_terminal_name,
                        rule_index_in_vec + 1
                    )?;
                }
            }
            writeln!(output, "return;",)?;
            writeln!(output, "}}")?;
        }
        writeln!(output, "default:")?;
        writeln!(output, "throw std::runtime_error(\"Tried reducing non-existent rule. This should never happen!\");")?;
        writeln!(output, "}}")?;
        Ok(())
    }

    fn write_impl_header<W: Write + ?Sized>(&self, output: &mut W) -> Result<(), std::io::Error> {
        let mut non_terminal_enum_variants = Vec::new();
        self.write_non_terminal_enum_variants(&mut non_terminal_enum_variants)?;

        let mut visitor_reduce_switch = Vec::new();
        self.write_visitor_reduce_switch(&mut visitor_reduce_switch)?;

        let context = ImplHeaderContext {
            non_terminal_enum_variants: String::from_utf8(non_terminal_enum_variants).unwrap(),
            visitor_reduce_switch: String::from_utf8(visitor_reduce_switch).unwrap(),
            entry_state: format!("{}", self.parser_table.entry_state()),
        };

        writeln!(
            output,
            "{}",
            self.template
                .render("parser_impl_header", &context)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
        )
    }

    fn write_visitor_header<W: Write + ?Sized>(
        &self,
        output: &mut W,
    ) -> Result<(), std::io::Error> {
        let mut visitor_methods = Vec::new();
        self.write_visitor_methods(&mut visitor_methods)?;

        let context: VisitorContext = VisitorContext {
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
}

fn get_rule_from_pointer(rule: &*const Rule) -> &Rule {
    // We created the hashmap from a known list of rules. The rule pointers are derived from the grammar rules, and the grammar outlives this struct.
    // Therefore, this operation is safe.
    let rule = unsafe { rule.as_ref() }.unwrap();
    rule
}

impl LRParserCodeGen for CppLRParserCodeGen {
    fn generate_code(
        &self,
        grammar: &lapex_parser::grammar::Grammar,
        parser_table: &lapex_parser::lr_parser::ActionGotoTable,
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
