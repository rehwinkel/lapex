use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    hash::Hash,
};

use lapex_input::{EntryRule, ProductionPattern, ProductionRule, TokenRule};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Symbol {
    NonTerminal { index: usize },
    NonTerminalRule { rule_index: usize },
    Terminal { token: usize },
    Epsilon,
    End,
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

#[derive(Debug)]
struct BnfRule {
    symbol: Symbol,
    produces: Vec<Symbol>,
}

impl BnfRule {}

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

fn first(rule: Symbol, bnf_rules: &Vec<BnfRule>) -> HashSet<Symbol> {
    match rule {
        Symbol::Terminal { token: _ } | Symbol::Epsilon => {
            let mut set = HashSet::new();
            set.insert(rule);
            return set;
        }
        _ => (),
    }
    let mut set = HashSet::new();
    for bnf_rule in bnf_rules {
        if bnf_rule.symbol == rule {
            let mut all_contain_epsilon = true;
            for symbol in &bnf_rule.produces {
                match symbol {
                    Symbol::Terminal { token: _ } | Symbol::Epsilon => {
                        set.insert(symbol.clone());
                        all_contain_epsilon = false;
                        break;
                    }
                    _ => {
                        let mut first_of_symbol = first(symbol.clone(), bnf_rules);
                        if first_of_symbol.contains(&Symbol::Epsilon) {
                            first_of_symbol.remove(&Symbol::Epsilon);
                        } else {
                            all_contain_epsilon = false;
                        }
                        set.extend(first_of_symbol);
                    }
                }
            }
            if all_contain_epsilon {
                set.insert(Symbol::Epsilon);
            }
        }
    }
    set
}

fn follow(rule: Symbol, entry: &Symbol, bnf_rules: &Vec<BnfRule>) -> HashSet<Symbol> {
    if &rule == entry {
        let mut follow_set = HashSet::new();
        follow_set.insert(Symbol::End);
        return follow_set;
    }
    let mut follow_set = HashSet::new();
    for bnf_rule in bnf_rules {
        let indices = bnf_rule
            .produces
            .iter()
            .enumerate()
            .filter_map(|(i, it)| (it == &rule).then(|| i));
        for i in indices {
            let tail = &bnf_rule.produces[i + 1..];
            if tail.len() > 0 {
                let mut all_contain_epsilon = true;
                for tail_symbol in tail {
                    let mut tail_symbol_first = first(tail_symbol.clone(), bnf_rules);
                    if tail_symbol_first.contains(&Symbol::Epsilon) {
                        tail_symbol_first.remove(&Symbol::Epsilon);
                        follow_set.extend(tail_symbol_first);
                    } else {
                        all_contain_epsilon = false;
                        follow_set.extend(tail_symbol_first);
                        break;
                    }
                }
                if all_contain_epsilon {
                    follow_set.extend(follow(bnf_rule.symbol.clone(), entry, bnf_rules));
                }
            } else {
                follow_set.extend(follow(bnf_rule.symbol.clone(), entry, bnf_rules));
            }
        }
    }
    follow_set
}

struct ParserTableBuilder<'bnf> {
    table: Vec<Option<usize>>,
    tokens_count: usize,
    rules_count: usize,
    nonterms_count: usize,
    productions: HashMap<&'bnf Vec<Symbol>, usize>,
    prod_index: usize,
}

#[derive(Debug)]
pub struct ParserTable {
    table: Vec<Option<usize>>,
    tokens_count: usize,
    rules_count: usize,
    nonterms_count: usize,
    productions: Vec<Vec<Symbol>>,
}

impl ParserTable {
    fn builder<'bnf>(tokens: &[TokenRule], bnf_rules: &'bnf [BnfRule]) -> ParserTableBuilder<'bnf> {
        let unique_symbols: HashSet<Symbol> =
            bnf_rules.iter().map(|rule| rule.symbol.clone()).collect();
        let (named_nonterms, unnamed_nonterms): (Vec<Symbol>, Vec<Symbol>) =
            unique_symbols.into_iter().partition(|symbol| {
                if let Symbol::NonTerminalRule { rule_index: _ } = symbol {
                    true
                } else {
                    false
                }
            });
        let columns = tokens.len() + 1;
        let rows = named_nonterms.len() + unnamed_nonterms.len();
        ParserTableBuilder {
            table: vec![None; columns * rows],
            rules_count: named_nonterms.len(),
            nonterms_count: unnamed_nonterms.len(),
            tokens_count: tokens.len(),
            productions: HashMap::new(),
            prod_index: 0,
        }
    }
}

struct ParserTableDebug<'src> {
    tokens: &'src [TokenRule<'src>],
    prods: &'src [ProductionRule<'src>],
    table: &'src ParserTable,
}

impl<'src> Debug for ParserTableDebug<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "ParserTable")?;
        let columns: Vec<Vec<String>> = std::iter::once(String::new())
            .chain(self.tokens.iter().map(|token| token.token().to_string()))
            .chain(std::iter::once(String::from("<end>")))
            .enumerate()
            .map(|(i, token)| {
                std::iter::once(token)
                    .chain(self.prods.iter().enumerate().map(|(j, rule)| {
                        if i == 0 {
                            rule.name().to_string()
                        } else if i - 1 == self.tokens.len() {
                            format!(
                                "{:?}",
                                self.table
                                    .get(Symbol::NonTerminalRule { rule_index: j }, Symbol::End)
                            )
                        } else {
                            format!(
                                "{:?}",
                                self.table.get(
                                    Symbol::NonTerminalRule { rule_index: j },
                                    Symbol::Terminal { token: i - 1 },
                                )
                            )
                        }
                    }))
                    .chain((1..=self.table.nonterms_count).into_iter().map(|j| {
                        if i == 0 {
                            format!("nt{}", j)
                        } else if i - 1 == self.tokens.len() {
                            format!(
                                "{:?}",
                                self.table
                                    .get(Symbol::NonTerminalRule { rule_index: j }, Symbol::End)
                            )
                        } else {
                            format!(
                                "{:?}",
                                self.table.get(
                                    Symbol::NonTerminal { index: j },
                                    Symbol::Terminal { token: i - 1 },
                                )
                            )
                        }
                    }))
                    .collect()
            })
            .collect();
        for row in 0..columns[0].len() {
            for col in &columns {
                let col_width = col.iter().max_by_key(|s| s.len()).unwrap().len();
                let current = &col[row];
                write!(f, "{}{} | ", current, " ".repeat(col_width - current.len()))?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

impl ParserTable {
    pub fn debug<'src>(
        &'src self,
        tokens: &'src [TokenRule],
        prods: &'src [ProductionRule],
    ) -> impl Debug + 'src {
        ParserTableDebug {
            table: &self,
            tokens,
            prods,
        }
    }

    fn get(&self, nonterminal: Symbol, lookahead: Symbol) -> Option<&Vec<Symbol>> {
        let row_index = match nonterminal {
            Symbol::NonTerminalRule { rule_index: i } => i,
            Symbol::NonTerminal { index: i } => self.rules_count + i - 1,
            _ => unreachable!(),
        };
        let col_index = match lookahead {
            Symbol::Terminal { token: i } => i,
            Symbol::End => self.tokens_count,
            _ => unreachable!(),
        };
        let row_length = self.tokens_count + 1;
        let index = row_index * row_length + col_index;

        self.table[index].map(|i| &self.productions[i])
    }
}

impl<'bnf> ParserTableBuilder<'bnf> {
    fn insert(&mut self, nonterminal: Symbol, lookahead: Symbol, production: &'bnf Vec<Symbol>) {
        assert!(match lookahead {
            Symbol::Terminal { token: _ } | Symbol::End => true,
            _ => false,
        });
        assert!(match nonterminal {
            Symbol::NonTerminal { index: _ } | Symbol::NonTerminalRule { rule_index: _ } => true,
            _ => false,
        });

        let row_index = match nonterminal {
            Symbol::NonTerminalRule { rule_index: i } => i,
            Symbol::NonTerminal { index: i } => self.rules_count + i - 1,
            _ => unreachable!(),
        };
        let col_index = match lookahead {
            Symbol::Terminal { token: i } => i,
            Symbol::End => self.tokens_count,
            _ => unreachable!(),
        };
        let row_length = self.tokens_count + 1;
        let table_index = row_index * row_length + col_index;
        let i = if let Some(index) = self.productions.get(production) {
            *index
        } else {
            self.productions.insert(production, self.prod_index);
            let index = self.prod_index;
            self.prod_index += 1;
            index
        };
        self.table[table_index] = Some(i);
    }

    fn build(self) -> ParserTable {
        let mut productions = vec![Vec::new(); self.productions.len()];
        for (rule, index) in self.productions {
            productions[index] = rule.clone();
        }
        ParserTable {
            table: self.table,
            rules_count: self.rules_count,
            tokens_count: self.tokens_count,
            nonterms_count: self.nonterms_count,
            productions,
        }
    }
}

pub fn generate_table(
    entry: &EntryRule,
    tokens: &[TokenRule],
    rules: &[ProductionRule],
) -> ParserTable {
    let mut bnf_rules = Vec::new();
    let mut tmp_id = 0;
    for (i, rule) in rules.iter().enumerate() {
        build_bnf_rule(
            &mut tmp_id,
            &mut bnf_rules,
            tokens,
            rules,
            Symbol::NonTerminalRule { rule_index: i },
            Some(rule.pattern()),
        )
    }

    let entry_symbol = rules
        .iter()
        .enumerate()
        .find(|(_, it)| entry.name() == it.name())
        .map(|(i, _)| Symbol::NonTerminalRule { rule_index: i })
        .expect("entry symbol must be a valid rule"); // TODO return result

    let mut parser_table = ParserTable::builder(tokens, &bnf_rules);
    for bnf_rule in &bnf_rules {
        let first_symbols = first(bnf_rule.produces[0].clone(), &bnf_rules);
        if first_symbols.contains(&Symbol::Epsilon) {
            let follow_symbols = follow(bnf_rule.symbol.clone(), &entry_symbol, &bnf_rules);
            if follow_symbols.contains(&Symbol::End) {
                parser_table.insert(bnf_rule.symbol.clone(), Symbol::End, &bnf_rule.produces);
            } else {
                for sym in follow_symbols {
                    if sym != Symbol::End {
                        parser_table.insert(bnf_rule.symbol.clone(), sym, &bnf_rule.produces);
                    }
                }
            }
        }
        for sym in first_symbols {
            if sym != Symbol::Epsilon {
                parser_table.insert(bnf_rule.symbol.clone(), sym, &bnf_rule.produces);
            }
        }
    }
    // TODO: remove rules that map one symbol to another: A -> B
    parser_table.build()
}
