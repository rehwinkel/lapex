use std::{
    collections::HashMap,
    error::Error,
    fmt::{Debug, Display},
    hash::Hash,
    num::TryFromIntError,
};

use lapex_input::{ProductionPattern, ProductionRule, RuleSet, TokenRule};

#[derive(Debug, PartialEq)]
pub enum GrammarError {
    TooManyRules,
    MissingSymbol(String),
    ConflictingRules {
        rule_name: String,
        rule_matches: usize,
    },
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

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Symbol {
    Epsilon,
    End,
    NonTerminal(u32),
    Terminal(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    lhs: Option<u32>,
    rhs: Vec<Symbol>,
}

pub struct RuleDisplay<'rule, 'grammar> {
    rule: &'rule Rule,
    grammar: &'grammar Grammar<'rule>,
}

impl Rule {
    pub fn new(lhs: Symbol, rhs: Vec<Symbol>) -> Result<Self, GrammarError> {
        let non_terminal_index = match lhs {
            Symbol::NonTerminal(i) => Some(i),
            _ => None,
        };
        if let Some(non_terminal_index) = non_terminal_index {
            Ok(Rule {
                lhs: Some(non_terminal_index),
                rhs,
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
    rules: Vec<Rule>,
    non_terminal_mapping: HashMap<Symbol, &'rules ProductionRule<'rules>>,
    tokens: &'rules [TokenRule<'rules>],
    non_terminal_count: u32,
    entry_symbol: Symbol,
    entry_rule: Rule,
}

struct GrammarBuilder<'rules> {
    temp_count: u32,
    named_non_terminal_count: u32,
    non_terminal_mapping: HashMap<&'rules ProductionRule<'rules>, Symbol>,
    token_rules: &'rules [TokenRule<'rules>],
    production_rules: &'rules [ProductionRule<'rules>],
    rules: Vec<Rule>,
}

impl<'rules> GrammarBuilder<'rules> {
    fn new(token_rules: &'rules [TokenRule], production_rules: &'rules [ProductionRule]) -> Self {
        GrammarBuilder {
            temp_count: 0,
            named_non_terminal_count: 0,
            non_terminal_mapping: HashMap::new(),
            token_rules,
            production_rules,
            rules: Vec::new(),
        }
    }

    fn get_temp_symbol(&mut self) -> Result<Symbol, GrammarError> {
        let non_terminal = Symbol::NonTerminal(self.temp_count + self.named_non_terminal_count);
        self.temp_count += 1;
        Ok(non_terminal)
    }

    fn get_symbol_by_name(&mut self, symbol_name: &str) -> Result<Symbol, GrammarError> {
        let matching_tokens: Vec<usize> = self
            .token_rules
            .iter()
            .enumerate()
            .filter(|(_, token)| token.token() == symbol_name)
            .map(|(i, _)| i)
            .collect();

        let matching_prods: Vec<&ProductionRule> = self
            .production_rules
            .iter()
            .filter(|token| token.name() == symbol_name)
            .collect();
        let match_count = matching_tokens.len() + matching_prods.len();
        if match_count == 0 {
            Err(GrammarError::MissingSymbol(symbol_name.to_string()))
        } else {
            if let Some(token) = matching_tokens.first() {
                Ok(Symbol::Terminal(u32::try_from(*token)?))
            } else {
                let first_prod_rule = self
                    .non_terminal_mapping
                    .get(*matching_prods.first().unwrap());
                let symbols_are_equal = matching_prods
                    .iter()
                    .map(|pr| self.non_terminal_mapping.get(pr))
                    .all(|s| s == first_prod_rule);
                if !symbols_are_equal {
                    return Err(GrammarError::ConflictingRules {
                        rule_name: symbol_name.to_string(),
                        rule_matches: match_count,
                    });
                }
                if let Some(nonterminal) = first_prod_rule {
                    Ok(*nonterminal)
                } else {
                    let nonterminal =
                        Symbol::NonTerminal(self.temp_count + self.named_non_terminal_count);
                    for prod_rule in matching_prods {
                        self.non_terminal_mapping.insert(prod_rule, nonterminal);
                    }
                    self.named_non_terminal_count += 1;
                    Ok(nonterminal)
                }
            }
        }
    }
}

impl<'rules> Grammar<'rules> {
    pub fn from_rule_set(rule_set: &'rules RuleSet) -> Result<Self, GrammarError> {
        let mut grammar_builder = GrammarBuilder::new(rule_set.tokens(), rule_set.productions());
        for prod_rule in rule_set.productions() {
            grammar_builder.add_production_rule(prod_rule)?;
        }

        let entry_symbol = grammar_builder.get_symbol_by_name(rule_set.entry().name())?;
        let non_terminal_mapping: HashMap<Symbol, &ProductionRule> = grammar_builder
            .non_terminal_mapping
            .into_iter()
            .map(|(a, b)| (b, a))
            .collect();
        let non_terminal_count =
            grammar_builder.temp_count + grammar_builder.named_non_terminal_count;
        // the entry rule is a pseudo-rule that has no LHS and maps to the entry symbol.
        let entry_rule = Rule {
            lhs: None,
            rhs: vec![entry_symbol],
        };
        Ok(Grammar {
            rules: grammar_builder.rules,
            tokens: rule_set.tokens(),
            non_terminal_mapping,
            non_terminal_count,
            entry_symbol,
            entry_rule,
        })
    }

    pub fn non_terminals(&self) -> impl Iterator<Item = Symbol> {
        (0..self.non_terminal_count).map(|i| Symbol::NonTerminal(i as u32))
    }

    pub fn terminals(&self) -> impl Iterator<Item = Symbol> {
        (0..self.tokens.len()).map(|i| Symbol::Terminal(i as u32))
    }

    pub fn symbols(&self) -> impl Iterator<Item = Symbol> {
        self.terminals().chain(self.non_terminals())
    }

    pub fn terminals_with_names(&self) -> impl Iterator<Item = (Symbol, &str)> {
        self.tokens
            .iter()
            .enumerate()
            .map(|(i, token_rule)| (Symbol::Terminal(i as u32), token_rule.token()))
    }

    pub fn get_token_name(&self, index: u32) -> &str {
        self.tokens[index as usize].token()
    }

    pub fn get_production_name(&self, non_terminal: &Symbol) -> Option<&str> {
        if let Symbol::NonTerminal(_) = non_terminal {
            if let Some(rule) = self.non_terminal_mapping.get(non_terminal) {
                Some(rule.name())
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

    pub fn is_named_non_terminal(&self, symbol: Symbol) -> Option<&str> {
        match symbol {
            Symbol::NonTerminal(_) => {
                if let Some(rule) = self.non_terminal_mapping.get(&symbol) {
                    Some(rule.name())
                } else {
                    None
                }
            }
            Symbol::Terminal(_) | Symbol::Epsilon | Symbol::End => None,
        }
    }

    pub fn get_symbol_name(&self, symbol: &Symbol) -> String {
        match symbol {
            Symbol::Terminal(terminal_index) => {
                format!(
                    "{}({})",
                    self.tokens[*terminal_index as usize].token(),
                    terminal_index
                )
            }
            Symbol::NonTerminal(non_terminal_index) => {
                if let Some(rule) = self.non_terminal_mapping.get(&symbol) {
                    format!("{}({})", rule.name(), non_terminal_index)
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

impl<'rules> GrammarBuilder<'rules> {
    fn add_production_rule(
        &mut self,
        prod_rule: &'rules ProductionRule<'rules>,
    ) -> Result<(), GrammarError> {
        let symbol = self.get_symbol_by_name(prod_rule.name())?;
        let produces = self.transform_pattern(prod_rule.pattern())?;
        self.rules.push(Rule::new(symbol, produces)?);
        Ok(())
    }

    fn transform_pattern(
        &mut self,
        pattern: &ProductionPattern,
    ) -> Result<Vec<Symbol>, GrammarError> {
        match pattern {
            ProductionPattern::Sequence { elements } => {
                let symbols: Result<Vec<Vec<Symbol>>, GrammarError> = elements
                    .into_iter()
                    .map(|pattern| self.transform_pattern(pattern))
                    .collect();
                let symbols: Vec<Symbol> = symbols?.into_iter().flat_map(|v| v).collect();
                Ok(symbols)
            }
            ProductionPattern::Alternative { elements } => {
                let alt_symbol = self.get_temp_symbol()?;
                for elem in elements {
                    let inner_produces = self.transform_pattern(elem)?;
                    self.rules.push(Rule::new(alt_symbol, inner_produces)?);
                }
                Ok(vec![alt_symbol])
            }
            ProductionPattern::OneOrMany { inner } => {
                let rep_symbol = self.get_temp_symbol()?;
                let mut inner_produces = self.transform_pattern(inner)?;
                self.rules
                    .push(Rule::new(rep_symbol, inner_produces.clone())?);
                inner_produces.push(rep_symbol);
                self.rules.push(Rule::new(rep_symbol, inner_produces)?);
                Ok(vec![rep_symbol])
            }
            ProductionPattern::ZeroOrMany { inner } => {
                let rep_symbol = self.get_temp_symbol()?;
                let mut inner_produces = self.transform_pattern(inner)?;
                inner_produces.push(rep_symbol);
                self.rules
                    .push(Rule::new(rep_symbol, vec![Symbol::Epsilon])?);
                self.rules.push(Rule::new(rep_symbol, inner_produces)?);
                Ok(vec![rep_symbol])
            }
            ProductionPattern::Optional { inner } => {
                let symbol = self.get_temp_symbol()?;
                let inner_produces = self.transform_pattern(inner)?;
                self.rules.push(Rule::new(symbol, inner_produces)?);
                self.rules.push(Rule::new(symbol, vec![Symbol::Epsilon])?);
                Ok(vec![symbol])
            }
            ProductionPattern::Rule { rule_name } => Ok(vec![self.get_symbol_by_name(rule_name)?]),
        }
    }
}
