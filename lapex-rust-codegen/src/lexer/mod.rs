use std::ops::RangeInclusive;

use lapex_automaton::Dfa;
use lapex_codegen::GeneratedCodeWriter;
use lapex_input::TokenRule;
use lapex_lexer::LexerCodeGen;

use crate::RustLexerCodeGen;

impl LexerCodeGen for RustLexerCodeGen {
    fn generate_lexer(
        &self,
        _rules: &[TokenRule],
        alphabet: &[RangeInclusive<u32>],
        dfa: &Dfa<Vec<String>, usize>,
        gen: &mut GeneratedCodeWriter,
    ) {
    }

    fn generate_tokens(&self, rules: &[TokenRule], gen: &mut GeneratedCodeWriter) {}
}
