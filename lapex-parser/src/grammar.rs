use std::{
    collections::HashMap,
    error::Error,
    fmt::{Debug, Display},
    hash::Hash,
    num::TryFromIntError,
};

use lapex_input::{EntryRule, ProductionPattern, ProductionRule, TokenRule};

#[derive(Debug)]
pub enum GrammarError {
    TooManyRules,
    ConflictingRules,
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

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Symbol {
    Epsilon,
    End,
    NonTerminal(u32),
    Terminal(u32),
}

#[derive(Debug)]
pub struct Rule {
    lhs: u32,
    rhs: Vec<Symbol>,
}

impl Rule {
    pub fn new(lhs: Symbol, rhs: Vec<Symbol>) -> Self {
        let non_terminal_index = match lhs {
            Symbol::NonTerminal(i) => Some(i),
            _ => None,
        };
        assert!(
            non_terminal_index.is_some(),
            "Left hand side must be a nonterminal"
        );
        Rule {
            lhs: non_terminal_index.unwrap(),
            rhs,
        }
    }

    pub fn lhs(&self) -> Symbol {
        Symbol::NonTerminal(self.lhs)
    }

    pub fn rhs(&self) -> &Vec<Symbol> {
        &self.rhs
    }
}

#[derive(Debug)]
pub struct Grammar<'rules> {
    rules: Vec<Rule>,
    non_terminal_mapping: HashMap<Symbol, &'rules ProductionRule<'rules>>,
    tokens: &'rules [TokenRule<'rules>],
    non_terminal_count: u32,
    entry_symbol: Symbol,
}

struct GrammarBuilder<'rules> {
    temp_count: u32,
    non_terminal_mapping: HashMap<&'rules ProductionRule<'rules>, Symbol>,
    token_rules: &'rules [TokenRule<'rules>],
    production_rules: &'rules [ProductionRule<'rules>],
    rules: Vec<Rule>,
}

impl<'rules> GrammarBuilder<'rules> {
    fn new(token_rules: &'rules [TokenRule], production_rules: &'rules [ProductionRule]) -> Self {
        GrammarBuilder {
            temp_count: 0,
            non_terminal_mapping: HashMap::new(),
            token_rules,
            production_rules,
            rules: Vec::new(),
        }
    }

    fn get_temp_symbol(&mut self) -> Result<Symbol, GrammarError> {
        let non_terminal =
            Symbol::NonTerminal(self.temp_count + u32::try_from(self.non_terminal_mapping.len())?);
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

        if matching_tokens.len() + matching_prods.len() != 1 {
            Err(GrammarError::ConflictingRules)
        } else {
            if let Some(token) = matching_tokens.first() {
                Ok(Symbol::Terminal(u32::try_from(*token)?))
            } else {
                let prod_rule = *matching_prods.first().unwrap();
                if let Some(nonterminal) = self.non_terminal_mapping.get(prod_rule) {
                    Ok(*nonterminal)
                } else {
                    let nonterminal = Symbol::NonTerminal(
                        self.temp_count + u32::try_from(self.non_terminal_mapping.len())?,
                    );
                    self.non_terminal_mapping.insert(prod_rule, nonterminal);
                    Ok(nonterminal)
                }
            }
        }
    }
}

impl<'rules> Grammar<'rules> {
    pub fn from_rules(
        entry_rule: &'rules EntryRule,
        token_rules: &'rules [TokenRule],
        production_rules: &'rules [ProductionRule],
    ) -> Result<Self, GrammarError> {
        let mut grammar_builder = GrammarBuilder::new(token_rules, production_rules);
        for prod_rule in production_rules {
            grammar_builder.add_production_rule(prod_rule)?;
        }

        let entry_symbol = grammar_builder.get_symbol_by_name(entry_rule.name())?;
        let non_terminal_mapping: HashMap<Symbol, &ProductionRule> = grammar_builder
            .non_terminal_mapping
            .into_iter()
            .map(|(a, b)| (b, a))
            .collect();
        let non_terminal_count = grammar_builder.temp_count + non_terminal_mapping.len() as u32;
        Ok(Grammar {
            rules: grammar_builder.rules,
            tokens: token_rules,
            non_terminal_mapping,
            non_terminal_count,
            entry_symbol,
        })
    }

    pub fn non_terminals(&self) -> impl Iterator<Item = Symbol> {
        (0..self.non_terminal_count).map(|i| Symbol::NonTerminal(i as u32))
    }

    pub fn terminals(&self) -> impl Iterator<Item = Symbol> {
        (0..self.tokens.len()).map(|i| Symbol::Terminal(i as u32))
    }

    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    pub fn entry_point(&self) -> &Symbol {
        &self.entry_symbol
    }

    pub fn get_symbol_name(&self, symbol: Symbol) -> String {
        match symbol {
            Symbol::Terminal(terminal_index) => {
                self.tokens[terminal_index as usize].token().to_string()
            }
            Symbol::NonTerminal(non_terminal_index) => {
                if let Some(rule) = self.non_terminal_mapping.get(&symbol) {
                    rule.name().to_string()
                } else {
                    format!("<anon {}>", non_terminal_index)
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
            self.get_symbol_name(self.entry_symbol)
        )?;
        for rule in &self.rules {
            let rhs_sequence: Vec<String> = rule
                .rhs()
                .into_iter()
                .map(|s| self.get_symbol_name(*s))
                .collect();
            writeln!(
                f,
                "\t{} -> {}",
                self.get_symbol_name(rule.lhs()),
                rhs_sequence.join(" ")
            )?;
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
        self.rules.push(Rule::new(symbol, produces));
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
                    self.rules.push(Rule::new(alt_symbol, inner_produces));
                }
                Ok(vec![alt_symbol])
            }
            ProductionPattern::OneOrMany { inner } => {
                let rep_symbol = self.get_temp_symbol()?;
                let mut inner_produces = self.transform_pattern(inner)?;
                self.rules
                    .push(Rule::new(rep_symbol, inner_produces.clone()));
                inner_produces.push(rep_symbol);
                self.rules.push(Rule::new(rep_symbol, inner_produces));
                Ok(vec![rep_symbol])
            }
            ProductionPattern::ZeroOrMany { inner } => {
                let rep_symbol = self.get_temp_symbol()?;
                let mut inner_produces = self.transform_pattern(inner)?;
                inner_produces.push(rep_symbol);
                self.rules
                    .push(Rule::new(rep_symbol, vec![Symbol::Epsilon]));
                self.rules.push(Rule::new(rep_symbol, inner_produces));
                Ok(vec![rep_symbol])
            }
            ProductionPattern::Optional { inner } => {
                let symbol = self.get_temp_symbol()?;
                let inner_produces = self.transform_pattern(inner)?;
                self.rules.push(Rule::new(symbol, inner_produces));
                self.rules.push(Rule::new(symbol, vec![Symbol::Epsilon]));
                Ok(vec![symbol])
            }
            ProductionPattern::Rule { rule_name } => Ok(vec![self.get_symbol_by_name(rule_name)?]),
        }
    }
}
