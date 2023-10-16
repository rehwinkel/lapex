use lapex_codegen::GeneratedCodeWriter;
use lapex_parser::{
    grammar::Grammar,
    lr_parser::{ActionGotoTable, LRParserCodeGen},
};

use crate::RustGLRParserCodeGen;

impl LRParserCodeGen for RustGLRParserCodeGen {
    fn generate_code(
        &self,
        _grammar: &Grammar,
        _parser_table: &ActionGotoTable,
        _gen: &mut GeneratedCodeWriter,
    ) {
        todo!()
    }
}
