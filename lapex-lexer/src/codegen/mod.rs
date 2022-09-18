mod cpp;

use std::{io::Write, ops::RangeInclusive};

use lapex_input::TokenRule;
use petgraph::Graph;

use crate::dfa::DfaState;

pub trait LexerCodeGen {
    fn has_header(&self) -> bool;

    fn generate_header<W: Write>(
        &self,
        rules: &[TokenRule],
        alphabet: &[RangeInclusive<u32>],
        dfa: &Graph<DfaState, usize>,
        output: &mut W,
    ) -> Result<(), std::io::Error>;

    fn generate_source<W: Write>(
        &self,
        rules: &[TokenRule],
        alphabet: &[RangeInclusive<u32>],
        dfa: &Graph<DfaState, usize>,
        output: &mut W,
    ) -> Result<(), std::io::Error>;
}

pub use cpp::CppLexerCodeGen;
