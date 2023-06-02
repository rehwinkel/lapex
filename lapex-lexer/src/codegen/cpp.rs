use std::{io::Write, ops::RangeInclusive};

use lapex_input::TokenRule;
use petgraph::{
    visit::{EdgeRef, IntoNodeReferences},
    Direction::Outgoing,
};

use crate::dfa::DfaState;

use super::LexerCodeGen;

pub struct CppLexerCodeGen {}

impl Default for CppLexerCodeGen {
    fn default() -> Self {
        Self::new()
    }
}

impl CppLexerCodeGen {
    pub fn new() -> Self {
        CppLexerCodeGen {}
    }

    fn write_token_enum<W: Write>(
        rules: &[TokenRule],
        output: &mut W,
    ) -> Result<(), std::io::Error> {
        writeln!(output, "enum class TokenType")?;
        writeln!(output, "{{")?;
        writeln!(output, "    TK_ERR = 0,")?;
        writeln!(output, "    TK_EOF = 1,")?;
        for rule in rules {
            writeln!(output, "    TK_{},", rule.token())?;
        }
        writeln!(output, "}};")?;
        Ok(())
    }

    fn write_lexer_class<W: Write>(output: &mut W) -> Result<(), std::io::Error> {
        writeln!(output, "class Lexer")?;
        writeln!(output, "{{")?;
        writeln!(output, "std::istream& in_chars;")?;
        writeln!(output, "uint32_t ch;")?;
        writeln!(output, "int err;")?;
        writeln!(output, "size_t position;")?;
        writeln!(output, "size_t start_pos;")?;
        writeln!(output, "size_t end_pos;")?;
        writeln!(output, "public:")?;
        writeln!(output, "Lexer(std::istream& in_chars);")?;
        writeln!(output, "TokenType next();")?;
        writeln!(output, "size_t start();")?;
        writeln!(output, "size_t end();")?;
        writeln!(output, "}};")?;
        Ok(())
    }

    fn write_alphabet_switch<W: Write>(
        alphabet: &[RangeInclusive<u32>],
        output: &mut W,
    ) -> Result<(), std::io::Error> {
        writeln!(output, "uint32_t i;")?;
        writeln!(output, "switch (ch)")?;
        writeln!(output, "{{")?;
        for (i, range) in alphabet.iter().enumerate() {
            if range.start() == range.end() {
                writeln!(output, "case {}:", range.start())?;
            } else {
                writeln!(output, "case {} ... {}:", range.start(), range.end())?;
            }
            writeln!(output, "i = {};", i)?;
            writeln!(output, "break;")?;
        }
        writeln!(output, "default:")?;
        writeln!(output, "return TokenType::TK_ERR;")?;
        writeln!(output, "}}")
    }

    fn write_state_machine_switch<W: Write>(
        dfa: &petgraph::Graph<DfaState, usize>,
        output: &mut W,
    ) -> Result<(), std::io::Error> {
        writeln!(output, "switch (state)")?;
        writeln!(output, "{{")?;
        for (index, node) in dfa.node_references() {
            writeln!(output, "case {}:", index.index())?;
            writeln!(output, "switch (i)")?;
            writeln!(output, "{{")?;
            if index.index() == 0 {
                writeln!(output, "case 0: ")?;
                writeln!(output, "return TokenType::TK_EOF;")?;
            }
            for edge in dfa.edges_directed(index, Outgoing) {
                if *edge.weight() != 0 {
                    writeln!(output, "case {}: ", edge.weight())?;
                    writeln!(output, "this->ch = -1;")?;
                    writeln!(output, "state = {};", edge.target().index())?;
                    writeln!(output, "break;")?;
                }
            }
            writeln!(output, "default:")?;
            if let DfaState::Accepting { accepts } = node {
                writeln!(output, "// ACCEPT: {:?}", accepts)?;
                writeln!(output, "this->end_pos = this->position;")?;
                writeln!(output, "return TokenType::TK_{};", accepts[0])?;
            } else {
                writeln!(output, "return TokenType::TK_ERR;")?;
            }
            writeln!(output, "}}")?;
            writeln!(output, "break;")?;
        }
        writeln!(output, "default:")?;
        writeln!(output, "return TokenType::TK_ERR;")?;
        writeln!(output, "}}")
    }
}

impl LexerCodeGen for CppLexerCodeGen {
    fn has_header(&self) -> bool {
        true
    }

    fn generate_header<W: Write>(
        &self,
        rules: &[TokenRule],
        _alphabet: &[RangeInclusive<u32>],
        _dfa: &petgraph::Graph<DfaState, usize>,
        output: &mut W,
    ) -> Result<(), std::io::Error> {
        writeln!(output, "#pragma once")?;
        writeln!(output)?;
        writeln!(output, "#include <istream>")?;
        writeln!(output)?;
        writeln!(output, "namespace lexer")?;
        writeln!(output, "{{")?;
        CppLexerCodeGen::write_token_enum(rules, output)?;
        writeln!(output)?;
        CppLexerCodeGen::write_lexer_class(output)?;
        writeln!(output, "}}")
    }

    fn generate_source<W: Write>(
        &self,
        _rules: &[TokenRule],
        alphabet: &[RangeInclusive<u32>],
        dfa: &petgraph::Graph<DfaState, usize>,
        output: &mut W,
    ) -> Result<(), std::io::Error> {
        let mut switch_code = Vec::new();

        CppLexerCodeGen::write_alphabet_switch(alphabet, &mut switch_code)?;
        CppLexerCodeGen::write_state_machine_switch(dfa, &mut switch_code)?;

        let lexer_cpp_template = include_str!("lexer.cpp");
        let replaced_template = lexer_cpp_template.replace(
            "/*INSERT_SWITCH*/",
            std::str::from_utf8(&switch_code).unwrap(),
        );
        write!(output, "{}", replaced_template)
    }
}
