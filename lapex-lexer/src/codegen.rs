use std::ops::RangeInclusive;

use lapex_automaton::Dfa;
use lapex_codegen::GeneratedCodeWriter;
use lapex_input::TokenRule;

pub trait LexerCodeGen {
    fn generate_tokens(&self, rules: &[TokenRule], gen: &mut GeneratedCodeWriter);
    fn generate_lexer(
        &self,
        rules: &[TokenRule],
        alphabet: &[RangeInclusive<u32>],
        dfa: &Dfa<Vec<&TokenRule>, usize>,
        gen: &mut GeneratedCodeWriter,
    );
}
