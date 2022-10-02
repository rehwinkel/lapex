use lapex_input::{ProductionPattern, ProductionRule, TokenRule};
use std::collections::{BTreeSet, HashMap};
use std::fmt::Debug;
use std::slice::Iter;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Symbol {
    NonTerminal { index: usize },
    NonTerminalRule { rule_index: usize },
    Terminal { token: usize },
    Epsilon,
    End,
}

#[derive(Debug)]
pub struct BnfRule {
    symbol: Symbol,
    produces: Vec<Symbol>,
}

impl BnfRule {
    pub fn lhs(&self) -> &Symbol {
        &self.symbol
    }

    pub fn rhs(&self) -> &Vec<Symbol> {
        &self.produces
    }
}

pub struct Bnf {
    rules: Vec<BnfRule>,
}

impl Bnf {
    pub fn iter(&self) -> Iter<BnfRule> {
        self.rules.iter()
    }
}

impl Debug for Bnf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "BNF:")?;
        for rule in &self.rules {
            writeln!(f, "{:?} => {:?}", rule.symbol, rule.produces)?;
        }
        Ok(())
    }
}

fn build_bnf_from_pattern<'pr>(
    tmp_id: &mut usize,
    bnf_rules: &mut Vec<BnfRule>,
    production: &mut Vec<Symbol>,
    terminals: &[TokenRule],
    nonterminals: &[ProductionRule],
    pattern: &'pr ProductionPattern,
) {
    match pattern {
        lapex_input::ProductionPattern::Sequence { elements } => {
            for elem in elements {
                build_bnf_from_pattern(
                    tmp_id,
                    bnf_rules,
                    production,
                    terminals,
                    nonterminals,
                    elem,
                );
            }
        }
        lapex_input::ProductionPattern::Alternative { elements } => {
            *tmp_id += 1;
            let index = *tmp_id;
            for elem in elements {
                build_bnf_rule(
                    tmp_id,
                    bnf_rules,
                    terminals,
                    nonterminals,
                    Symbol::NonTerminal { index },
                    Some(elem),
                )
            }
            production.push(Symbol::NonTerminal { index });
        }
        lapex_input::ProductionPattern::OneOrMany { inner } => {
            *tmp_id += 1;
            let inner_index = *tmp_id;
            build_bnf_rule(
                tmp_id,
                bnf_rules,
                terminals,
                nonterminals,
                Symbol::NonTerminal { index: inner_index },
                Some(inner),
            );
            *tmp_id += 1;
            let index = *tmp_id;
            bnf_rules.push(BnfRule {
                symbol: Symbol::NonTerminal { index },
                produces: vec![Symbol::NonTerminal { index: inner_index }],
            });
            bnf_rules.push(BnfRule {
                symbol: Symbol::NonTerminal { index },
                produces: vec![
                    Symbol::NonTerminal { index: inner_index },
                    Symbol::NonTerminal { index },
                ],
            });
            production.push(Symbol::NonTerminal { index });
        }
        lapex_input::ProductionPattern::ZeroOrMany { inner } => {
            *tmp_id += 1;
            let inner_index = *tmp_id;
            build_bnf_rule(
                tmp_id,
                bnf_rules,
                terminals,
                nonterminals,
                Symbol::NonTerminal { index: inner_index },
                Some(inner),
            );
            *tmp_id += 1;
            let index = *tmp_id;
            bnf_rules.push(BnfRule {
                symbol: Symbol::NonTerminal { index },
                produces: vec![Symbol::Epsilon],
            });
            bnf_rules.push(BnfRule {
                symbol: Symbol::NonTerminal { index },
                produces: vec![
                    Symbol::NonTerminal { index: inner_index },
                    Symbol::NonTerminal { index },
                ],
            });
            production.push(Symbol::NonTerminal { index });
        }
        lapex_input::ProductionPattern::Optional { inner } => {
            *tmp_id += 1;
            let index = *tmp_id;
            build_bnf_rule(
                tmp_id,
                bnf_rules,
                terminals,
                nonterminals,
                Symbol::NonTerminal { index },
                Some(inner),
            );
            build_bnf_rule(
                tmp_id,
                bnf_rules,
                terminals,
                nonterminals,
                Symbol::NonTerminal { index },
                None,
            );
            production.push(Symbol::NonTerminal { index });
        }
        lapex_input::ProductionPattern::Rule { rule_name } => {
            let terminal_index = terminals
                .iter()
                .position(|tr| tr.token() == rule_name.as_str());
            let sym = if let Some(index) = terminal_index {
                Symbol::Terminal { token: index }
            } else {
                let nonterminal_index = nonterminals
                    .iter()
                    .position(|tr| tr.name() == rule_name.as_str());
                if let Some(index) = nonterminal_index {
                    Symbol::NonTerminalRule { rule_index: index }
                } else {
                    panic!("neither nonterm nor term!!")
                }
            };
            production.push(sym);
        }
    }
}

fn build_bnf_rule<'pr>(
    tmp_id: &mut usize,
    bnf_rules: &mut Vec<BnfRule>,
    terminals: &[TokenRule],
    nonterminals: &[ProductionRule],
    name: Symbol,
    pattern: Option<&'pr ProductionPattern>,
) {
    let mut seq = Vec::new();
    if let Some(pattern) = pattern {
        build_bnf_from_pattern(
            tmp_id,
            bnf_rules,
            &mut seq,
            terminals,
            nonterminals,
            pattern,
        );
    } else {
        seq.push(Symbol::Epsilon);
    }
    bnf_rules.push(BnfRule {
        symbol: name,
        produces: seq,
    });
}

impl Bnf {
    pub fn optimize_bnf(self, entry: &Symbol) -> Bnf {
        let mut occurences: HashMap<&Symbol, usize> = HashMap::new();
        for rule in &self.rules {
            occurences
                .entry(&rule.symbol)
                .and_modify(|v| *v += 1)
                .or_insert(1);
        }
        let mut mappings = HashMap::new();
        for (symbol, _) in occurences.iter().filter(|(_, v)| **v == 1) {
            let mut mapping: Option<(Symbol, Symbol)> = None;
            let mut mapping_set = false;
            for rule in self.rules.iter().filter(|rule| &&rule.symbol == symbol) {
                if rule.produces.len() == 1 {
                    if !mapping_set {
                        mapping = Some(((*symbol).clone(), rule.produces[0].clone()));
                        mapping_set = true;
                    }
                }
            }
            mapping.map(|(key, value)| mappings.insert(key, value));
        }
        let mut used = vec![entry.clone()];
        let rules: Vec<BnfRule> = self
            .rules
            .into_iter()
            .map(|rule| BnfRule {
                symbol: rule.symbol,
                produces: rule
                    .produces
                    .into_iter()
                    .map(|mut sym| {
                        while let Some(s) = mappings.get(&sym) {
                            sym = s.clone();
                        }
                        used.push(sym.clone());
                        sym
                    })
                    .collect(),
            })
            .collect();
        let rules: Vec<BnfRule> = rules
            .into_iter()
            .filter(|rule| used.contains(&&rule.symbol))
            .collect();

        let mut non_terminal_index_mapping = BTreeSet::new();
        for rule in &rules {
            if let Symbol::NonTerminal { index } = &rule.symbol {
                non_terminal_index_mapping.insert(*index);
            }
        }
        let rules = rules
            .into_iter()
            .map(|rule| BnfRule {
                symbol: match rule.symbol {
                    Symbol::NonTerminal { index } => Symbol::NonTerminal {
                        index: non_terminal_index_mapping
                            .iter()
                            .position(|i| *i == index)
                            .unwrap(),
                    },
                    _ => rule.symbol,
                },
                produces: rule
                    .produces
                    .into_iter()
                    .map(|s| match s {
                        Symbol::NonTerminal { index } => Symbol::NonTerminal {
                            index: non_terminal_index_mapping
                                .iter()
                                .position(|i| *i == index)
                                .unwrap(),
                        },
                        _ => s,
                    })
                    .collect(),
            })
            .collect();
        Bnf { rules }
    }
}

pub fn build_bnf(tokens: &[TokenRule], prods: &[ProductionRule]) -> Bnf {
    let mut bnf_rules = Vec::new();
    let mut tmp_id = 0;
    for (i, rule) in prods.iter().enumerate() {
        build_bnf_rule(
            &mut tmp_id,
            &mut bnf_rules,
            tokens,
            prods,
            Symbol::NonTerminalRule { rule_index: i },
            Some(rule.pattern()),
        )
    }
    Bnf { rules: bnf_rules }
}
