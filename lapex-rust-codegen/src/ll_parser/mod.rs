use lapex_codegen::GeneratedCodeWriter;
use lapex_parser::ll_parser::LLParserCodeGen;

use crate::RustLLParserCodeGen;

impl LLParserCodeGen for RustLLParserCodeGen {
    fn generate_code(
        &self,
        grammar: &lapex_parser::grammar::Grammar,
        parser_table: &lapex_parser::ll_parser::LLParserTable,
        gen: &mut GeneratedCodeWriter,
    ) {
    }
}
