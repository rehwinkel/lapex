pub struct CppLexerCodeGen {}

impl Default for CppLexerCodeGen {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CppTableParserCodeGen {}

impl Default for CppTableParserCodeGen {
    fn default() -> Self {
        Self::new()
    }
}

mod lexer;
mod table_parser;
