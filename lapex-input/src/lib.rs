use std::fmt::{Display, Formatter};

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
    pub fn token(&self) -> &str {
        self.name
    }

    pub fn pattern(&self) -> &TokenPattern {
        &self.pattern
    }

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

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ProductionRule<'src> {
    pub name: &'src str,
    pub pattern: ProductionPattern<'src>,
}

impl<'src> ProductionRule<'src> {
    pub fn name(&self) -> &str {
        self.name
    }

    pub fn pattern(&self) -> &ProductionPattern {
        &self.pattern
    }
}

#[derive(Debug)]
pub struct EntryRule<'src> {
    pub name: &'src str,
}

impl<'src> EntryRule<'src> {
    pub fn new(name: &'src str) -> Self {
        EntryRule { name }
    }

    pub fn name(&self) -> &str {
        self.name
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
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
}

#[derive(Debug)]
pub enum Rule<'src> {
    TokenRule(TokenRule<'src>),
    ProductionRule(ProductionRule<'src>),
    EntryRule(EntryRule<'src>),
}

#[derive(Debug)]
pub struct RuleSet<'src> {
    pub entry_rule: EntryRule<'src>,
    pub token_rules: Vec<TokenRule<'src>>,
    pub production_rules: Vec<ProductionRule<'src>>,
}

impl<'src> RuleSet<'src> {
    pub fn entry(&self) -> &EntryRule {
        &self.entry_rule
    }
    pub fn tokens(&self) -> &[TokenRule] {
        &self.token_rules
    }
    pub fn productions(&self) -> &[ProductionRule] {
        &self.production_rules
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
