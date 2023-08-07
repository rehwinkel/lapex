use std::{io::Write, ops::RangeInclusive};

use lapex_automaton::{AutomatonState, Dfa};
use lapex_codegen::GeneratedCodeWriter;
use lapex_input::TokenRule;
use lapex_lexer::LexerCodeGen;
use quote::{__private::TokenStream, quote};

use crate::RustLexerCodeGen;

struct TokensCodeWriter<'grammar> {
    rules: &'grammar [TokenRule<'grammar>],
}

fn get_token_enum_name(name: &str) -> String {
    let (head, tail) = name.split_at(1);
    format!(
        "Tk{}{}",
        head.to_ascii_uppercase(),
        tail.to_ascii_lowercase()
    )
}

impl<'grammar> TokensCodeWriter<'grammar> {
    fn write_token_enum(&self, output: &mut dyn Write) -> Result<(), std::io::Error> {
        let mut other_tokens = Vec::new();
        for rule in self.rules {
            writeln!(&mut other_tokens, "{},", get_token_enum_name(rule.token()))?;
        }
        let other_tokens: TokenStream = String::from_utf8(other_tokens).unwrap().parse().unwrap();

        let tokens = quote! {
            #[derive(Clone, Copy, Debug)]
            pub enum TokenType {
                EndOfFile,
                Error,
                #other_tokens
            }
        };
        writeln!(output, "{}", tokens)
    }
}

struct LexerCodeWriter<'grammar> {
    alphabet: &'grammar [RangeInclusive<u32>],
    dfa: &'grammar Dfa<Vec<String>, usize>,
}

impl<'grammar> LexerCodeWriter<'grammar> {
    fn write_lexer(&self, output: &mut dyn Write) -> Result<(), std::io::Error> {
        let mut alphabet_cases: Vec<TokenStream> = Vec::new();
        for (i, entry) in self.alphabet.iter().enumerate() {
            let start = entry.start();
            let end = entry.end();
            if start == end {
                alphabet_cases.push(quote! { #start => Some(#i) });
            } else {
                alphabet_cases.push(quote! { #start..=#end => Some(#i) });
            }
        }

        let mut automaton_cases: Vec<TokenStream> = Vec::new();
        for (index, node) in self.dfa.states() {
            let state_id = index.index();
            if state_id == 0 {
                automaton_cases.push(quote! { (#state_id, 0) => { return TokenType::EndOfFile; } });
            }
            for (transition, target) in self.dfa.transitions_from(index) {
                if *transition != 0 {
                    let target_index = target.index();
                    automaton_cases.push(quote! {
                        (#state_id, #transition) => {
                            let next_ch = self.char_iter.next().unwrap();
                            self.position += next_ch.len_utf8();
                            state = #target_index;
                        }
                    });
                }
            }
            if let AutomatonState::Accepting(accepts) = node {
                assert!(accepts.len() == 1);
                let name: TokenStream = get_token_enum_name(accepts[0].as_str()).parse().unwrap();
                automaton_cases.push(quote! {
                    (#state_id, _) => {
                        return TokenType::#name;
                    }
                });
            } else {
                automaton_cases.push(quote! {
                    (#state_id, _) => {
                        return TokenType::Error;
                    }
                });
            }
        }

        let tokens = quote! {
            pub struct Lexer<'src> {
                src: &'src str,
                char_iter: std::iter::Peekable<std::str::Chars<'src>>,
                start: usize,
                position: usize
            }

            impl<'src> Lexer<'src> {
                pub fn new(src: &'src str) -> Self {
                    let char_iter = src.chars().peekable();
                    Lexer {
                        src,
                        char_iter,
                        start: 0,
                        position: 0
                    }
                }

                fn get_alphabet_index(c: u32) -> Option<usize> {
                    match c {
                        #( #alphabet_cases, )*
                        _ => None
                    }
                }

                pub fn next(&mut self) -> TokenType {
                    let mut state: usize = 0;
                    self.start = self.position;
                    loop {
                        let next_ch = self.char_iter.peek().copied().map(|c| c as u32).unwrap_or(0);
                        let symbol = if let Some(symbol) = Lexer::get_alphabet_index(next_ch) {
                            symbol
                        } else {
                            return TokenType::Error;
                        };
                        match (state, symbol) {
                            #( #automaton_cases, )*
                            (_, _) => unreachable!()
                        }
                    }
                }

                pub fn span(&self) -> std::ops::Range<usize> {
                    self.start..self.position
                }

                pub fn slice(&self) -> &'src str {
                    &self.src[self.span()]
                }
            }
        };
        writeln!(output, "{}", tokens)
    }
}

impl LexerCodeGen for RustLexerCodeGen {
    fn generate_lexer(
        &self,
        _rules: &[TokenRule],
        alphabet: &[RangeInclusive<u32>],
        dfa: &Dfa<Vec<String>, usize>,
        gen: &mut GeneratedCodeWriter,
    ) {
        let writer = LexerCodeWriter { alphabet, dfa };
        gen.generate_code("lexer.rs", |output| writer.write_lexer(output))
            .unwrap();
    }

    fn generate_tokens(&self, rules: &[TokenRule], gen: &mut GeneratedCodeWriter) {
        let writer = TokensCodeWriter { rules };
        gen.generate_code("tokens.rs", |output| writer.write_token_enum(output))
            .unwrap();
    }
}
