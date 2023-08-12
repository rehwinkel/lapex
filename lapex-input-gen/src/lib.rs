use std::f32::consts::E;

use lapex_input::{
    EntryRule, LapexInputParser, ProductionPattern, ProductionRule, Rule, RuleSet, TokenRule,
};
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

#[derive(Debug)]
struct TokenData<'src> {
    text: &'src str,
}

#[derive(Debug)]
enum Ast<'src> {
    Token(&'src str),
    Rule(Rule<'src>),
    Pattern(ProductionPattern<'src>),
    Rules(Vec<Rule<'src>>),
}

struct LapexAstVisitor<'stack, 'src> {
    stack: &'stack mut Vec<Ast<'src>>,
}

fn get_unescaped_chars(text: &str) -> Vec<char> {
    // TODO: remove quotes and escaping
    let mut chars: Vec<char> = text.chars().skip(1).collect();
    chars.pop();
    chars
}

impl<'stack, 'src> parser::Visitor<TokenData<'src>> for LapexAstVisitor<'stack, 'src> {
    fn shift(&mut self, _token: TokenType, data: TokenData<'src>) {
        self.stack.push(Ast::Token(data.text));
    }

    fn reduce_unary_1(&mut self) {
        // NOOP
    }

    fn reduce_unary_2(&mut self) {
        // NOOP
    }

    fn reduce_unary_3(&mut self) {
        // NOOP
    }

    fn reduce_unary_4(&mut self) {
        // NOOP
    }

    fn reduce_prod_rule(&mut self) {
        self.stack.pop();
        let rhs = if let Some(Ast::Pattern(pattern)) = self.stack.pop() {
            pattern
        } else {
            panic!("Stack is broken")
        };
        self.stack.pop();
        let name = if let Some(Ast::Token(name)) = self.stack.pop() {
            name
        } else {
            panic!("Stack is broken")
        };
        self.stack.pop();
        self.stack
            .push(Ast::Rule(Rule::ProductionRule(ProductionRule {
                name,
                pattern: rhs,
            })));
    }

    fn reduce_repetition_zero(&mut self) {
        self.stack.pop();
        let pattern = if let Some(Ast::Pattern(pattern)) = self.stack.pop() {
            pattern
        } else {
            panic!("Stack is broken")
        };
        self.stack.push(Ast::Pattern(ProductionPattern::ZeroOrMany {
            inner: Box::new(pattern),
        }))
    }

    fn reduce_item_1(&mut self) {
        let name = if let Some(Ast::Token(name)) = self.stack.pop() {
            name
        } else {
            panic!("Stack is broken")
        };
        self.stack
            .push(Ast::Pattern(ProductionPattern::Rule { rule_name: name }))
    }

    fn reduce_item_2(&mut self) {
        // NOOP
    }

    fn reduce_concatenation_1(&mut self) {
        let mut elements = match self.stack.pop() {
            Some(Ast::Pattern(ProductionPattern::Sequence { elements })) => elements,
            Some(Ast::Pattern(pattern)) => vec![pattern],
            _ => panic!("Stack is broken"),
        };
        let pattern = if let Some(Ast::Pattern(pattern)) = self.stack.pop() {
            pattern
        } else {
            panic!("Stack is broken")
        };
        elements.insert(0, pattern);
        self.stack
            .push(Ast::Pattern(ProductionPattern::Sequence { elements }))
    }

    fn reduce_concatenation_2(&mut self) {
        // NOOP
    }

    fn reduce_pattern(&mut self) {
        // NOOP
    }

    fn reduce_token_rule(&mut self) {
        self.stack.pop();
        let rhs = if let Some(Ast::Token(rhs)) = self.stack.pop() {
            rhs
        } else {
            panic!("Stack is broken")
        };
        self.stack.pop();
        let name = if let Some(Ast::Token(name)) = self.stack.pop() {
            name
        } else {
            panic!("Stack is broken")
        };
        self.stack.pop();
        match rhs.chars().next() {
            Some('"') => {
                self.stack.push(Ast::Rule(Rule::TokenRule(TokenRule {
                    name,
                    pattern: lapex_input::TokenPattern::Literal {
                        characters: get_unescaped_chars(rhs),
                    },
                })));
            }
            Some('/') => {
                self.stack.push(Ast::Rule(Rule::TokenRule(TokenRule {
                    name,
                    pattern: lapex_input::TokenPattern::Literal {
                        characters: Vec::new(), // TODO
                    },
                })));
            }
            _ => unreachable!(),
        }
    }

    fn reduce_option(&mut self) {
        self.stack.pop();
        let pattern = if let Some(Ast::Pattern(pattern)) = self.stack.pop() {
            pattern
        } else {
            panic!("Stack is broken")
        };
        self.stack.push(Ast::Pattern(ProductionPattern::Optional {
            inner: Box::new(pattern),
        }))
    }

    fn reduce_entry_rule(&mut self) {
        self.stack.pop();
        let name = if let Some(Ast::Token(name)) = self.stack.pop() {
            name
        } else {
            panic!("Stack is broken")
        };
        self.stack.pop();
        self.stack
            .push(Ast::Rule(Rule::EntryRule(EntryRule { name })));
    }

    fn reduce_repetition_one(&mut self) {
        self.stack.pop();
        let pattern = if let Some(Ast::Pattern(pattern)) = self.stack.pop() {
            pattern
        } else {
            panic!("Stack is broken")
        };
        self.stack.push(Ast::Pattern(ProductionPattern::OneOrMany {
            inner: Box::new(pattern),
        }))
    }

    fn reduce_alternative_1(&mut self) {
        let mut elements = match self.stack.pop() {
            Some(Ast::Pattern(ProductionPattern::Alternative { elements })) => elements,
            Some(Ast::Pattern(pattern)) => vec![pattern],
            _ => panic!("Stack is broken"),
        };
        self.stack.pop();
        let pattern = if let Some(Ast::Pattern(pattern)) = self.stack.pop() {
            pattern
        } else {
            panic!("Stack is broken")
        };
        elements.push(pattern);
        self.stack
            .push(Ast::Pattern(ProductionPattern::Alternative { elements }))
    }

    fn reduce_alternative_2(&mut self) {
        // NOOP
    }

    fn reduce_rule_1(&mut self) {
        // NOOP
    }

    fn reduce_rule_2(&mut self) {
        // NOOP
    }

    fn reduce_rule_3(&mut self) {
        // NOOP
    }

    fn reduce_rules_1(&mut self) {
        let rule = if let Some(Ast::Rule(rule)) = self.stack.pop() {
            rule
        } else {
            panic!("Stack is broken")
        };
        self.stack.push(Ast::Rules(vec![rule]))
    }

    fn reduce_rules_2(&mut self) {
        let mut rules = if let Some(Ast::Rules(rules)) = self.stack.pop() {
            rules
        } else {
            panic!("Stack is broken")
        };
        let rule = if let Some(Ast::Rule(rule)) = self.stack.pop() {
            rule
        } else {
            panic!("Stack is broken")
        };
        rules.push(rule);
        self.stack.push(Ast::Rules(rules))
    }

    fn reduce_string_or_regex_1(&mut self) {
        // NOOP
    }

    fn reduce_string_or_regex_2(&mut self) {
        // NOOP
    }
}

pub struct GeneratedLapexInputParser;

impl LapexInputParser for GeneratedLapexInputParser {
    fn parse_lapex<'src>(
        &self,
        source: &'src str,
    ) -> Result<lapex_input::RuleSet<'src>, lapex_input::LapexParsingError> {
        let mut lexer = lexer::Lexer::new(source);
        let mut stack = Vec::new();
        let visitor = LapexAstVisitor { stack: &mut stack };
        let token_fun = || {
            let mut next_tk = lexer.next().unwrap();
            while let TokenType::TkWhitespace = next_tk {
                next_tk = lexer.next().unwrap();
            }
            return (
                next_tk,
                TokenData {
                    text: lexer.slice(),
                },
            );
        };
        let mut parser = Parser::new(token_fun, visitor);
        parser.parse().expect("error: parsing");
        assert_eq!(stack.len(), 1);
        let rules = if let Ast::Rules(rules) = stack.pop().unwrap() {
            rules
        } else {
            panic!("Stack is broken")
        };
        let mut token_rules = Vec::new();
        let mut prod_rules = Vec::new();
        let mut entry_rules = Vec::new();

        for rule in rules {
            match rule {
                Rule::TokenRule(token_rule) => token_rules.push(token_rule),
                Rule::ProductionRule(prod_rule) => prod_rules.push(prod_rule),
                Rule::EntryRule(entry_rule) => entry_rules.push(entry_rule),
            }
        }

        assert_eq!(entry_rules.len(), 1);
        let entry_rule = entry_rules.pop().unwrap();
        Ok(RuleSet {
            entry_rule: entry_rule,
            token_rules,
            production_rules: prod_rules,
        })
    }
}
