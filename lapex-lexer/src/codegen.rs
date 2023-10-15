use std::ops::RangeInclusive;

use lapex_automaton::Dfa;
use lapex_codegen::GeneratedCodeWriter;
use lapex_input::{Spanned, TokenRule};

pub trait LexerCodeGen {
    fn generate_tokens(&self, rules: &[Spanned<TokenRule>], gen: &mut GeneratedCodeWriter);
    fn generate_lexer(
        &self,
        rules: &[Spanned<TokenRule>],
        alphabet: &[RangeInclusive<u32>],
        dfa: &Dfa<&TokenRule, usize>,
        gen: &mut GeneratedCodeWriter,
    );
}
