pub struct CppLexerCodeGen {
    template: tinytemplate::TinyTemplate<'static>,
}

impl Default for CppLexerCodeGen {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CppTableParserCodeGen {
    template: tinytemplate::TinyTemplate<'static>,
}

impl Default for CppTableParserCodeGen {
    fn default() -> Self {
        Self::new()
    }
}

mod lexer;
mod ll_parser;
