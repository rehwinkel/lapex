use std::ops::RangeInclusive;

use lapex_automaton::Dfa;
use lapex_codegen::GeneratedCode;
use lapex_input::TokenRule;

pub trait LexerCodeGen {
    fn generate_code(
        &self,
        rules: &[TokenRule],
        alphabet: &[RangeInclusive<u32>],
        dfa: &Dfa<Vec<String>, usize>,
    ) -> GeneratedCode;
}
