use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
};

use lapex_input::{EntryRule, ProductionRule, TokenRule};

mod bnf;
use bnf::{Bnf, BnfRule, Symbol};
use petgraph::{data::Build, dot::Dot, graph::NodeIndex, prelude::DiGraph, Graph};

/*
fn first(rule: Symbol, bnf: &Bnf) -> HashSet<Symbol> {
    match rule {
        Symbol::Terminal { token: _ } | Symbol::Epsilon => {
            let mut set = HashSet::new();
            set.insert(rule);
            return set;
        }
        _ => (),
    }
    let mut set = HashSet::new();
    for bnf_rule in bnf.iter() {
        if bnf_rule.lhs() == &rule {
            let mut all_contain_epsilon = true;
            for symbol in bnf_rule.rhs() {
                match symbol {
                    Symbol::Terminal { token: _ } | Symbol::Epsilon => {
                        set.insert(symbol.clone());
                        all_contain_epsilon = false;
                        break;
                    }
                    _ => {
                        let mut first_of_symbol = first(symbol.clone(), bnf);
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

fn follow(rule: Symbol, entry: &Symbol, bnf: &Bnf) -> HashSet<Symbol> {
    if &rule == entry {
        let mut follow_set = HashSet::new();
        follow_set.insert(Symbol::End);
        return follow_set;
    }
    let mut follow_set = HashSet::new();
    for bnf_rule in bnf.iter() {
        let indices = bnf_rule
            .rhs()
            .iter()
            .enumerate()
            .filter_map(|(i, it)| (it == &rule).then(|| i));
        for i in indices {
            let tail = &bnf_rule.rhs()[i + 1..];
            if tail.len() > 0 {
                let mut all_contain_epsilon = true;
                for tail_symbol in tail {
                    let mut tail_symbol_first = first(tail_symbol.clone(), bnf);
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
                    follow_set.extend(follow(bnf_rule.lhs().clone(), entry, bnf));
                }
            } else {
                if &rule != bnf_rule.lhs() {
                    follow_set.extend(follow(bnf_rule.lhs().clone(), entry, bnf));
                } else {
                    let mut parent_first = first(bnf_rule.lhs().clone(), bnf);
                    parent_first.remove(&Symbol::Epsilon);
                    follow_set.extend(parent_first);
                }
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
    fn builder<'bnf>(tokens: &[TokenRule], bnf: &'bnf Bnf) -> ParserTableBuilder<'bnf> {
        let unique_symbols: HashSet<Symbol> = bnf.iter().map(|rule| rule.lhs().clone()).collect();
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
                    .chain((0..self.table.nonterms_count).into_iter().map(|j| {
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
            Symbol::NonTerminal { index: i } => self.rules_count + i,
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
        println!(
            "table {:?} (L: {:?}) => {:?}",
            &nonterminal, &lookahead, &production
        );
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
            Symbol::NonTerminal { index: i } => self.rules_count + i,
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
*/

fn build_first_graph(
    graph: &mut Graph<Symbol, ()>,
    symbol_map: &mut HashMap<Symbol, NodeIndex>,
    entry: &Symbol,
    bnf: &Bnf,
) -> NodeIndex {
    let mut entry_node_index = None;
    for rule in bnf.iter().filter(|rule| rule.lhs() == entry) {
        let node_index = *symbol_map
            .entry(rule.lhs().clone())
            .or_insert_with_key(|key| graph.add_node(key.clone()));
        entry_node_index = Some(node_index);
        if let Some(first_symbol) = rule.rhs().first() {
            let existing_target_index = symbol_map.get(first_symbol);
            let target_index = if let Some(target) = existing_target_index {
                *target
            } else {
                match first_symbol {
                    Symbol::Terminal { token: _ } | Symbol::Epsilon => {
                        graph.add_node(first_symbol.clone())
                    }
                    _ => build_first_graph(graph, symbol_map, first_symbol, bnf),
                }
            };
            graph.add_edge(node_index, target_index, ());
        }
    }
    entry_node_index.expect("should be result")
}

fn first<'bnf>(
    symbol: &Symbol,
    first_map: &mut HashMap<Symbol, HashMap<Symbol, &'bnf BnfRule>>,
    bnf: &'bnf Bnf,
) {
    let mut first_set = HashMap::new();
    for rule in bnf.iter().filter(|rule| rule.lhs() == symbol) {
        let first_symbol = rule.rhs().first().unwrap();
        match first_symbol {
            Symbol::Terminal { token: _ } | Symbol::Epsilon => {
                let previous = first_set.insert(first_symbol.clone(), rule);
                if previous.is_some() {
                    panic!("two child rules start with the same token")
                }
            }
            _ => {
                let existing_symbols = first_map
                    .get(first_symbol)
                    .expect("every first-symbol should already exist");
                for existing in existing_symbols {
                    let previous = first_set.insert(existing.0.clone(), *existing.1);
                    if previous.is_some() {
                        panic!("two child rules start with the same token")
                    }
                }
            }
        }
    }
    first_map.insert(symbol.clone(), first_set);
}

pub fn generate_table(entry: &EntryRule, tokens: &[TokenRule], rules: &[ProductionRule]) -> () {
    let entry_symbol = rules
        .iter()
        .enumerate()
        .find(|(_, it)| entry.name() == it.name())
        .map(|(i, _)| Symbol::NonTerminalRule { rule_index: i })
        .expect("entry symbol must be a valid rule"); // TODO return result
    let bnf = bnf::build_bnf(tokens, rules); //.optimize_bnf(&entry_symbol);

    // println!("{:?}", bnf);
    let mut symmap = HashMap::new();
    let mut g = DiGraph::new();
    build_first_graph(&mut g, &mut symmap, &entry_symbol, &bnf);
    let sorted_symbols: Vec<&Symbol> = petgraph::algo::toposort(&g, None)
        .expect("cylces found, should be result")
        .into_iter()
        .map(|nid| g.node_weight(nid).unwrap())
        .rev()
        .collect();

    let mut first_map = HashMap::new();
    for symbol in sorted_symbols {
        match symbol {
            Symbol::NonTerminal { index: _ } | Symbol::NonTerminalRule { rule_index: _ } => {
                first(symbol, &mut first_map, &bnf);
                println!(
                    "{:?} => {:?}",
                    symbol,
                    first_map.get(symbol).unwrap().keys()
                );
            }
            _ => (),
        }
    }

    /*
    let mut parser_table = ParserTable::builder(tokens, &bnf);
    for bnf_rule in bnf.iter() {
        let first_symbols = first(bnf_rule.rhs()[0].clone(), &bnf);
        println!("first {:?} -> {:?}", &bnf_rule.lhs(), &first_symbols);
        if first_symbols.contains(&Symbol::Epsilon) {
            let follow_symbols = follow(bnf_rule.lhs().clone(), &entry_symbol, &bnf);
            println!("follo {:?} -> {:?}", &bnf_rule.lhs(), &follow_symbols);
            // TODO: rules are overwriting each other
            for sym in &follow_symbols {
                if sym != &Symbol::End {
                    parser_table.insert(bnf_rule.lhs().clone(), sym.clone(), &bnf_rule.rhs());
                }
            }
            if follow_symbols.contains(&Symbol::End) {
                parser_table.insert(bnf_rule.lhs().clone(), Symbol::End, &bnf_rule.rhs());
            }
        }
        for sym in first_symbols {
            if sym != Symbol::Epsilon {
                parser_table.insert(bnf_rule.lhs().clone(), sym, &bnf_rule.rhs());
            }
        }
    }
    // TODO: remove rules that map one symbol to another: A -> B
    parser_table.build()
    */
}
