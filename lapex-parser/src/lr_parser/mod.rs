use std::{
    collections::{BTreeSet, HashMap},
    fmt::Display,
};

use petgraph::{graph::NodeIndex, prelude::DiGraph, visit::EdgeRef, Direction::Outgoing, Graph};

use crate::grammar::{Grammar, Rule, Symbol};

use self::bidimap::BidiMap;

mod bidimap;
mod codegen;
mod item;

pub use codegen::LRParserCodeGen;

use item::Item;

type ItemSet<'grammar> = BTreeSet<Item<'grammar>>;

fn expand_item<'grammar>(item: Item<'grammar>, grammar: &'grammar Grammar) -> ItemSet<'grammar> {
    let mut item_set: ItemSet<'grammar> = BTreeSet::new();
    let mut to_expand: Vec<Item> = Vec::new();
    item_set.insert(item.clone());
    to_expand.push(item);
    while !to_expand.is_empty() {
        let top = to_expand.pop().unwrap(); // stack is not empty so pop always works
        if let Some(symbol_after_dot) = top.symbol_after_dot() {
            for rule in grammar.rules() {
                // since LHS is always nonterminal, no additional check is needed
                if let Some(lhs) = rule.lhs() {
                    if lhs == symbol_after_dot {
                        let item = Item::from(rule);
                        if !item_set.contains(&item) {
                            item_set.insert(item.clone());
                            to_expand.push(item);
                        }
                    }
                }
            }
        }
    }
    item_set
}

struct ParserGraph<'grammar> {
    state_map: BidiMap<ItemSet<'grammar>, NodeIndex>,
    graph: Graph<(), Symbol>,
    entry_state: Option<NodeIndex>,
}

impl<'grammar> ParserGraph<'grammar> {
    fn new() -> Self {
        ParserGraph {
            state_map: BidiMap::new(),
            graph: DiGraph::new(),
            entry_state: None,
        }
    }

    fn add_state(&mut self, set: ItemSet<'grammar>) -> NodeIndex {
        let entry_node = self.graph.add_node(());
        self.state_map.insert(set, entry_node);
        entry_node
    }

    fn get_item_set(&self, state: &NodeIndex) -> Option<&ItemSet<'grammar>> {
        self.state_map.get_b_to_a(state)
    }

    fn get_state(&self, set: &ItemSet<'grammar>) -> Option<&NodeIndex> {
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

fn generate_parser_graph<'grammar>(grammar: &'grammar Grammar) -> ParserGraph<'grammar> {
    let entry_item = Item::from(grammar.entry_rule());
    let entry_item_set = expand_item(entry_item, grammar);
    let mut parser_graph = ParserGraph::new();
    let entry_state = parser_graph.add_state(entry_item_set);
    parser_graph.entry_state = Some(entry_state);

    let mut unprocessed_states = Vec::new();
    unprocessed_states.push(entry_state);

    while let Some(start_state) = unprocessed_states.pop() {
        let item_set = parser_graph.get_item_set(&start_state).unwrap();
        let mut transition_map: HashMap<Symbol, ItemSet<'grammar>> = HashMap::new();
        for item in item_set {
            if let Some(transition_symbol) = item.symbol_after_dot() {
                let mut target_item = item.clone();
                if target_item.rule().lhs().is_some() {
                    target_item.advance_dot();
                    let target_item_set = expand_item(target_item, grammar);
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
pub enum Conflict<'grammar> {
    ShiftReduce {
        item_to_reduce: Item<'grammar>,
        shift_symbol: Symbol,
    },
    ReduceReduce {
        items: Vec<Item<'grammar>>,
    },
}

fn find_conflicts<'grammar>(parser_graph: &ParserGraph<'grammar>) -> Vec<Conflict<'grammar>> {
    let mut conflicts = Vec::new();
    for (item_set, state) in parser_graph.state_map.iter() {
        let mut reducing_items = Vec::new();
        for item in item_set {
            if item.symbol_after_dot().is_none() {
                reducing_items.push(item);
            }
        }
        if reducing_items.len() > 1 {
            conflicts.push(Conflict::ReduceReduce {
                items: reducing_items.into_iter().map(|i| i.clone()).collect(),
            })
        } else if reducing_items.len() == 1 {
            let outgoing_edges = parser_graph.graph.edges_directed(*state, Outgoing);
            for edge in outgoing_edges {
                conflicts.push(Conflict::ShiftReduce {
                    item_to_reduce: reducing_items.first().map(|i| *i).unwrap().clone(),
                    shift_symbol: *edge.weight(),
                })
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
}

impl<'grammar> Display for TableEntry<'grammar> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TableEntry::Shift { target } => write!(f, "s{}", target),
            TableEntry::Reduce { rule } => write!(f, "r{:?}", rule),
            TableEntry::Error => write!(f, "er"),
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
        self.entries
            .insert((state.index(), symbol), TableEntry::Reduce { rule });
    }

    fn insert_shift(&mut self, state: NodeIndex, symbol: Symbol, target: NodeIndex) {
        self.entries.insert(
            (state.index(), symbol),
            TableEntry::Shift {
                target: target.index(),
            },
        );
    }

    fn insert_error(&mut self, state: NodeIndex, symbol: Symbol) {
        self.entries
            .insert((state.index(), symbol), TableEntry::Error);
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

pub fn generate_table<'grammar>(
    grammar: &'grammar Grammar,
) -> Result<ActionGotoTable<'grammar>, Vec<Conflict<'grammar>>> {
    let parser_graph = generate_parser_graph(grammar);
    let conflicts = find_conflicts(&parser_graph);
    if !conflicts.is_empty() {
        return Err(conflicts);
    }

    let node_count = parser_graph.graph.node_indices().count();
    let mut table: ActionGotoTable<'grammar> =
        ActionGotoTable::new(node_count, parser_graph.entry_state.unwrap().index());
    'states: for (item_set, state) in parser_graph.state_map.iter() {
        for item in item_set {
            if item.symbol_after_dot().is_none() {
                for symbol in grammar.symbols().chain(std::iter::once(Symbol::End)) {
                    table.insert_reduce(*state, symbol, item.rule())
                }
                continue 'states;
            }
        }
        let reachable_states: HashMap<Symbol, NodeIndex> = parser_graph
            .graph
            .edges_directed(*state, Outgoing)
            .map(|e| (*e.weight(), e.target()))
            .collect();
        for symbol in grammar.symbols() {
            if let Some(target) = reachable_states.get(&symbol) {
                table.insert_shift(*state, symbol, *target);
            } else {
                table.insert_error(*state, symbol);
            }
        }
    }
    Ok(table)
}
