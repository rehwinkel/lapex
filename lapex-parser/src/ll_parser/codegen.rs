use lapex_codegen::GeneratedCodeWriter;

use crate::grammar::Grammar;
use crate::ll_parser::LLParserTable;

pub trait LLParserCodeGen {
    fn generate_code(
        &self,
        grammar: &Grammar,
        parser_table: &LLParserTable,
        gen: &mut GeneratedCodeWriter,
    );
}
