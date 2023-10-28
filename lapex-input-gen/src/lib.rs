use std::{error::Error, fmt::Display, str::Utf8Error};

use lapex_input::{
    Characters, EntryRule, LapexInputParser, Pattern, ProductionPattern, ProductionRule, RuleSet,
    SourcePos, SourceSpan, Spanned, TokenPattern, TokenRule,
};
use parser::Parser;
use regex_syntax::hir::{Class, Hir, HirKind};
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
    span: SourceSpan,
}

#[derive(Debug)]
enum Rule<'src> {
    TokenRule(TokenRule<'src>),
    ProductionRule(ProductionRule<'src>),
    EntryRule(EntryRule<'src>),
}

#[derive(Debug)]
enum Ast<'src> {
    Token(&'src str),
    Rule(Rule<'src>),
    Tag(Option<&'src str>),
    Pattern(ProductionPattern<'src>),
    Rules(Vec<Spanned<Rule<'src>>>),
    Precedence(Option<u16>),
}

struct LapexAstVisitor<'stack, 'src> {
    stack: &'stack mut Vec<Spanned<Ast<'src>>>,
}

fn get_unescaped_chars(text: &str) -> Vec<char> {
    // TODO: remove quotes and escaping
    let mut chars: Vec<char> = text.chars().skip(1).collect();
    chars.pop();
    chars
}

#[derive(Debug)]
enum RegexConversionError {
    LazyRepetition,
    Lookaround,
    EmptyRegex,
    RegexSyntax(regex_syntax::Error),
    Utf8Conversion(std::str::Utf8Error),
    ByteClass,
}

impl From<regex_syntax::Error> for RegexConversionError {
    fn from(value: regex_syntax::Error) -> Self {
        RegexConversionError::RegexSyntax(value)
    }
}

impl From<Utf8Error> for RegexConversionError {
    fn from(value: Utf8Error) -> Self {
        RegexConversionError::Utf8Conversion(value)
    }
}

impl Error for RegexConversionError {}

impl Display for RegexConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self) // TODO
    }
}

fn make_pattern_from_hir(hir: &Hir) -> Result<Pattern, RegexConversionError> {
    Ok(match hir.kind() {
        HirKind::Empty => {
            return Err(RegexConversionError::EmptyRegex);
        }
        HirKind::Literal(lit) => {
            let chars = std::str::from_utf8(lit.0.as_ref())?;
            Pattern::Sequence {
                elements: chars
                    .chars()
                    .map(|c| Pattern::Char {
                        chars: lapex_input::Characters::Single(c),
                    })
                    .collect(),
            }
        }
        HirKind::Class(class) => match class {
            Class::Unicode(unicode) => Pattern::CharSet {
                chars: unicode
                    .iter()
                    .map(|r| Characters::Range(r.start(), r.end()))
                    .collect(),
                negated: false,
            },
            Class::Bytes(_) => return Err(RegexConversionError::ByteClass),
        },
        HirKind::Look(_) => {
            return Err(RegexConversionError::Lookaround);
        }
        HirKind::Repetition(rep) => {
            if rep.greedy == false {
                return Err(RegexConversionError::LazyRepetition);
            }
            Pattern::Repetition {
                min: rep.min,
                max: rep.max,
                inner: Box::new(make_pattern_from_hir(rep.sub.as_ref())?),
            }
        }
        HirKind::Capture(capture) => make_pattern_from_hir(capture.sub.as_ref())?,
        HirKind::Concat(inner) => Pattern::Sequence {
            elements: inner
                .iter()
                .map(|h| make_pattern_from_hir(h))
                .collect::<Result<Vec<Pattern>, RegexConversionError>>()?,
        },
        HirKind::Alternation(opts) => Pattern::Alternative {
            elements: opts
                .iter()
                .map(|h| make_pattern_from_hir(h))
                .collect::<Result<Vec<Pattern>, RegexConversionError>>()?,
        },
    })
}

fn get_regex_pattern(text: &str) -> Result<Pattern, RegexConversionError> {
    let regex_ast = regex_syntax::parse(&text[1..text.len() - 1])?;
    Ok(make_pattern_from_hir(&regex_ast)?)
}

impl<'stack, 'src> parser::Visitor<TokenData<'src>> for LapexAstVisitor<'stack, 'src> {
    fn shift(&mut self, _token: TokenType, data: TokenData<'src>) {
        self.stack
            .push(Spanned::new(data.span, Ast::Token(data.text)));
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
        let semi_span = self.stack.pop().unwrap().span;
        let rhs = if let Some(Ast::Pattern(pattern)) = self.stack.pop().map(|s| s.inner) {
            pattern
        } else {
            panic!("Stack is broken")
        };
        self.stack.pop();
        let tag = if let Some(Ast::Tag(tag)) = self.stack.pop().map(|s| s.inner) {
            tag
        } else {
            panic!("Stack is broken")
        };
        let name = if let Some(Ast::Token(name)) = self.stack.pop().map(|s| s.inner) {
            name
        } else {
            panic!("Stack is broken")
        };
        let prod_span = self.stack.pop().unwrap().span;
        self.stack.push(Spanned::between(
            prod_span,
            semi_span,
            Ast::Rule(Rule::ProductionRule(ProductionRule {
                name,
                tag,
                pattern: rhs,
            })),
        ));
    }

    fn reduce_repetition_zero(&mut self) {
        let asterisk_span = self.stack.pop().unwrap().span;
        let (prod_span, pattern) = if let Some(Spanned {
            inner: Ast::Pattern(pattern),
            span,
        }) = self.stack.pop()
        {
            (span, pattern)
        } else {
            panic!("Stack is broken")
        };
        self.stack.push(Spanned::between(
            prod_span,
            asterisk_span,
            Ast::Pattern(ProductionPattern::ZeroOrMany {
                inner: Box::new(pattern),
            }),
        ))
    }

    fn reduce_item_1(&mut self) {
        let pattern = self.stack.pop().unwrap().map(|s| {
            if let Ast::Token(name) = s {
                Ast::Pattern(ProductionPattern::Rule { rule_name: name })
            } else {
                panic!("Stack is broken")
            }
        });
        self.stack.push(pattern)
    }

    fn reduce_item_2(&mut self) {
        let end = self.stack.pop().unwrap().span;
        let pattern = if let Some(Ast::Pattern(pattern)) = self.stack.pop().map(|s| s.inner) {
            pattern
        } else {
            panic!("Stack is broken")
        };
        let start = self.stack.pop().unwrap().span;
        self.stack
            .push(Spanned::between(start, end, Ast::Pattern(pattern)))
    }

    fn reduce_concatenation_1(&mut self) {
        let (mut elements, concat_span) = match self.stack.pop() {
            Some(Spanned {
                inner: Ast::Pattern(ProductionPattern::Sequence { elements }),
                span,
            }) => (elements, span),
            Some(Spanned {
                inner: Ast::Pattern(pattern),
                span,
            }) => (vec![pattern], span),
            _ => panic!("Stack is broken"),
        };
        let (pattern, unary_span) = if let Some(Spanned {
            inner: Ast::Pattern(pattern),
            span,
        }) = self.stack.pop()
        {
            (pattern, span)
        } else {
            panic!("Stack is broken")
        };
        elements.insert(0, pattern);
        self.stack.push(Spanned::between(
            unary_span,
            concat_span,
            Ast::Pattern(ProductionPattern::Sequence { elements }),
        ))
    }

    fn reduce_concatenation_2(&mut self) {
        // NOOP
    }

    fn reduce_pattern_1(&mut self) {
        // NOOP
    }

    fn reduce_pattern_2(&mut self) {
        let span = self.stack.pop().unwrap().span;
        self.stack
            .push(Spanned::new(span, Ast::Pattern(ProductionPattern::Epsilon)));
    }

    fn reduce_token_rule(&mut self) {
        let semi_span = self.stack.pop().unwrap().span;
        let rhs = if let Some(Ast::Token(rhs)) = self.stack.pop().map(|s| s.inner) {
            rhs
        } else {
            panic!("Stack is broken")
        };
        self.stack.pop();
        let precedence = if let Some(Ast::Precedence(prec)) = self.stack.pop().map(|s| s.inner) {
            prec
        } else {
            panic!("Stack is broken")
        };
        let name = if let Some(Ast::Token(name)) = self.stack.pop().map(|s| s.inner) {
            name
        } else {
            panic!("Stack is broken")
        };
        let token_span = self.stack.pop().unwrap().span;
        let pattern = match rhs.chars().next() {
            Some('"') => TokenPattern::Literal {
                characters: get_unescaped_chars(rhs),
            },
            Some('/') => TokenPattern::Pattern {
                pattern: get_regex_pattern(rhs).unwrap(),
            },
            _ => unreachable!(),
        };
        self.stack.push(Spanned::between(
            token_span,
            semi_span,
            Ast::Rule(Rule::TokenRule(TokenRule {
                name,
                precedence,
                pattern,
            })),
        ));
    }

    fn reduce_option(&mut self) {
        let que_span = self.stack.pop().unwrap().span;
        let (pattern, span) = if let Some(Spanned {
            inner: Ast::Pattern(pattern),
            span,
        }) = self.stack.pop()
        {
            (pattern, span)
        } else {
            panic!("Stack is broken")
        };
        self.stack.push(Spanned::between(
            span,
            que_span,
            Ast::Pattern(ProductionPattern::Optional {
                inner: Box::new(pattern),
            }),
        ))
    }

    fn reduce_entry_rule(&mut self) {
        let semi_span = self.stack.pop().unwrap().span;
        let name = if let Some(Ast::Token(name)) = self.stack.pop().map(|s| s.inner) {
            name
        } else {
            panic!("Stack is broken")
        };
        let entry_span = self.stack.pop().unwrap().span;
        self.stack.push(Spanned::between(
            entry_span,
            semi_span,
            Ast::Rule(Rule::EntryRule(EntryRule { name })),
        ));
    }

    fn reduce_repetition_one(&mut self) {
        let plus_span = self.stack.pop().unwrap().span;
        let (pattern, span) = if let Some(Spanned {
            inner: Ast::Pattern(pattern),
            span,
        }) = self.stack.pop()
        {
            (pattern, span)
        } else {
            panic!("Stack is broken")
        };
        self.stack.push(Spanned::between(
            span,
            plus_span,
            Ast::Pattern(ProductionPattern::OneOrMany {
                inner: Box::new(pattern),
            }),
        ))
    }

    fn reduce_alternative_1(&mut self) {
        let (mut elements, alt_span) = match self.stack.pop() {
            Some(Spanned {
                inner: Ast::Pattern(ProductionPattern::Alternative { elements }),
                span,
            }) => (elements, span),
            Some(Spanned {
                inner: Ast::Pattern(pattern),
                span,
            }) => (vec![pattern], span),
            _ => panic!("Stack is broken"),
        };
        self.stack.pop();
        let (pattern, concat_span) = if let Some(Spanned {
            inner: Ast::Pattern(pattern),
            span,
        }) = self.stack.pop()
        {
            (pattern, span)
        } else {
            panic!("Stack is broken")
        };
        elements.push(pattern);
        self.stack.push(Spanned::between(
            concat_span,
            alt_span,
            Ast::Pattern(ProductionPattern::Alternative { elements }),
        ))
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
        let rule = if let Some(Spanned {
            inner: Ast::Rule(rule),
            span,
        }) = self.stack.pop()
        {
            Spanned::new(span, rule)
        } else {
            panic!("Stack is broken")
        };
        self.stack.push(Spanned::zero(Ast::Rules(vec![rule])))
    }

    fn reduce_rules_2(&mut self) {
        let mut rules = if let Some(Ast::Rules(rules)) = self.stack.pop().map(|s| s.inner) {
            rules
        } else {
            panic!("Stack is broken")
        };
        let rule = if let Some(Spanned {
            inner: Ast::Rule(rule),
            span,
        }) = self.stack.pop()
        {
            Spanned::new(span, rule)
        } else {
            panic!("Stack is broken")
        };
        rules.push(rule);
        self.stack.push(Spanned::zero(Ast::Rules(rules)))
    }

    fn reduce_string_or_regex_1(&mut self) {
        // NOOP
    }

    fn reduce_string_or_regex_2(&mut self) {
        // NOOP
    }

    fn reduce_precedence(&mut self) {
        let end = self.stack.pop().unwrap().span;
        let precedence: u16 = if let Some(Ast::Token(digit)) = self.stack.pop().map(|s| s.inner) {
            digit.parse().unwrap()
        } else {
            panic!("Stack is broken")
        };
        let start = self.stack.pop().unwrap().span;
        self.stack.push(Spanned::between(
            start,
            end,
            Ast::Precedence(Some(precedence)),
        ));
    }

    fn reduce_anon27_1(&mut self) {
        // NOOP
    }

    fn reduce_anon27_2(&mut self) {
        self.stack.push(Spanned::zero(Ast::Precedence(None)));
    }

    fn reduce_tag(&mut self) {
        let end_span = self.stack.pop().unwrap().span;
        let tag = if let Some(Ast::Token(name)) = self.stack.pop().map(|s| s.inner) {
            name
        } else {
            panic!("Stack is broken")
        };
        let start_span = self.stack.pop().unwrap().span;
        self.stack
            .push(Spanned::between(start_span, end_span, Ast::Tag(Some(tag))));
    }

    fn reduce_anon26_1(&mut self) {
        // NOOP
    }

    fn reduce_anon26_2(&mut self) {
        self.stack.push(Spanned::zero(Ast::Tag(None)));
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
        let mut col: u16 = 1;
        let mut line: u16 = 1;
        let token_fun = || {
            let mut next_tk = lexer.next().unwrap();
            loop {
                match next_tk {
                    TokenType::TkNewline => {
                        next_tk = lexer.next().unwrap();
                        col = 1;
                        line += 1;
                    }
                    TokenType::TkWhitespace => {
                        col += lexer.slice().len() as u16;
                        next_tk = lexer.next().unwrap();
                    }
                    _ => break,
                }
            }
            let start_line = line;
            let start_col = col;
            col += lexer.slice().len() as u16;

            let token_data = TokenData {
                text: lexer.slice(),
                span: SourceSpan {
                    start: SourcePos {
                        line: start_line,
                        col: start_col,
                    },
                    end: SourcePos { line, col },
                },
            };
            return (next_tk, token_data);
        };
        let mut parser = Parser::new(token_fun, visitor);
        parser.parse().expect("error: parsing");
        assert_eq!(stack.len(), 1);
        let rules = if let Ast::Rules(rules) = stack.pop().unwrap().inner {
            rules
        } else {
            panic!("Stack is broken")
        };
        let mut token_rules = Vec::new();
        let mut prod_rules = Vec::new();
        let mut entry_rules = Vec::new();

        for rule in rules {
            let span = rule.span;
            match rule.inner {
                Rule::TokenRule(token_rule) => token_rules.push(Spanned::new(span, token_rule)),
                Rule::ProductionRule(prod_rule) => prod_rules.push(Spanned::new(span, prod_rule)),
                Rule::EntryRule(entry_rule) => entry_rules.push(Spanned::new(span, entry_rule)),
            }
        }

        assert_eq!(entry_rules.len(), 1);
        let entry_rule = entry_rules.pop().unwrap();
        Ok(RuleSet::new(entry_rule, token_rules, prod_rules))
    }
}
