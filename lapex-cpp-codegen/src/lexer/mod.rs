use std::io::Write;
use std::ops::RangeInclusive;
use std::path::Path;

use lapex_automaton::{AutomatonState, Dfa};

use lapex_codegen::GeneratedCode;
use lapex_input::TokenRule;
use lapex_lexer::LexerCodeGen;
use serde::Serialize;
use tinytemplate::TinyTemplate;

use crate::CppLexerCodeGen;

#[derive(Serialize)]
struct TokensHeaderTemplateContext {
    token_enum_variants: String,
}

#[derive(Serialize)]
struct LexerImplTemplateContext {
    alphabet_switch: String,
    automaton_switch: String,
}

#[derive(Serialize)]
struct TokensImplTemplateContext {
    get_token_name_function: String,
}

struct LexerCodeWriter<'lexer> {
    template: TinyTemplate<'static>,
    alphabet: &'lexer [RangeInclusive<u32>],
    dfa: &'lexer Dfa<Vec<String>, usize>,
}

impl<'lexer> LexerCodeWriter<'lexer> {
    pub fn new(
        alphabet: &'lexer [RangeInclusive<u32>],
        dfa: &'lexer Dfa<Vec<String>, usize>,
    ) -> Self {
        let mut template = tinytemplate::TinyTemplate::new();
        template.set_default_formatter(&tinytemplate::format_unescaped);
        template
            .add_template("lexer_header", include_str!("lexer_header.tpl"))
            .unwrap();
        template
            .add_template("lexer_impl", include_str!("lexer_impl.tpl"))
            .unwrap();
        LexerCodeWriter {
            alphabet,
            dfa,
            template,
        }
    }

    fn write_alphabet_switch<W: Write>(&self, output: &mut W) -> Result<(), std::io::Error> {
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

    fn write_state_machine_switch<W: Write>(&self, output: &mut W) -> Result<(), std::io::Error> {
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

    fn write_header<W: Write + ?Sized>(&self, output: &mut W) -> Result<(), std::io::Error> {
        writeln!(
            output,
            "{}",
            self.template
                .render("lexer_header", &())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
        )
    }

    fn write_impl<W: Write + ?Sized>(&self, output: &mut W) -> Result<(), std::io::Error> {
        let mut alphabet_switch = Vec::new();
        let mut automaton_switch = Vec::new();

        self.write_alphabet_switch(&mut alphabet_switch)?;
        self.write_state_machine_switch(&mut automaton_switch)?;

        let context = LexerImplTemplateContext {
            alphabet_switch: String::from_utf8(alphabet_switch).unwrap(),
            automaton_switch: String::from_utf8(automaton_switch).unwrap(),
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

struct TokensCodeWriter<'lexer> {
    template: TinyTemplate<'static>,
    rules: &'lexer [TokenRule<'lexer>],
}

impl<'lexer> TokensCodeWriter<'lexer> {
    fn new(rules: &'lexer [TokenRule]) -> Self {
        let mut template = tinytemplate::TinyTemplate::new();
        template.set_default_formatter(&tinytemplate::format_unescaped);
        template
            .add_template("tokens_header", include_str!("tokens_header.tpl"))
            .unwrap();
        template
            .add_template("tokens_impl", include_str!("tokens_impl.tpl"))
            .unwrap();
        TokensCodeWriter { rules, template }
    }

    fn write_token_enum_variants<W: Write>(&self, output: &mut W) -> Result<(), std::io::Error> {
        for rule in self.rules {
            writeln!(output, "TK_{},", rule.token())?;
        }
        Ok(())
    }

    fn write_get_token_name_function<W: Write>(
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

    fn write_tokens_impl<W: Write + ?Sized>(&self, output: &mut W) -> Result<(), std::io::Error> {
        let mut get_token_name_function = Vec::new();

        self.write_get_token_name_function(&mut get_token_name_function)?;
        let context = TokensImplTemplateContext {
            get_token_name_function: String::from_utf8(get_token_name_function).unwrap(),
        };

        writeln!(
            output,
            "{}",
            self.template
                .render("tokens_impl", &context)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
        )
    }

    fn write_tokens_header<W: Write + ?Sized>(&self, output: &mut W) -> Result<(), std::io::Error> {
        let mut token_enum_variants = Vec::new();
        self.write_token_enum_variants(&mut token_enum_variants)?;
        let context = TokensHeaderTemplateContext {
            token_enum_variants: String::from_utf8(token_enum_variants).unwrap(),
        };

        writeln!(
            output,
            "{}",
            self.template
                .render("tokens_header", &context)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
        )
    }
}

impl LexerCodeGen for CppLexerCodeGen {
    fn generate_lexer(
        &self,
        _rules: &[TokenRule],
        alphabet: &[RangeInclusive<u32>],
        dfa: &Dfa<Vec<String>, usize>,
    ) -> GeneratedCode {
        let code_writer = LexerCodeWriter::new(alphabet, dfa);
        let mut generators = GeneratedCode::new();
        generators
            .add_generated_code(Path::new("lexer.h"), |output| {
                code_writer.write_header(output)
            })
            .unwrap();
        generators
            .add_generated_code(Path::new("lexer.cpp"), |output| {
                code_writer.write_impl(output)
            })
            .unwrap();
        generators
    }

    fn generate_tokens(&self, rules: &[TokenRule]) -> GeneratedCode {
        let code_writer = TokensCodeWriter::new(rules);
        let mut generators = GeneratedCode::new();
        generators
            .add_generated_code(Path::new("tokens.h"), |output| {
                code_writer.write_tokens_header(output)
            })
            .unwrap();
        generators
            .add_generated_code(Path::new("tokens.cpp"), |output| {
                code_writer.write_tokens_impl(output)
            })
            .unwrap();
        generators
    }
}
