use std::{
    collections::BTreeMap,
    error::Error,
    fmt::{Debug, Display},
    num::TryFromIntError,
};

use lapex_input::{ProductionRule, RuleSet, SourceSpan, Spanned};

use crate::grammar_builder::GrammarBuilder;

#[derive(Debug, PartialEq)]
pub enum GrammarError {
    TooManyRules,
    MissingSymbol(String),
    ConflictingRules { rules: Vec<SourceSpan> },
    RuleWithTerminalLeftHandSide,
}

impl Error for GrammarError {}

impl Display for GrammarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self) // TODO
    }
}

impl From<TryFromIntError> for GrammarError {
    fn from(_: TryFromIntError) -> Self {
        GrammarError::TooManyRules
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Symbol {
    Epsilon,
    End,
    NonTerminal(u32),
    Terminal(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule<'rules> {
    lhs: Option<u32>,
    rhs: Vec<Symbol>,
    rule: &'rules Spanned<ProductionRule<'rules>>,
}

impl<'rules> Rule<'rules> {
    pub fn entry(entry_symbol: Symbol, rule: &'rules Spanned<ProductionRule<'rules>>) -> Self {
        Rule {
            lhs: None,
            rhs: vec![entry_symbol],
            rule,
        }
    }

    pub fn rule(&self) -> &'rules Spanned<ProductionRule<'rules>> {
        self.rule
    }
}

pub struct RuleDisplay<'rule, 'grammar> {
    rule: &'rule Rule<'rule>,
    grammar: &'grammar Grammar<'rule>,
}

impl<'rules> Rule<'rules> {
    pub fn new(
        lhs: Symbol,
        rhs: Vec<Symbol>,
        rule: &'rules Spanned<ProductionRule<'rules>>,
    ) -> Result<Self, GrammarError> {
        let non_terminal_index = match lhs {
            Symbol::NonTerminal(i) => Some(i),
            _ => None,
        };
        if let Some(non_terminal_index) = non_terminal_index {
            Ok(Rule {
                lhs: Some(non_terminal_index),
                rhs,
                rule,
            })
        } else {
            Err(GrammarError::RuleWithTerminalLeftHandSide)
        }
    }

    pub fn lhs(&self) -> Option<Symbol> {
        self.lhs.map(Symbol::NonTerminal)
    }

    pub fn rhs(&self) -> &Vec<Symbol> {
        &self.rhs
    }

    pub fn display<'rule, 'grammar>(
        &'rule self,
        grammar: &'grammar Grammar<'rule>,
    ) -> RuleDisplay<'rule, 'grammar> {
        RuleDisplay {
            rule: self,
            grammar: grammar,
        }
    }
}

impl<'rule, 'grammar> Display for RuleDisplay<'rule, 'grammar> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let rhs_sequence: Vec<String> = self
            .rule
            .rhs()
            .into_iter()
            .map(|s| self.grammar.get_symbol_name(s))
            .collect();
        if let Some(lhs) = &self.rule.lhs() {
            write!(
                f,
                "{} -> {}",
                self.grammar.get_symbol_name(lhs),
                rhs_sequence.join(" ")
            )
        } else {
            write!(f, "{}", rhs_sequence.join(" "))
        }
    }
}

#[derive(Debug)]
pub struct Grammar<'rules> {
    rules: Vec<Rule<'rules>>,
    anonymous_non_terminals: Vec<Symbol>,
    productions: BTreeMap<Symbol, &'rules str>,
    tokens: BTreeMap<Symbol, &'rules str>,
    entry_rule: Rule<'rules>,
    entry_symbol: Symbol,
}

impl<'rules> Grammar<'rules> {
    pub fn new(
        entry_symbol: Symbol,
        entry_rule: Rule<'rules>,
        rules: Vec<Rule<'rules>>,
        tokens: BTreeMap<Symbol, &'rules str>,
        productions: BTreeMap<Symbol, &'rules str>,
        anonymous_non_terminals: Vec<Symbol>,
    ) -> Self {
        Grammar {
            rules,
            anonymous_non_terminals,
            productions,
            tokens,
            entry_rule,
            entry_symbol,
        }
    }
}

impl<'rules> Grammar<'rules> {
    pub fn from_rule_set(rule_set: &'rules RuleSet) -> Result<Self, GrammarError> {
        GrammarBuilder::from_rule_set(rule_set)?.build()
    }

    pub fn non_terminals(&'rules self) -> impl Iterator<Item = Symbol> + 'rules {
        self.productions
            .keys()
            .chain(self.anonymous_non_terminals.iter())
            .map(|s| s.clone())
    }

    pub fn terminals(&'rules self) -> impl Iterator<Item = Symbol> + 'rules {
        self.tokens.keys().map(|s| s.clone())
    }

    pub fn symbols(&'rules self) -> impl Iterator<Item = Symbol> + 'rules {
        self.terminals().chain(self.non_terminals())
    }

    pub fn terminals_with_names(&self) -> impl Iterator<Item = (Symbol, &str)> {
        self.tokens
            .iter()
            .map(|(sym, token_rule)| (sym.clone(), *token_rule))
    }

    pub fn get_token_name(&self, index: u32) -> &str {
        self.tokens
            .get(&Symbol::Terminal(index))
            .map(|r| r)
            .unwrap()
    }

    pub fn get_production_name(&self, non_terminal: &Symbol) -> Option<&str> {
        if let Symbol::NonTerminal(_) = non_terminal {
            if let Some(rule) = self.productions.get(non_terminal) {
                Some(rule)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    pub fn entry_rule(&self) -> &Rule {
        &self.entry_rule
    }

    pub fn entry_point(&self) -> &Symbol {
        &self.entry_symbol
    }

    pub fn get_symbol_name(&self, symbol: &Symbol) -> String {
        match symbol {
            Symbol::Terminal(terminal_index) => {
                format!(
                    "{}({})",
                    self.tokens.get(&symbol).map(|r| r).unwrap(),
                    terminal_index
                )
            }
            Symbol::NonTerminal(non_terminal_index) => {
                if let Some(rule) = self.productions.get(&symbol) {
                    format!("{}({})", rule, non_terminal_index)
                } else {
                    format!("<anon>({})", non_terminal_index)
                }
            }
            Symbol::Epsilon => String::from("<eps>"),
            Symbol::End => String::from("<end>"),
        }
    }
}

impl<'rules> Display for Grammar<'rules> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Grammar (entry: {}) {{",
            self.get_symbol_name(&self.entry_symbol)
        )?;
        for rule in &self.rules {
            writeln!(f, "\t{}", rule.display(self))?;
        }
        write!(f, "}}")?;
        Ok(())
    }
}
