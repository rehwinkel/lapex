pub struct CppLexerCodeGen {}

impl CppLexerCodeGen {
    pub fn new() -> Self {
        CppLexerCodeGen {}
    }
}

impl Default for CppLexerCodeGen {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CppLLParserCodeGen {}

impl CppLLParserCodeGen {
    pub fn new() -> Self {
        CppLLParserCodeGen {}
    }
}

impl Default for CppLLParserCodeGen {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CppLRParserCodeGen {}

impl CppLRParserCodeGen {
    pub fn new() -> Self {
        CppLRParserCodeGen {}
    }
}

impl Default for CppLRParserCodeGen {
    fn default() -> Self {
        Self::new()
    }
}

mod lexer;
mod ll_parser;
mod lr_parser;
