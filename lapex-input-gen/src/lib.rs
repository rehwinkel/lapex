use parser::{Parser, ParserError};
use tokens::TokenType;

mod parser {
    include!(concat!(env!("OUT_DIR"), "/generated_lapex/parser.rs"));
}
mod lexer {
    include!(concat!(env!("OUT_DIR"), "/generated_lapex/lexer.rs"));
}
mod tokens {
    include!(concat!(env!("OUT_DIR"), "/generated_lapex/tokens.rs"));
}

struct TokenData {}

struct LapexAstVisitor {}

impl parser::Visitor<TokenData> for LapexAstVisitor {
    fn shift(&mut self, token: TokenType, data: TokenData) {
        todo!()
    }

    #[doc = "unary(7) -> option(8)"]
    fn reduce_unary_1(&mut self) {
        todo!()
    }

    #[doc = "unary(7) -> repetition_one(9)"]
    fn reduce_unary_2(&mut self) {
        todo!()
    }

    #[doc = "unary(7) -> repetition_zero(10)"]
    fn reduce_unary_3(&mut self) {
        todo!()
    }

    #[doc = "unary(7) -> item(11)"]
    fn reduce_unary_4(&mut self) {
        todo!()
    }

    #[doc = "prod_rule(2) -> KW_PROD(2) IDENT(11) EQUALS(3) pattern(4)"]
    fn reduce_prod_rule(&mut self) {
        todo!()
    }

    #[doc = "repetition_zero(10) -> item(11) ASTERISK(8)"]
    fn reduce_repetition_zero(&mut self) {
        todo!()
    }

    #[doc = "item(11) -> IDENT(11)"]
    fn reduce_item_1(&mut self) {
        todo!()
    }

    #[doc = "item(11) -> LPAR(5) pattern(4) RPAR(6)"]
    fn reduce_item_2(&mut self) {
        todo!()
    }

    #[doc = "concatenation(6) -> unary(7) concatenation(6)"]
    fn reduce_concatenation_1(&mut self) {
        todo!()
    }

    #[doc = "concatenation(6) -> unary(7)"]
    fn reduce_concatenation_2(&mut self) {
        todo!()
    }

    #[doc = "pattern(4) -> alternative(5)"]
    fn reduce_pattern(&mut self) {
        todo!()
    }

    #[doc = "token_rule(3) -> KW_TOKEN(0) IDENT(11) EQUALS(3) STRING(12)"]
    fn reduce_token_rule(&mut self) {
        todo!()
    }

    #[doc = "option(8) -> item(11) QUESTION(7)"]
    fn reduce_option(&mut self) {
        todo!()
    }

    #[doc = "entry_rule(1) -> KW_ENTRY(1) IDENT(11)"]
    fn reduce_entry_rule(&mut self) {
        todo!()
    }

    #[doc = "repetition_one(9) -> item(11) PLUS(9)"]
    fn reduce_repetition_one(&mut self) {
        todo!()
    }

    #[doc = "alternative(5) -> concatenation(6) PIPE(10) concatenation(6)"]
    fn reduce_alternative_1(&mut self) {
        todo!()
    }

    #[doc = "alternative(5) -> concatenation(6)"]
    fn reduce_alternative_2(&mut self) {
        todo!()
    }

    #[doc = "rule(0) -> entry_rule(1)"]
    fn reduce_rule_1(&mut self) {
        todo!()
    }

    #[doc = "rule(0) -> prod_rule(2)"]
    fn reduce_rule_2(&mut self) {
        todo!()
    }

    #[doc = "rule(0) -> token_rule(3)"]
    fn reduce_rule_3(&mut self) {
        todo!()
    }
}

pub fn parse_lapex_file<'src>(source: &'src str) -> Result<(), ParserError> {
    let mut lexer = lexer::Lexer::new(source);
    let visitor = LapexAstVisitor {};
    let token_fun = || {
        let mut next_tk = lexer.next();
        while let TokenType::TkWhitespace = next_tk {
            next_tk = lexer.next();
        }
        return (next_tk, TokenData {});
    };
    let mut parser = Parser::new(token_fun, visitor);
    parser.parse()?;
    Ok(())
}
