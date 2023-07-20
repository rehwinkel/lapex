use lapex_codegen::GeneratedCodeWriter;

use crate::grammar::Grammar;

use super::ActionGotoTable;

pub trait LRParserCodeGen {
    fn generate_code(
        &self,
        grammar: &Grammar,
        parser_table: &ActionGotoTable,
        gen: &mut GeneratedCodeWriter,
    );
}
