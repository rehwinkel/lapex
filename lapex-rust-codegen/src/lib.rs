pub struct RustLexerCodeGen {}

impl RustLexerCodeGen {
    pub fn new() -> Self {
        RustLexerCodeGen {}
    }
}

impl Default for RustLexerCodeGen {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RustLLParserCodeGen {}

impl RustLLParserCodeGen {
    pub fn new() -> Self {
        RustLLParserCodeGen {}
    }
}

impl Default for RustLLParserCodeGen {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RustLRParserCodeGen {}

impl RustLRParserCodeGen {
    pub fn new() -> Self {
        RustLRParserCodeGen {}
    }
}

impl Default for RustLRParserCodeGen {
    fn default() -> Self {
        Self::new()
    }
}

mod lexer;
mod ll_parser;
mod lr_parser;
