use lapex_codegen::GeneratedCode;

use crate::grammar::Grammar;

use super::ActionGotoTable;

pub trait LRParserCodeGen {
    fn generate_code(&self, grammar: &Grammar, parser_table: &ActionGotoTable) -> GeneratedCode;
}
