use std::io::Write;
use std::ops::RangeInclusive;

use lapex_automaton::{AutomatonState, Dfa};

use lapex_input::TokenRule;
use lapex_lexer::LexerCodeGen;
use serde::Serialize;

use crate::CppLexerCodeGen;

impl CppLexerCodeGen {
    pub fn new() -> Self {
        let mut template = tinytemplate::TinyTemplate::new();
        template.set_default_formatter(&tinytemplate::format_unescaped);
        template
            .add_template("lexer_header", include_str!("lexer_header.tpl"))
            .unwrap();
        template
            .add_template("lexer_impl", include_str!("lexer_impl.tpl"))
            .unwrap();
        CppLexerCodeGen { template }
    }

    fn write_token_enum_variants<W: Write>(
        rules: &[TokenRule],
        output: &mut W,
    ) -> Result<(), std::io::Error> {
        for rule in rules {
            writeln!(output, "TK_{},", rule.token())?;
        }
        Ok(())
    }

    fn write_get_token_name_function<W: Write>(
        rules: &[TokenRule],
        output: &mut W,
    ) -> Result<(), std::io::Error> {
        writeln!(output, "const char* get_token_name(TokenType tk_type) {{")?;
        writeln!(output, "switch (tk_type) {{")?;
        writeln!(output, "case TokenType::TK_ERR:")?;
        writeln!(output, "return \"<ERR>\";")?;
        writeln!(output, "case TokenType::TK_EOF:")?;
        writeln!(output, "return \"<EOF>\";")?;
        for rule in rules {
            writeln!(output, "case TokenType::TK_{}:", rule.token())?;
            writeln!(output, "return \"{}\";", rule.token())?;
        }
        writeln!(output, "default:")?;
        writeln!(output, "return nullptr;")?;
        writeln!(output, "}}")?;
        writeln!(output, "}}")
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
        dfa: &Dfa<Vec<String>, usize>,
        output: &mut W,
    ) -> Result<(), std::io::Error> {
        writeln!(output, "switch (state)")?;
        writeln!(output, "{{")?;
        for (index, node) in dfa.states() {
            writeln!(output, "case {}:", index.index())?;
            writeln!(output, "switch (i)")?;
            writeln!(output, "{{")?;
            if index.index() == 0 {
                writeln!(output, "case 0: ")?;
                writeln!(output, "return TokenType::TK_EOF;")?;
            }
            for (transition, target) in dfa.transitions_from(index) {
                if *transition != 0 {
                    writeln!(output, "case {}: ", transition)?;
                    writeln!(output, "this->ch = -1;")?;
                    writeln!(output, "state = {};", target.index())?;
                    writeln!(output, "break;")?;
                }
            }
            writeln!(output, "default:")?;
            if let AutomatonState::Accepting(accepts) = node {
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
        // TODO: position references code point position, not position in string/stream. This is useless.
        writeln!(output, "return TokenType::TK_ERR;")?;
        writeln!(output, "}}")
    }
}

#[derive(Serialize)]
struct LexerTemplateContext {
    token_enum_variants: String,
}

#[derive(Serialize)]
struct LexerImplTemplateContext {
    get_token_name_function: String,
    alphabet_switch: String,
    automaton_switch: String,
}

impl LexerCodeGen for CppLexerCodeGen {
    fn has_header(&self) -> bool {
        true
    }

    fn generate_header<W: Write>(
        &self,
        rules: &[TokenRule],
        _alphabet: &[RangeInclusive<u32>],
        _dfa: &Dfa<Vec<String>, usize>,
        output: &mut W,
    ) -> Result<(), std::io::Error> {
        let mut token_enum_variants = Vec::new();
        CppLexerCodeGen::write_token_enum_variants(rules, &mut token_enum_variants)?;
        let context = LexerTemplateContext {
            token_enum_variants: String::from_utf8(token_enum_variants).unwrap(),
        };

        writeln!(
            output,
            "{}",
            self.template
                .render("lexer_header", &context)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
        )
    }

    fn generate_source<W: Write>(
        &self,
        rules: &[TokenRule],
        alphabet: &[RangeInclusive<u32>],
        dfa: &Dfa<Vec<String>, usize>,
        output: &mut W,
    ) -> Result<(), std::io::Error> {
        let mut alphabet_switch = Vec::new();
        let mut automaton_switch = Vec::new();

        CppLexerCodeGen::write_alphabet_switch(alphabet, &mut alphabet_switch)?;
        CppLexerCodeGen::write_state_machine_switch(dfa, &mut automaton_switch)?;

        let mut get_token_name_function = Vec::new();
        CppLexerCodeGen::write_get_token_name_function(rules, &mut get_token_name_function)?;

        let context = LexerImplTemplateContext {
            alphabet_switch: String::from_utf8(alphabet_switch).unwrap(),
            automaton_switch: String::from_utf8(automaton_switch).unwrap(),
            get_token_name_function: String::from_utf8(get_token_name_function).unwrap(),
        };

        writeln!(
            output,
            "{}",
            self.template
                .render("lexer_impl", &context)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
        )
    }
}
