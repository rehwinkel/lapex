use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fmt::Display,
    io::Write,
};

use petgraph::{graph::NodeIndex, prelude::DiGraph, visit::EdgeRef, Direction::Outgoing, Graph};

use crate::{
    grammar::{Grammar, Rule, Symbol},
    util::{compute_first_sets, get_first_terminals_of_sequence},
};

use self::bidimap::BidiMap;

mod bidimap;
mod codegen;
mod item;

pub use codegen::LRParserCodeGen;

use item::Item;

type ItemSet<'grammar, const N: usize> = BTreeSet<Item<'grammar, N>>;

fn expand_item<'grammar, const N: usize>(
    item: Item<'grammar, N>,
    grammar: &'grammar Grammar,
    first_sets: &HashMap<Symbol, HashSet<Symbol>>,
) -> ItemSet<'grammar, N> {
    let mut item_set: ItemSet<'grammar, N> = BTreeSet::new();
    let mut to_expand: Vec<Item<N>> = Vec::new();
    item_set.insert(item.clone());
    to_expand.push(item);
    while !to_expand.is_empty() {
        let top = to_expand.pop().unwrap(); // stack is not empty so pop always works
        if let Some(symbol_after_dot) = top.symbol_after_dot() {
            for rule in grammar.rules() {
                // since LHS is always nonterminal, no additional check is needed
                if let Some(lhs) = rule.lhs() {
                    if lhs == symbol_after_dot {
                        let lookaheads = determine_lookaheads_to_expand(&top, first_sets, &top);

                        for lookahead in lookaheads {
                            let mut item = Item::new(rule, lookahead);
                            while let Some(Symbol::Epsilon) = item.symbol_after_dot() {
                                item.advance_dot();
                            }
                            if !item_set.contains(&item) {
                                item_set.insert(item.clone());
                                to_expand.push(item);
                            }
                        }
                    }
                }
            }
        }
    }
    item_set
}

fn determine_lookaheads_to_expand<const N: usize>(
    item: &Item<N>,
    first_sets: &HashMap<Symbol, HashSet<Symbol>>,
    top: &Item<'_, N>,
) -> Vec<[Symbol; N]> {
    if N > 1 {
        panic!("LR(N) with N > 1 not supported");
    }
    let follow_symbol = item.symbol_after_dot_offset(1);
    let lookaheads = if N > 0 {
        if let Some(follow_symbol) = follow_symbol {
            match follow_symbol {
                t @ Symbol::Terminal(_) => vec![[t; N]],
                Symbol::NonTerminal(_) => {
                    let remaining_rhs: Vec<Symbol> = item
                        .symbols_following_symbol_after_dot()
                        .chain(std::iter::once(top.lookahead()[0]))
                        .collect();
                    let terminals = get_first_terminals_of_sequence(&remaining_rhs, first_sets);
                    terminals.iter().map(|s| [*s; N]).collect()
                }
                _ => unreachable!(),
            }
        } else {
            vec![top.lookahead().clone()]
        }
    } else {
        vec![top.lookahead().clone()]
    };
    lookaheads
}

struct ParserGraph<'grammar, const N: usize> {
    state_map: BidiMap<ItemSet<'grammar, N>, NodeIndex>,
    graph: Graph<(), Symbol>,
    entry_state: Option<NodeIndex>,
}

impl<'grammar, const N: usize> ParserGraph<'grammar, N> {
    fn new() -> Self {
        ParserGraph {
            state_map: BidiMap::new(),
            graph: DiGraph::new(),
            entry_state: None,
        }
    }

    fn add_state(&mut self, set: ItemSet<'grammar, N>) -> NodeIndex {
        let entry_node = self.graph.add_node(());
        self.state_map.insert(set, entry_node);
        entry_node
    }

    fn get_item_set(&self, state: &NodeIndex) -> Option<&ItemSet<'grammar, N>> {
        self.state_map.get_b_to_a(state)
    }

    fn get_state(&self, set: &ItemSet<'grammar, N>) -> Option<&NodeIndex> {
        self.state_map.get_a_to_b(set)
    }

    fn add_transition(
        &mut self,
        start_state: NodeIndex,
        target_state: NodeIndex,
        transition: Symbol,
    ) {
        self.graph.add_edge(start_state, target_state, transition);
    }
}

fn generate_parser_graph<'grammar, const N: usize>(
    grammar: &'grammar Grammar,
    first_sets: &HashMap<Symbol, HashSet<Symbol>>,
) -> ParserGraph<'grammar, N> {
    let entry_item = Item::new(grammar.entry_rule(), [Symbol::End; N]);
    let entry_item_set = expand_item(entry_item, grammar, first_sets);
    let mut parser_graph = ParserGraph::new();
    let entry_state = parser_graph.add_state(entry_item_set);
    parser_graph.entry_state = Some(entry_state);

    let mut unprocessed_states = Vec::new();
    unprocessed_states.push(entry_state);

    while let Some(start_state) = unprocessed_states.pop() {
        let item_set = parser_graph.get_item_set(&start_state).unwrap();
        let mut transition_map: HashMap<Symbol, ItemSet<'grammar, N>> = HashMap::new();
        for item in item_set {
            if let Some(transition_symbol) = item.symbol_after_dot() {
                let mut target_item = item.clone();
                if target_item.rule().lhs().is_some() {
                    target_item.advance_dot();
                    let target_item_set = expand_item(target_item, grammar, first_sets);
                    let transition_set = transition_map
                        .entry(transition_symbol)
                        .or_insert(BTreeSet::new());
                    transition_set.extend(target_item_set.into_iter());
                }
            }
        }
        for (edge, item_set) in transition_map {
            let target_state = if let Some(state) = parser_graph.get_state(&item_set) {
                *state
            } else {
                let state = parser_graph.add_state(item_set);
                unprocessed_states.push(state);
                state
            };
            parser_graph.add_transition(start_state, target_state, edge);
        }
    }
    parser_graph
}

#[derive(Debug)]
pub enum Conflict<'grammar, const N: usize> {
    ShiftReduce {
        item_to_reduce: Item<'grammar, N>,
        shift_symbol: Symbol,
    },
    ReduceReduce {
        items: Vec<Item<'grammar, N>>,
    },
}

fn find_conflicts<'grammar, const N: usize>(
    parser_graph: &ParserGraph<'grammar, N>,
) -> Vec<Conflict<'grammar, N>> {
    let mut conflicts = Vec::new();
    for (item_set, state) in parser_graph.state_map.iter() {
        let mut reducing_items: HashMap<[Symbol; N], Vec<&Item<N>>> = HashMap::new();
        for item in item_set {
            if item.symbol_after_dot().is_none() {
                reducing_items
                    .entry(item.lookahead().clone())
                    .or_insert(Vec::new())
                    .push(item);
            }
        }
        for (lookahead, reducing_items) in reducing_items {
            if reducing_items.len() > 1 {
                conflicts.push(Conflict::ReduceReduce {
                    items: reducing_items.into_iter().map(|i| i.clone()).collect(),
                });
            } else if reducing_items.len() == 1 {
                let outgoing_edges = parser_graph.graph.edges_directed(*state, Outgoing);
                for edge in outgoing_edges {
                    if N > 1 {
                        panic!("LR(N) with N > 1 not supported");
                    } else if N == 1 {
                        if lookahead[0] == *edge.weight() {
                            conflicts.push(Conflict::ShiftReduce {
                                item_to_reduce: reducing_items.first().map(|i| *i).unwrap().clone(),
                                shift_symbol: *edge.weight(),
                            })
                        }
                    } else {
                        conflicts.push(Conflict::ShiftReduce {
                            item_to_reduce: reducing_items.first().map(|i| *i).unwrap().clone(),
                            shift_symbol: *edge.weight(),
                        })
                    }
                }
            }
        }
    }
    conflicts
}

#[derive(Clone, Debug)]
pub enum TableEntry<'grammar> {
    Shift { target: usize },
    Reduce { rule: &'grammar Rule },
    Error,
    Accept,
}

impl<'grammar> Display for TableEntry<'grammar> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TableEntry::Shift { target } => write!(f, "s{}", target),
            TableEntry::Reduce { rule } => write!(f, "r{:?}", rule),
            TableEntry::Error => write!(f, "er"),
            TableEntry::Accept => write!(f, "ac"),
        }
    }
}

#[derive(Debug)]
pub struct ActionGotoTable<'grammar> {
    entries: HashMap<(usize, Symbol), TableEntry<'grammar>>,
    state_count: usize,
    entry_state: usize,
}

impl<'grammar> ActionGotoTable<'grammar> {
    fn new(state_count: usize, entry_state: usize) -> Self {
        ActionGotoTable {
            entries: HashMap::new(),
            state_count,
            entry_state,
        }
    }

    pub fn get_entry(&self, state: usize, symbol: Symbol) -> Option<&TableEntry> {
        self.entries.get(&(state, symbol))
    }

    pub fn iter_state_terminals(
        &self,
        state: usize,
        grammar: &'grammar Grammar,
    ) -> impl Iterator<Item = (Symbol, Option<&TableEntry>)> {
        grammar
            .terminals()
            .chain(std::iter::once(Symbol::End))
            .map(move |s| (s, self.get_entry(state, s)))
    }

    pub fn iter_state_non_terminals(
        &self,
        state: usize,
        grammar: &'grammar Grammar,
    ) -> impl Iterator<Item = (Symbol, Option<&TableEntry>)> {
        grammar
            .non_terminals()
            .map(move |s| (s, self.get_entry(state, s)))
    }

    pub fn entry_state(&self) -> usize {
        self.entry_state
    }

    pub fn states(&self) -> usize {
        self.state_count
    }

    fn insert_reduce(&mut self, state: NodeIndex, symbol: Symbol, rule: &'grammar Rule) {
        let prev_entry = self
            .entries
            .insert((state.index(), symbol), TableEntry::Reduce { rule });
        assert!(prev_entry.is_none());
    }

    fn insert_shift(&mut self, state: NodeIndex, symbol: Symbol, target: NodeIndex) {
        let prev_entry = self.entries.insert(
            (state.index(), symbol),
            TableEntry::Shift {
                target: target.index(),
            },
        );
        assert!(prev_entry.is_none());
    }

    fn insert_error(&mut self, state: NodeIndex, symbol: Symbol) {
        let prev_entry = self
            .entries
            .insert((state.index(), symbol), TableEntry::Error);
        assert!(prev_entry.is_none());
    }

    fn insert_accept(&mut self, state: NodeIndex, symbol: Symbol) {
        let prev_entry = self
            .entries
            .insert((state.index(), symbol), TableEntry::Accept);
        assert!(prev_entry.is_none());
    }

    pub fn state_has_shift(&self, state: usize, grammar: &'grammar Grammar) -> bool {
        self.iter_state_non_terminals(state, grammar)
            .chain(self.iter_state_terminals(state, grammar))
            .any(|(_s, e)| {
                if let Some(TableEntry::Shift { target: _ }) = e {
                    true
                } else {
                    false
                }
            })
    }
}

pub fn generate_table<'grammar, const N: usize>(
    grammar: &'grammar Grammar,
) -> Result<ActionGotoTable<'grammar>, Vec<Conflict<'grammar, N>>> {
    let first_sets = if N > 0 {
        compute_first_sets(grammar)
    } else {
        HashMap::new()
    };
    let parser_graph = generate_parser_graph(grammar, &first_sets);
    let conflicts = find_conflicts(&parser_graph);
    if !conflicts.is_empty() {
        return Err(conflicts);
    }

    let entry_state = parser_graph.entry_state.unwrap().index();
    let node_count = parser_graph.graph.node_indices().count();
    let mut table: ActionGotoTable<'grammar> = ActionGotoTable::new(node_count, entry_state);
    for (item_set, state) in parser_graph.state_map.iter() {
        for item in item_set {
            // we can continue after this since there can be at most one reducable (conflicts already checked)
            if item.symbol_after_dot().is_none() {
                match N {
                    0 => {
                        for symbol in grammar.symbols().chain(std::iter::once(Symbol::End)) {
                            table.insert_reduce(*state, symbol, item.rule())
                        }
                    }
                    1 => {
                        table.insert_reduce(*state, item.lookahead()[0], item.rule());
                    }
                    _ => panic!("LR(N) with N > 1 not supported"),
                }
            }
        }
        let reachable_states: HashMap<Symbol, NodeIndex> = parser_graph
            .graph
            .edges_directed(*state, Outgoing)
            .map(|e| (*e.weight(), e.target()))
            .collect();
        for symbol in grammar.symbols() {
            if symbol == *grammar.entry_point() && state.index() == entry_state {
                table.insert_accept(*state, symbol);
            } else {
                if let Some(target) = reachable_states.get(&symbol) {
                    table.insert_shift(*state, symbol, *target);
                } else {
                    if table.get_entry(state.index(), symbol).is_none() {
                        table.insert_error(*state, symbol);
                    }
                }
            }
        }
    }

    Ok(table)
}

pub fn output_table<'grammar>(
    grammar: &'grammar Grammar,
    table: &ActionGotoTable<'grammar>,
    output: &mut dyn Write,
) -> std::io::Result<()> {
    let rule_index_map: HashMap<*const Rule, usize> = grammar
        .rules()
        .iter()
        .enumerate()
        .map(|(i, r)| (r as *const Rule, i))
        .collect();
    writeln!(output, "Rules:")?;
    for (rule, index) in rule_index_map.iter() {
        writeln!(
            output,
            "{}: {}",
            index,
            unsafe { rule.as_ref() }.unwrap().display(grammar)
        )?;
    }
    writeln!(output, "")?;
    let state_count_digits = format!("{}", table.state_count).len();
    let mut column_sizes = Vec::new();
    write!(output, "{: >width$}", "", width = state_count_digits)?;
    for symbol in grammar.symbols().chain(std::iter::once(Symbol::End)) {
        let name = grammar.get_symbol_name(&symbol);
        column_sizes.push(name.len());
        write!(output, "|{}", name)?;
    }
    writeln!(output, "|")?;
    for state in 0..table.state_count {
        write!(output, "{:0width$}|", state, width = state_count_digits)?;
        for (i, symbol) in grammar
            .symbols()
            .chain(std::iter::once(Symbol::End))
            .enumerate()
        {
            match table.get_entry(state, symbol) {
                Some(TableEntry::Shift { target }) => {
                    let rule_id_text = format!("s{}", target);
                    write!(
                        output,
                        "{: <width$}|",
                        rule_id_text,
                        width = column_sizes[i]
                    )?;
                }
                Some(TableEntry::Reduce { rule }) => {
                    let rule_id_text =
                        format!("r{}", rule_index_map.get(&(*rule as *const Rule)).unwrap());
                    write!(
                        output,
                        "{: <width$}|",
                        rule_id_text,
                        width = column_sizes[i]
                    )?;
                }
                Some(TableEntry::Error) => {
                    write!(output, "{: <width$}|", "e", width = column_sizes[i])?
                }
                Some(TableEntry::Accept) => {
                    write!(output, "{: <width$}|", "a", width = column_sizes[i])?
                }
                None => write!(output, "{: <width$}|", "", width = column_sizes[i])?,
            }
        }
        writeln!(output, "")?;
    }
    Ok(())
}
