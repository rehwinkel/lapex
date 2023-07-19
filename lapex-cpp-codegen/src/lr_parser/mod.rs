use lapex_parser::lr_parser::LRParserCodeGen;

use crate::CppLRParserCodeGen;

impl LRParserCodeGen for CppLRParserCodeGen {
    fn generate_code(
        &self,
        _grammar: &lapex_parser::grammar::Grammar,
        _parser_table: &lapex_parser::lr_parser::ActionGotoTable,
    ) -> lapex_codegen::GeneratedCode {
        todo!()
    }
}
