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
}
impl<'grammar> ParserGraph<'grammar> {
    fn new() -> Self {
        ParserGraph {
            state_map: BidiMap::new(),
            graph: DiGraph::new(),
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
enum TableEntry {
    Shift { target: usize },
    Reduce { rule: Rule },
    Error,
    None,
}

impl Display for TableEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TableEntry::Shift { target } => write!(f, "s{}", target),
            TableEntry::Reduce { rule } => write!(f, "r{:?}", rule),
            TableEntry::Error => write!(f, "er"),
            TableEntry::None => write!(f, "  "),
        }
    }
}

#[derive(Debug)]
pub struct ActionGotoTable {
    entries: Vec<TableEntry>,
    terminal_count: usize,
    non_terminal_count: usize,
}

impl Display for ActionGotoTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let states = self.entries.len() / (self.terminal_count + self.non_terminal_count);
        write!(f, "  ")?;
        for token in 0..self.terminal_count {
            write!(f, "  T{}", token)?;
        }
        for non_terminal in 0..self.non_terminal_count {
            write!(f, "  N{}", non_terminal)?;
        }
        writeln!(f, "")?;
        for state in 0..states {
            write!(f, "S{}", state)?;
            for token in 0..self.terminal_count {
                let entry = self.get_entry(state, Symbol::Terminal(token as u32));
                write!(f, "  {}", entry)?;
            }
            for non_terminal in 0..self.non_terminal_count {
                let entry = self.get_entry(state, Symbol::NonTerminal(non_terminal as u32));
                write!(f, "  {}", entry)?;
            }
            writeln!(f, "")?;
        }
        write!(f, "")
    }
}

impl ActionGotoTable {
    fn new(terminal_count: usize, non_terminal_count: usize, node_count: usize) -> Self {
        ActionGotoTable {
            entries: vec![TableEntry::None; (terminal_count + non_terminal_count) * node_count],
            terminal_count,
            non_terminal_count,
        }
    }

    fn get_entry_mut(&mut self, state: usize, symbol: Symbol) -> &mut TableEntry {
        let symbol = match symbol {
            Symbol::Terminal(t) => t as usize,
            Symbol::NonTerminal(t) => t as usize + self.terminal_count,
            _ => unreachable!(),
        };
        &mut self.entries[state * (self.terminal_count + self.non_terminal_count) + symbol]
    }
    fn get_entry(&self, state: usize, symbol: Symbol) -> &TableEntry {
        let symbol = match symbol {
            Symbol::Terminal(t) => t as usize,
            Symbol::NonTerminal(t) => t as usize + self.terminal_count,
            _ => unreachable!(),
        };
        &self.entries[state * (self.terminal_count + self.non_terminal_count) + symbol]
    }

    fn insert_reduce(&mut self, state: NodeIndex, symbol: Symbol, rule: Rule) {
        *self.get_entry_mut(state.index(), symbol) = TableEntry::Reduce { rule };
    }

    fn insert_shift(&mut self, state: NodeIndex, symbol: Symbol, target: NodeIndex) {
        *self.get_entry_mut(state.index(), symbol) = TableEntry::Shift {
            target: target.index(),
        };
    }

    fn insert_error(&mut self, state: NodeIndex, symbol: Symbol) {
        *self.get_entry_mut(state.index(), symbol) = TableEntry::Error;
    }
}

pub fn generate_table<'grammar>(
    grammar: &'grammar Grammar,
) -> Result<ActionGotoTable, Vec<Conflict<'grammar>>> {
    let parser_graph = generate_parser_graph(grammar);
    let conflicts = find_conflicts(&parser_graph);
    if !conflicts.is_empty() {
        return Err(conflicts);
    }

    let terminal_count = grammar.terminals().count();
    let non_terminal_count = grammar.non_terminals().count();
    let node_count = parser_graph.graph.node_indices().count();
    let mut table = ActionGotoTable::new(terminal_count, non_terminal_count, node_count);
    'states: for (item_set, state) in parser_graph.state_map.iter() {
        for item in item_set {
            if item.symbol_after_dot().is_none() {
                for symbol in grammar.symbols() {
                    table.insert_reduce(*state, symbol, item.rule().clone())
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
