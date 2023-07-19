use lapex_codegen::GeneratedCode;

use crate::grammar::Grammar;
use crate::ll_parser::LLParserTable;

pub trait LLParserCodeGen {
    fn generate_code(&self, grammar: &Grammar, parser_table: &LLParserTable) -> GeneratedCode;
}
