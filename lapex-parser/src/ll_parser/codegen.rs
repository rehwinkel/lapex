use std::io::Write;

use crate::grammar::Grammar;
use crate::ll_parser::ParserTable;

pub trait ParserCodeGen {
    fn has_header(&self) -> bool;

    fn generate_header<W: Write>(
        &self,
        grammar: &Grammar,
        parser_table: &ParserTable,
        output: &mut W,
    ) -> Result<(), std::io::Error>;

    fn generate_source<W: Write>(
        &self,
        grammar: &Grammar,
        parser_table: &ParserTable,
        output: &mut W,
    ) -> Result<(), std::io::Error>;
}
