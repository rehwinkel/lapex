use std::io::Write;
use std::ops::RangeInclusive;

use lapex_automaton::{AutomatonState, Dfa};

use lapex_codegen::{GeneratedCodeWriter, Template};
use lapex_input::TokenRule;
use lapex_lexer::LexerCodeGen;

use crate::CppLexerCodeGen;

struct LexerCodeWriter<'lexer> {
    lexer_header_template: Template<'static>,
    lexer_impl_template: Template<'static>,
    alphabet: &'lexer [RangeInclusive<u32>],
    dfa: &'lexer Dfa<Vec<String>, usize>,
}

impl<'lexer> LexerCodeWriter<'lexer> {
    pub fn new(
        alphabet: &'lexer [RangeInclusive<u32>],
        dfa: &'lexer Dfa<Vec<String>, usize>,
    ) -> Self {
        let lexer_header_template = Template::new(include_str!("lexer.h.tpl"));
        let lexer_impl_template = Template::new(include_str!("lexer.cpp.tpl"));
        LexerCodeWriter {
            alphabet,
            dfa,
            lexer_header_template,
            lexer_impl_template,
        }
    }

    fn write_alphabet_switch<W: Write + ?Sized>(
        &self,
        output: &mut W,
    ) -> Result<(), std::io::Error> {
        writeln!(output, "uint32_t i;")?;
        writeln!(output, "switch (ch)")?;
        writeln!(output, "{{")?;
        for (i, range) in self.alphabet.iter().enumerate() {
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

    fn write_state_machine_switch(&self, output: &mut dyn Write) -> Result<(), std::io::Error> {
        writeln!(output, "switch (state)")?;
        writeln!(output, "{{")?;
        for (index, node) in self.dfa.states() {
            writeln!(output, "case {}:", index.index())?;
            writeln!(output, "switch (i)")?;
            writeln!(output, "{{")?;
            if index.index() == 0 {
                writeln!(output, "case 0: ")?;
                writeln!(output, "return TokenType::TK_EOF;")?;
            }
            for (transition, target) in self.dfa.transitions_from(index) {
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

    fn write_header(&self, output: &mut dyn Write) -> Result<(), std::io::Error> {
        self.lexer_header_template.writer().write(output)
    }

    fn write_impl(&self, output: &mut dyn Write) -> Result<(), std::io::Error> {
        let mut writer = self.lexer_impl_template.writer();
        writer.substitute("alphabet_switch", |w| self.write_alphabet_switch(w));
        writer.substitute("automaton_switch", |w| self.write_state_machine_switch(w));
        writer.write(output)
    }
}

struct TokensCodeWriter<'lexer> {
    tokens_header_template: Template<'static>,
    tokens_impl_template: Template<'static>,
    rules: &'lexer [TokenRule<'lexer>],
}

impl<'lexer> TokensCodeWriter<'lexer> {
    fn new(rules: &'lexer [TokenRule]) -> Self {
        let tokens_header_template = Template::new(include_str!("tokens.h.tpl"));
        let tokens_impl_template = Template::new(include_str!("tokens.cpp.tpl"));
        TokensCodeWriter {
            rules,
            tokens_header_template,
            tokens_impl_template,
        }
    }

    fn write_token_enum_variants(&self, output: &mut dyn Write) -> Result<(), std::io::Error> {
        for rule in self.rules {
            writeln!(output, "TK_{},", rule.token())?;
        }
        Ok(())
    }

    fn write_get_token_name_function<W: Write + ?Sized>(
        &self,
        output: &mut W,
    ) -> Result<(), std::io::Error> {
        writeln!(output, "switch (tk_type) {{")?;
        writeln!(output, "case TokenType::TK_ERR:")?;
        writeln!(output, "return \"<ERR>\";")?;
        writeln!(output, "case TokenType::TK_EOF:")?;
        writeln!(output, "return \"<EOF>\";")?;
        for rule in self.rules {
            writeln!(output, "case TokenType::TK_{}:", rule.token())?;
            writeln!(output, "return \"{}\";", rule.token())?;
        }
        writeln!(output, "default:")?;
        writeln!(output, "return nullptr;")?;
        writeln!(output, "}}")
    }

    fn write_tokens_impl(&self, output: &mut dyn Write) -> Result<(), std::io::Error> {
        let mut writer = self.tokens_impl_template.writer();
        writer.substitute("get_token_name_function", |w| {
            self.write_get_token_name_function(w)
        });
        writer.write(output)
    }

    fn write_tokens_header(&self, output: &mut dyn Write) -> Result<(), std::io::Error> {
        let mut writer = self.tokens_header_template.writer();
        writer.substitute("token_enum_variants", |w| self.write_token_enum_variants(w));
        writer.write(output)
    }
}

impl LexerCodeGen for CppLexerCodeGen {
    fn generate_lexer(
        &self,
        _rules: &[TokenRule],
        alphabet: &[RangeInclusive<u32>],
        dfa: &Dfa<Vec<String>, usize>,
        gen: &mut GeneratedCodeWriter,
    ) {
        let code_writer = LexerCodeWriter::new(alphabet, dfa);
        gen.generate_code("lexer.h", |output| code_writer.write_header(output))
            .unwrap();
        gen.generate_code("lexer.cpp", |output| code_writer.write_impl(output))
            .unwrap();
    }

    fn generate_tokens(&self, rules: &[TokenRule], gen: &mut GeneratedCodeWriter) {
        let code_writer = TokensCodeWriter::new(rules);
        gen.generate_code("tokens.h", |output| code_writer.write_tokens_header(output))
            .unwrap();
        gen.generate_code("tokens.cpp", |output| code_writer.write_tokens_impl(output))
            .unwrap();
    }
}
