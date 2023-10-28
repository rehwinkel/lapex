use std::fmt::{Display, Formatter};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct SourcePos {
    pub line: u16,
    pub col: u16,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct SourceSpan {
    pub start: SourcePos,
    pub end: SourcePos,
}

impl SourcePos {
    fn offset(&self, text: &str) -> Option<usize> {
        let mut line = 1;
        let mut col = 1;
        for (offset, ch) in text.char_indices() {
            if line == self.line && col == self.col {
                return Some(offset);
            }
            match ch {
                '\n' => {
                    line += 1;
                    col = 1;
                }
                _ => {
                    col += 1;
                }
            }
        }
        (line == self.line && col == self.col).then_some(text.len())
    }
}

impl SourceSpan {
    pub fn substring<'a>(&self, text: &'a str) -> Option<&'a str> {
        let start = self.start.offset(text)?;
        let end = self.end.offset(text)?;
        Some(&text[start..end])
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Spanned<T> {
    pub span: SourceSpan,
    pub inner: T,
}

impl<T> Spanned<T> {
    pub fn zero(inner: T) -> Self {
        Spanned {
            span: SourceSpan {
                start: SourcePos { line: 0, col: 0 },
                end: SourcePos { line: 0, col: 0 },
            },
            inner,
        }
    }
    pub fn new(span: SourceSpan, inner: T) -> Self {
        Spanned { span, inner }
    }
    pub fn between(start: SourceSpan, end: SourceSpan, inner: T) -> Self {
        Spanned {
            span: SourceSpan {
                start: start.start,
                end: end.end,
            },
            inner,
        }
    }

    pub fn map<F, V>(self, mapping: F) -> Spanned<V>
    where
        F: FnOnce(T) -> V,
    {
        Spanned {
            span: self.span,
            inner: mapping(self.inner),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Characters {
    Single(char),
    Range(char, char),
}

#[derive(Debug)]
pub enum Pattern {
    Sequence {
        elements: Vec<Pattern>,
    },
    Alternative {
        elements: Vec<Pattern>,
    },
    Repetition {
        min: u32,
        max: Option<u32>,
        inner: Box<Pattern>,
    },
    CharSet {
        chars: Vec<Characters>,
        negated: bool,
    },
    Char {
        chars: Characters,
    },
}

impl Pattern {
    pub fn from_chars(chars: &Vec<char>) -> Self {
        Pattern::Sequence {
            elements: chars
                .into_iter()
                .map(|c| Pattern::Char {
                    chars: Characters::Single(*c),
                })
                .collect(),
        }
    }

    fn precedence(&self) -> usize {
        match self {
            Pattern::Sequence { elements } => elements.iter().map(|p| p.precedence()).sum(),
            Pattern::Alternative { elements } => {
                elements.iter().map(|p| p.precedence()).min().unwrap()
            }
            Pattern::Repetition { min, max: _, inner } => *min as usize * inner.precedence(),
            Pattern::CharSet {
                chars: _,
                negated: _,
            } => 1,
            Pattern::Char { chars: _ } => 1,
        }
    }
}

#[derive(Debug)]
pub enum TokenPattern {
    Literal { characters: Vec<char> },
    Pattern { pattern: Pattern },
}

#[derive(Debug)]
pub struct TokenRule<'src> {
    pub name: &'src str,
    pub precedence: Option<u16>,
    pub pattern: TokenPattern,
}

impl<'src> TokenRule<'src> {
    pub fn precedence(&self) -> usize {
        if let Some(prec) = self.precedence {
            prec as usize
        } else {
            match &self.pattern {
                TokenPattern::Literal { characters } => characters.len() * 2,
                TokenPattern::Pattern { pattern } => pattern.precedence(),
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ProductionRule<'src> {
    pub name: &'src str,
    pub tag: Option<&'src str>,
    pub pattern: ProductionPattern<'src>,
}

#[derive(Debug)]
pub struct EntryRule<'src> {
    pub name: &'src str,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProductionPattern<'src> {
    Sequence {
        elements: Vec<ProductionPattern<'src>>,
    },
    Alternative {
        elements: Vec<ProductionPattern<'src>>,
    },
    OneOrMany {
        inner: Box<ProductionPattern<'src>>,
    },
    ZeroOrMany {
        inner: Box<ProductionPattern<'src>>,
    },
    Optional {
        inner: Box<ProductionPattern<'src>>,
    },
    Rule {
        rule_name: &'src str,
    },
    Epsilon,
}

#[derive(Debug)]
pub struct RuleSet<'src> {
    pub entry_rule: Spanned<EntryRule<'src>>,
    pub token_rules: Vec<Spanned<TokenRule<'src>>>,
    pub production_rules: Vec<Spanned<ProductionRule<'src>>>,
}

impl<'src> RuleSet<'src> {
    pub fn new(
        entry_rule: Spanned<EntryRule<'src>>,
        token_rules: Vec<Spanned<TokenRule<'src>>>,
        production_rules: Vec<Spanned<ProductionRule<'src>>>,
    ) -> Self {
        RuleSet {
            entry_rule,
            token_rules,
            production_rules,
        }
    }
}

#[derive(Debug)]
pub enum LapexParsingError {
    IncompleteParsing(String),
    NoEntryRule,
    TooManyEntryRules,
}

impl std::error::Error for LapexParsingError {}

impl Display for LapexParsingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub trait LapexInputParser {
    fn parse_lapex<'src>(&self, source: &'src str) -> Result<RuleSet<'src>, LapexParsingError>;
}
