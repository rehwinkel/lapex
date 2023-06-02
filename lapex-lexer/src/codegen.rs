use std::{io::Write, ops::RangeInclusive};

use lapex_input::TokenRule;

use crate::Dfa;

pub trait LexerCodeGen {
    fn has_header(&self) -> bool;

    fn generate_header<W: Write>(
        &self,
        rules: &[TokenRule],
        alphabet: &[RangeInclusive<u32>],
        dfa: &Dfa,
        output: &mut W,
    ) -> Result<(), std::io::Error>;

    fn generate_source<W: Write>(
        &self,
        rules: &[TokenRule],
        alphabet: &[RangeInclusive<u32>],
        dfa: &Dfa,
        output: &mut W,
    ) -> Result<(), std::io::Error>;
}
