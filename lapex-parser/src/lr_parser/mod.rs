use std::{
    collections::{BTreeMap, BTreeSet},
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

type ItemSet<'grammar, 'rules, const N: usize> = BTreeSet<Item<'grammar, 'rules, N>>;

fn get_lr0_core<'grammar, 'rules, const N: usize>(
    item_set: &ItemSet<'grammar, 'rules, N>,
) -> ItemSet<'grammar, 'rules, 0> {
    item_set.into_iter().map(|item| item.to_lr0()).collect()
}

fn expand_item<'grammar: 'rules, 'rules, const N: usize>(
    item: Item<'grammar, 'rules, N>,
    grammar: &'grammar Grammar,
    first_sets: &BTreeMap<Symbol, BTreeSet<Symbol>>,
) -> ItemSet<'grammar, 'rules, N> {
    let mut item_set: ItemSet<'grammar, 'rules, N> = BTreeSet::new();
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
    first_sets: &BTreeMap<Symbol, BTreeSet<Symbol>>,
    top: &Item<N>,
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

struct ParserGraph<'grammar: 'rules, 'rules, const N: usize> {
    state_map: BidiMap<ItemSet<'grammar, 'rules, N>, NodeIndex>,
    lr0_core_map: BTreeMap<ItemSet<'grammar, 'rules, 0>, NodeIndex>,
    graph: Graph<(), Symbol>,
    entry_state: Option<NodeIndex>,
}

impl<'grammar, 'rules, const N: usize> ParserGraph<'grammar, 'rules, N> {
    fn new() -> Self {
        ParserGraph {
            state_map: BidiMap::new(),
            lr0_core_map: BTreeMap::new(),
            graph: DiGraph::new(),
            entry_state: None,
        }
    }

    fn add_state(&mut self, set: ItemSet<'grammar, 'rules, N>) -> NodeIndex {
        let entry_node = self.graph.add_node(());
        self.lr0_core_map
            .insert(get_lr0_core(&set), entry_node.clone());
        self.state_map.insert(set, entry_node);
        entry_node
    }

    fn get_item_set(&self, state: &NodeIndex) -> Option<&ItemSet<'grammar, 'rules, N>> {
        self.state_map.get_b_to_a(state)
    }

    fn update_item_set<R, F>(&mut self, state: &NodeIndex, op: F) -> Option<R>
    where
        F: FnOnce(&mut ItemSet<'grammar, 'rules, N>) -> R,
    {
        let (mut set, state) = self.state_map.remove_by_b(state)?;
        let return_value = op(&mut set);
        self.lr0_core_map.insert(get_lr0_core(&set), state.clone());
        self.state_map.insert(set, state);
        Some(return_value)
    }

    fn get_state(&self, set: &ItemSet<'grammar, 'rules, N>) -> Option<&NodeIndex> {
        self.state_map.get_a_to_b(set)
    }

    fn get_state_by_lr0_core(&self, set: &ItemSet<'grammar, 'rules, N>) -> Option<&NodeIndex> {
        self.lr0_core_map.get(&get_lr0_core(set))
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

fn generate_parser_graph<'grammar: 'rules, 'rules, const N: usize>(
    grammar: &'grammar Grammar<'rules>,
    first_sets: &BTreeMap<Symbol, BTreeSet<Symbol>>,
    lalr: bool,
) -> ParserGraph<'grammar, 'rules, N> {
    let entry_item = Item::new(grammar.entry_rule(), [Symbol::End; N]);
    let entry_item_set = expand_item(entry_item, grammar, first_sets);
    let mut parser_graph = ParserGraph::new();
    let entry_state = parser_graph.add_state(entry_item_set);
    parser_graph.entry_state = Some(entry_state);

    let mut unprocessed_states = Vec::new();
    unprocessed_states.push(entry_state);

    while let Some(start_state) = unprocessed_states.pop() {
        let item_set = parser_graph.get_item_set(&start_state).unwrap();
        let mut transition_map: BTreeMap<Symbol, ItemSet<'grammar, 'rules, N>> = BTreeMap::new();
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
            if lalr {
                let target_state = if let Some(state) =
                    parser_graph.get_state_by_lr0_core(&item_set).map(|s| *s)
                {
                    let merged = merge_into_state(&mut parser_graph, state, item_set).unwrap();
                    if merged {
                        unprocessed_states.push(state);
                    }
                    state
                } else {
                    let state = parser_graph.add_state(item_set);
                    unprocessed_states.push(state);
                    state
                };
                parser_graph.add_transition(start_state, target_state, edge);
            } else {
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
    }
    parser_graph
}

fn merge_into_state<'grammar: 'rules, 'rules, const N: usize>(
    parser_graph: &mut ParserGraph<'grammar, 'rules, N>,
    state: NodeIndex,
    item_set: BTreeSet<Item<'grammar, 'rules, N>>,
) -> Option<bool> {
    parser_graph.update_item_set(&state, |update| {
        let mut reprocess = false;
        for item in item_set {
            let inserted = update.insert(item);
            if inserted {
                reprocess = true;
            }
        }
        reprocess
    })
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Conflict<'grammar, 'rules> {
    ShiftReduce {
        item_to_reduce: Item<'grammar, 'rules, 0>,
        shift_symbol: Symbol,
    },
    ReduceReduce {
        items: Vec<Item<'grammar, 'rules, 0>>,
    },
}

fn find_conflicts<'grammar, 'rules, const N: usize>(
    parser_graph: &ParserGraph<'grammar, 'rules, N>,
) -> BTreeSet<Conflict<'grammar, 'rules>> {
    let mut conflicts = BTreeSet::new();
    for (item_set, state) in parser_graph.state_map.iter() {
        let mut reducing_items: BTreeMap<[Symbol; N], Vec<&Item<N>>> = BTreeMap::new();
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
                conflicts.insert(Conflict::ReduceReduce {
                    items: reducing_items.into_iter().map(|i| i.to_lr0()).collect(),
                });
            } else if reducing_items.len() == 1 {
                let outgoing_edges = parser_graph.graph.edges_directed(*state, Outgoing);
                for edge in outgoing_edges {
                    if N > 1 {
                        panic!("LR(N) with N > 1 not supported");
                    } else if N == 1 {
                        if lookahead[0] == *edge.weight() {
                            conflicts.insert(Conflict::ShiftReduce {
                                item_to_reduce: reducing_items
                                    .first()
                                    .map(|i| i.to_lr0())
                                    .unwrap()
                                    .clone(),
                                shift_symbol: *edge.weight(),
                            });
                        }
                    } else {
                        conflicts.insert(Conflict::ShiftReduce {
                            item_to_reduce: reducing_items
                                .first()
                                .map(|i| i.to_lr0())
                                .unwrap()
                                .clone(),
                            shift_symbol: *edge.weight(),
                        });
                    }
                }
            }
        }
    }
    conflicts
}

#[derive(Clone, Debug)]
pub enum TableEntry<'grammar, 'rules> {
    Shift { target: usize },
    Reduce { rule: &'grammar Rule<'rules> },
    Error,
    Accept,
}

impl<'grammar, 'rules> Display for TableEntry<'grammar, 'rules> {
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
pub struct ActionGotoTable<'grammar, 'rules> {
    entries: BTreeMap<(usize, Symbol), Vec<TableEntry<'grammar, 'rules>>>,
    state_count: usize,
    entry_state: usize,
}

impl<'grammar: 'rules, 'rules> ActionGotoTable<'grammar, 'rules> {
    fn new(state_count: usize, entry_state: usize) -> Self {
        ActionGotoTable {
            entries: BTreeMap::new(),
            state_count,
            entry_state,
        }
    }

    pub fn get_entry(&self, state: usize, symbol: Symbol) -> Option<&Vec<TableEntry>> {
        self.entries.get(&(state, symbol))
    }

    pub fn iter_state_terminals(
        &self,
        state: usize,
        grammar: &'grammar Grammar,
    ) -> impl Iterator<Item = (Symbol, Option<&Vec<TableEntry>>)> {
        grammar
            .terminals()
            .chain(std::iter::once(Symbol::End))
            .map(move |s| (s, self.get_entry(state, s)))
    }

    pub fn iter_state_non_terminals(
        &self,
        state: usize,
        grammar: &'grammar Grammar,
    ) -> impl Iterator<Item = (Symbol, Option<&Vec<TableEntry>>)> {
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

    fn insert_reduce(&mut self, state: NodeIndex, symbol: Symbol, rule: &'grammar Rule<'rules>) {
        self.entries
            .entry((state.index(), symbol))
            .or_insert(Vec::new())
            .push(TableEntry::Reduce { rule });
    }

    fn insert_shift(&mut self, state: NodeIndex, symbol: Symbol, target: NodeIndex) {
        self.entries
            .entry((state.index(), symbol))
            .or_insert(Vec::new())
            .push(TableEntry::Shift {
                target: target.index(),
            });
    }

    fn insert_error(&mut self, state: NodeIndex, symbol: Symbol) {
        self.entries
            .entry((state.index(), symbol))
            .or_insert(Vec::new())
            .push(TableEntry::Error);
    }

    fn insert_accept(&mut self, state: NodeIndex, symbol: Symbol) {
        self.entries
            .entry((state.index(), symbol))
            .or_insert(Vec::new())
            .push(TableEntry::Accept);
    }

    pub fn state_has_shift(&self, state: usize, grammar: &'grammar Grammar) -> bool {
        self.iter_state_non_terminals(state, grammar)
            .chain(self.iter_state_terminals(state, grammar))
            .filter_map(|(_s, e)| e)
            .flat_map(|e| e)
            .any(|e| {
                if let TableEntry::Shift { target: _ } = e {
                    true
                } else {
                    false
                }
            })
    }
}

pub enum GenerationResult<'grammar, 'rules, const N: usize> {
    NoConflicts(ActionGotoTable<'grammar, 'rules>),
    AllowedConflicts {
        table: ActionGotoTable<'grammar, 'rules>,
        conflicts: Vec<Conflict<'grammar, 'rules>>,
    },
    BadConflicts(Vec<Conflict<'grammar, 'rules>>),
}

pub fn generate_table<'grammar: 'rules, 'rules, const N: usize>(
    grammar: &'grammar Grammar<'rules>,
    allow_conflicts: bool,
    lalr: bool,
) -> GenerationResult<'grammar, 'rules, N> {
    let first_sets = if N > 0 {
        compute_first_sets(grammar)
    } else {
        BTreeMap::new()
    };
    let parser_graph = generate_parser_graph::<N>(grammar, &first_sets, lalr);
    let conflicts: Vec<Conflict> = find_conflicts(&parser_graph).into_iter().collect();
    if !allow_conflicts && !conflicts.is_empty() {
        return GenerationResult::BadConflicts(conflicts);
    }

    let table = build_table(parser_graph, grammar);

    if conflicts.is_empty() {
        GenerationResult::NoConflicts(table)
    } else {
        GenerationResult::AllowedConflicts { table, conflicts }
    }
}

fn build_table<'grammar, 'rules, const N: usize>(
    parser_graph: ParserGraph<'grammar, 'rules, N>,
    grammar: &Grammar<'rules>,
) -> ActionGotoTable<'grammar, 'rules> {
    let entry_state = parser_graph.entry_state.unwrap().index();
    let node_count = parser_graph.graph.node_indices().count();

    let mut table: ActionGotoTable<'grammar, 'rules> =
        ActionGotoTable::new(node_count, entry_state);
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
        let reachable_states: BTreeMap<Symbol, NodeIndex> = parser_graph
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
    table
}

pub fn output_table<'grammar, 'rules>(
    grammar: &'grammar Grammar,
    table: &ActionGotoTable<'grammar, 'rules>,
    output: &mut dyn Write,
) -> std::io::Result<()> {
    let rule_index_map: BTreeMap<*const Rule, usize> = grammar
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
            if let Some(entries) = table.get_entry(state, symbol) {
                match entries.as_slice() {
                    [TableEntry::Shift { target }] => {
                        let rule_id_text = format!("s{}", target);
                        write!(
                            output,
                            "{: <width$}|",
                            rule_id_text,
                            width = column_sizes[i]
                        )?;
                    }
                    [TableEntry::Reduce { rule }] => {
                        let rule_id_text =
                            format!("r{}", rule_index_map.get(&(*rule as *const Rule)).unwrap());
                        write!(
                            output,
                            "{: <width$}|",
                            rule_id_text,
                            width = column_sizes[i]
                        )?;
                    }
                    [TableEntry::Error] => {
                        write!(output, "{: <width$}|", "e", width = column_sizes[i])?
                    }
                    [TableEntry::Accept] => {
                        write!(output, "{: <width$}|", "a", width = column_sizes[i])?
                    }
                    _ => write!(output, "{: <width$}|", "c", width = column_sizes[i])?,
                }
            } else {
                write!(output, "{: <width$}|", "", width = column_sizes[i])?
            }
        }
        writeln!(output, "")?;
    }
    Ok(())
}
