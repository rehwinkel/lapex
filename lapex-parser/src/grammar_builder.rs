use std::collections::BTreeMap;

use lapex_input::{ProductionPattern, ProductionRule, RuleSet, Spanned, TokenRule};

use crate::grammar::{Grammar, GrammarError, Rule, Symbol, SymbolIdx};

pub struct GrammarBuilder<'rules> {
    temp_count: SymbolIdx,
    symbols: BTreeMap<&'rules str, Symbol>,
    max_symbol: SymbolIdx,
    anonymous_non_terminals: Vec<Symbol>,
    tokens: BTreeMap<Symbol, &'rules str>,
    productions: BTreeMap<Symbol, &'rules str>,
    rule_set: &'rules RuleSet<'rules>,
    rules: Vec<Rule<'rules>>,
}

impl<'rules> GrammarBuilder<'rules> {
    pub fn from_rule_set(rule_set: &'rules RuleSet) -> Result<Self, GrammarError> {
        let token_triples: Vec<(&str, Symbol, &Spanned<TokenRule>)> = rule_set
            .token_rules
            .iter()
            .enumerate()
            .map(|(i, rule)| (rule.inner.name, Symbol::Terminal(i as SymbolIdx), rule))
            .collect();
        let production_triples: Vec<(&str, Symbol, &Spanned<ProductionRule>)> = rule_set
            .production_rules
            .iter()
            .enumerate()
            .map(|(i, rule)| (rule.inner.name, Symbol::NonTerminal(i as SymbolIdx), rule))
            .collect();
        let mut symbols_with_span = BTreeMap::new();
        let mut tokens = BTreeMap::new();
        let mut productions = BTreeMap::new();

        for (token_name, symbol, rule) in token_triples {
            if let Some((_, prev_span)) = symbols_with_span.insert(token_name, (symbol, rule.span))
            {
                return Err(GrammarError::ConflictingRules {
                    rules: vec![prev_span, rule.span],
                });
            }
            tokens.insert(symbol, rule.inner.name);
        }
        for (prod_name, symbol, rule) in production_triples {
            if let Some((existing_symbol, existing_span)) = symbols_with_span.get(prod_name) {
                if tokens.contains_key(existing_symbol) {
                    return Err(GrammarError::ConflictingRules {
                        rules: vec![existing_span.clone(), rule.span],
                    });
                }
            } else {
                symbols_with_span.insert(prod_name, (symbol, rule.span));
                productions.insert(symbol, rule.inner.name);
            }
        }

        Ok(GrammarBuilder {
            temp_count: 0,
            rule_set,
            rules: Vec::new(),
            max_symbol: symbols_with_span
                .values()
                .map(|(s, _)| match s {
                    Symbol::Epsilon => 0,
                    Symbol::End => 0,
                    Symbol::NonTerminal(nt_index) => *nt_index,
                    Symbol::Terminal(t_index) => *t_index,
                })
                .max()
                .unwrap(),
            symbols: symbols_with_span
                .into_iter()
                .map(|(name, (symbol, _span))| (name, symbol))
                .collect(),
            anonymous_non_terminals: Vec::new(),
            tokens,
            productions,
        })
    }

    fn get_temp_symbol(&mut self) -> Result<Symbol, GrammarError> {
        let non_terminal = Symbol::NonTerminal(self.temp_count + self.max_symbol + 1);
        self.anonymous_non_terminals.push(non_terminal.clone());
        self.temp_count = self.temp_count.checked_add(1).unwrap();
        Ok(non_terminal)
    }

    fn get_symbol_by_name(&mut self, symbol_name: &str) -> Result<Symbol, GrammarError> {
        let symbol = self
            .symbols
            .get(symbol_name)
            .map(|s| s.clone())
            .ok_or(GrammarError::MissingSymbol(symbol_name.to_string()))?;
        Ok(symbol)
    }

    pub fn build(mut self) -> Result<Grammar<'rules>, GrammarError> {
        for rule in &self.rule_set.production_rules {
            self.add_production_rule(&rule)?;
        }
        let entry_name = self.rule_set.entry_rule.inner.name;
        let entry_symbol = self.get_symbol_by_name(entry_name)?;
        let entry_production = self
            .rule_set
            .production_rules
            .iter()
            .find(|r| r.inner.name == entry_name)
            .ok_or(GrammarError::MissingSymbol(String::from(entry_name)))?;
        // the entry rule is a pseudo-rule that has no LHS and maps to the entry symbol.
        let entry_rule = Rule::entry(entry_symbol, &entry_production);
        Ok(Grammar::new(
            entry_symbol,
            entry_rule,
            self.rules,
            self.tokens,
            self.productions,
            self.anonymous_non_terminals,
        ))
    }
}

impl<'rules> GrammarBuilder<'rules> {
    fn add_production_rule(
        &mut self,
        prod_rule: &'rules Spanned<ProductionRule<'rules>>,
    ) -> Result<(), GrammarError> {
        let symbol = self.get_symbol_by_name(prod_rule.inner.name)?;
        let produces = self.transform_pattern(&prod_rule.inner.pattern, prod_rule)?;
        self.rules.push(Rule::new(symbol, produces, prod_rule)?);
        Ok(())
    }

    fn transform_pattern(
        &mut self,
        pattern: &ProductionPattern,
        parent_rule: &'rules Spanned<ProductionRule<'rules>>,
    ) -> Result<Vec<Symbol>, GrammarError> {
        match pattern {
            ProductionPattern::Sequence { elements } => {
                let symbols: Result<Vec<Vec<Symbol>>, GrammarError> = elements
                    .into_iter()
                    .map(|pattern| self.transform_pattern(pattern, parent_rule))
                    .collect();
                let symbols: Vec<Symbol> = symbols?.into_iter().flat_map(|v| v).collect();
                Ok(symbols)
            }
            ProductionPattern::Alternative { elements } => {
                let alt_symbol = self.get_temp_symbol()?;
                for elem in elements {
                    let inner_produces = self.transform_pattern(elem, parent_rule)?;
                    self.rules
                        .push(Rule::new(alt_symbol, inner_produces, parent_rule)?);
                }
                Ok(vec![alt_symbol])
            }
            ProductionPattern::OneOrMany { inner } => {
                let rep_symbol = self.get_temp_symbol()?;
                let mut inner_produces = self.transform_pattern(inner, parent_rule)?;
                self.rules
                    .push(Rule::new(rep_symbol, inner_produces.clone(), parent_rule)?);
                inner_produces.push(rep_symbol);
                self.rules
                    .push(Rule::new(rep_symbol, inner_produces, parent_rule)?);
                Ok(vec![rep_symbol])
            }
            ProductionPattern::ZeroOrMany { inner } => {
                let rep_symbol = self.get_temp_symbol()?;
                let mut inner_produces = self.transform_pattern(inner, parent_rule)?;
                inner_produces.push(rep_symbol);
                self.rules
                    .push(Rule::new(rep_symbol, vec![Symbol::Epsilon], parent_rule)?);
                self.rules
                    .push(Rule::new(rep_symbol, inner_produces, parent_rule)?);
                Ok(vec![rep_symbol])
            }
            ProductionPattern::Optional { inner } => {
                let symbol = self.get_temp_symbol()?;
                let inner_produces = self.transform_pattern(inner, parent_rule)?;
                self.rules
                    .push(Rule::new(symbol, inner_produces, parent_rule)?);
                self.rules
                    .push(Rule::new(symbol, vec![Symbol::Epsilon], parent_rule)?);
                Ok(vec![symbol])
            }
            ProductionPattern::Rule { rule_name } => Ok(vec![self.get_symbol_by_name(rule_name)?]),
            ProductionPattern::Epsilon => Ok(vec![Symbol::Epsilon]),
        }
    }
}
