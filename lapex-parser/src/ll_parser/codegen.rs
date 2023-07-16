use std::io::Write;

use crate::grammar::Grammar;
use crate::ll_parser::LLParserTable;

pub trait TableParserCodeGen {
    fn has_header(&self) -> bool;

    fn generate_header<W: Write>(
        &self,
        grammar: &Grammar,
        parser_table: &LLParserTable,
        output: &mut W,
    ) -> Result<(), std::io::Error>;

    fn generate_source<W: Write>(
        &self,
        grammar: &Grammar,
        parser_table: &LLParserTable,
        output: &mut W,
    ) -> Result<(), std::io::Error>;
}
