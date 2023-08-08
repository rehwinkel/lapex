use lapex_codegen::GeneratedCodeWriter;
use lapex_parser::ll_parser::LLParserCodeGen;

use crate::RustLLParserCodeGen;

impl LLParserCodeGen for RustLLParserCodeGen {
    fn generate_code(
        &self,
        _grammar: &lapex_parser::grammar::Grammar,
        _parser_table: &lapex_parser::ll_parser::LLParserTable,
        _gen: &mut GeneratedCodeWriter,
    ) {
        todo!()
    }
}
