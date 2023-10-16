use lapex_codegen::GeneratedCodeWriter;
use lapex_parser::lr_parser::LRParserCodeGen;

use crate::CppGLRParserCodeGen;

impl LRParserCodeGen for CppGLRParserCodeGen {
    fn generate_code(
        &self,
        _grammar: &lapex_parser::grammar::Grammar,
        _parser_table: &lapex_parser::lr_parser::ActionGotoTable,
        _gen: &mut GeneratedCodeWriter,
    ) {
        todo!()
    }
}
