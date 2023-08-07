use lapex_codegen::GeneratedCodeWriter;
use lapex_parser::lr_parser::LRParserCodeGen;

use crate::RustLRParserCodeGen;

impl LRParserCodeGen for RustLRParserCodeGen {
    fn generate_code(
        &self,
        grammar: &lapex_parser::grammar::Grammar,
        parser_table: &lapex_parser::lr_parser::ActionGotoTable,
        gen: &mut GeneratedCodeWriter,
    ) {
    }
}
