use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fmt::Debug,
    hash::Hash,
    ops::RangeInclusive,
};

use lapex_input::{Characters, Pattern, TokenRule};
use petgraph::{
    data::Build,
    graph::EdgeIndex,
    graph::NodeIndex,
    prelude::DiGraph,
    visit::{EdgeRef, IntoNodeReferences},
    Graph,
};

enum Connection {
    Epsilon,
    Alphabet(usize),
}

enum State {
    Final { token: String },
    Temporary { id: usize },
}

impl Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Epsilon => write!(f, "ε"),
            Self::Alphabet(b) => write!(f, "{}", b),
        }
    }
}

impl Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Final { token } => write!(f, "{}", token),
            Self::Temporary { id } => write!(f, "S{}", id),
        }
    }
}

struct Nfa {
    graph: Graph<State, Connection>,
    tmp_count: usize,
}

impl Nfa {
    fn add_tmp(&mut self) -> NodeIndex {
        self.tmp_count += 1;
        self.graph.add_node(State::Temporary { id: self.tmp_count })
    }

    fn add_final(&mut self, token: &str) -> NodeIndex {
        self.graph.add_node(State::Final {
            token: String::from(token),
        })
    }

    fn add_edge_epsilon(&mut self, start: NodeIndex, end: NodeIndex) -> EdgeIndex {
        self.graph.add_edge(start, end, Connection::Epsilon)
    }

    fn add_edge_byte(&mut self, start: NodeIndex, end: NodeIndex, index: usize) -> EdgeIndex {
        self.graph.add_edge(start, end, Connection::Alphabet(index))
    }
}

fn build_nfa_from_pattern(
    start: NodeIndex,
    end: NodeIndex,
    alphabet: &Alphabet,
    nfa: &mut Nfa,
    pattern: &Pattern,
) -> Option<()> {
    match &pattern {
        Pattern::Sequence { elements } => {
            if elements.len() > 0 {
                let mut start = start;
                for pat in &elements[..elements.len() - 1] {
                    let end = nfa.add_tmp();
                    build_nfa_from_pattern(start, end, alphabet, nfa, pat);
                    start = end;
                }
                build_nfa_from_pattern(start, end, alphabet, nfa, elements.last().unwrap());
            }
        }
        Pattern::Alternative { elements } => {
            for elem in elements {
                let inner_start = nfa.add_tmp();
                let inner_end = nfa.add_tmp();
                build_nfa_from_pattern(inner_start, inner_end, alphabet, nfa, elem);
                nfa.add_edge_epsilon(start, inner_start);
                nfa.add_edge_epsilon(inner_end, end);
            }
        }
        Pattern::Optional { inner } => {
            let inner_start = nfa.add_tmp();
            let inner_end = nfa.add_tmp();
            build_nfa_from_pattern(inner_start, inner_end, alphabet, nfa, inner);
            nfa.add_edge_epsilon(start, end);
            nfa.add_edge_epsilon(start, inner_start);
            nfa.add_edge_epsilon(inner_end, end);
        }
        Pattern::OneOrMany { inner } => {
            let inner_start = nfa.add_tmp();
            let inner_end = nfa.add_tmp();
            build_nfa_from_pattern(inner_start, inner_end, alphabet, nfa, inner);
            nfa.add_edge_epsilon(start, inner_start);
            nfa.add_edge_epsilon(inner_end, end);
            nfa.add_edge_epsilon(inner_end, inner_start);
        }
        Pattern::ZeroOrMany { inner } => {
            let inner_start = nfa.add_tmp();
            let inner_end = nfa.add_tmp();
            build_nfa_from_pattern(inner_start, inner_end, alphabet, nfa, inner);
            nfa.add_edge_epsilon(start, end);
            nfa.add_edge_epsilon(start, inner_start);
            nfa.add_edge_epsilon(inner_end, end);
            nfa.add_edge_epsilon(inner_end, inner_start);
        }
        Pattern::CharSet {
            chars: chars_vec,
            negated,
        } => {
            let mut indices = HashSet::new();
            for chars in chars_vec {
                match chars {
                    Characters::Single(ch) => {
                        let index = alphabet.find_range(*ch as u32)?;
                        indices.insert(index);
                    }
                    Characters::Range(rng_start, rng_end) => {
                        let index_start = alphabet.find_range(*rng_start as u32)?;
                        let index_end = alphabet.find_range(*rng_end as u32)?;
                        for i in index_start..=index_end {
                            indices.insert(i);
                        }
                    }
                }
            }
            if *negated {
                for i in 0..alphabet.ranges.len() {
                    if !indices.contains(&i) {
                        nfa.add_edge_byte(start, end, i);
                    }
                }
            } else {
                for i in indices {
                    nfa.add_edge_byte(start, end, i);
                }
            }
        }
        Pattern::Char { chars } => match chars {
            Characters::Single(ch) => {
                let index = alphabet.find_range(*ch as u32)?;
                nfa.add_edge_byte(start, end, index);
            }
            Characters::Range(rng_start, rng_end) => {
                let index_start = alphabet.find_range(*rng_start as u32)?;
                let index_end = alphabet.find_range(*rng_end as u32)?;
                for i in index_start..=index_end {
                    nfa.add_edge_byte(start, end, i);
                }
            }
        },
    }
    Some(())
}

fn get_chars_from_pattern(chars: &mut BTreeSet<char>, pattern: &Pattern) {
    match pattern {
        Pattern::Sequence { elements } => {
            for elem in elements {
                get_chars_from_pattern(chars, elem)
            }
        }
        Pattern::Alternative { elements } => {
            for elem in elements {
                get_chars_from_pattern(chars, elem)
            }
        }
        Pattern::Optional { inner } => get_chars_from_pattern(chars, inner),
        Pattern::OneOrMany { inner } => get_chars_from_pattern(chars, inner),
        Pattern::ZeroOrMany { inner } => get_chars_from_pattern(chars, inner),
        Pattern::CharSet {
            chars: ch,
            negated: _,
        } => {
            for ch in ch {
                match &ch {
                    Characters::Single(c) => {
                        chars.insert(*c);
                    }
                    Characters::Range(c1, c2) => {
                        chars.insert(*c1);
                        chars.insert(*c2);
                    }
                }
            }
        }
        Pattern::Char { chars: ch } => match &ch {
            Characters::Single(c) => {
                chars.insert(*c);
            }
            Characters::Range(c1, c2) => {
                chars.insert(*c1);
                chars.insert(*c2);
            }
        },
    }
}

#[derive(Debug)]
struct Alphabet {
    ranges: Vec<RangeInclusive<u32>>,
}

impl Alphabet {
    fn find_range(&self, ch: u32) -> Option<usize> {
        let search_result = self
            .ranges
            .binary_search_by_key(&ch, |range| *range.start());
        match search_result {
            Ok(index) => Some(index),
            Err(index) => {
                if self.ranges[index - 1].contains(&ch) {
                    Some(index - 1)
                } else {
                    None
                }
            }
        }
    }
}

fn generate_alphabet(rules: &Vec<TokenRule>) -> Alphabet {
    let mut chars = BTreeSet::new();
    for rule in rules {
        get_chars_from_pattern(&mut chars, rule.pattern())
    }
    chars.insert('\0');
    chars.insert(char::MAX);

    let mut ranges = Vec::new();
    let mut chars_iter = chars.iter();
    let mut prev = chars_iter.next().unwrap();
    ranges.push(RangeInclusive::new(*prev as u32, *prev as u32));
    for ch in chars_iter {
        if *ch as u32 - *prev as u32 > 1 {
            ranges.push(RangeInclusive::new(*prev as u32 + 1, *ch as u32 - 1));
        }
        ranges.push(RangeInclusive::new(*ch as u32, *ch as u32));
        prev = ch;
    }
    Alphabet { ranges }
}

fn epsilon_closure(
    graph: &Graph<State, Connection>,
    start_nodes: Vec<NodeIndex>,
    nodes: &mut HashSet<NodeIndex>,
) {
    for start_node in start_nodes {
        nodes.insert(start_node);
        let edges = graph.edges_directed(start_node, petgraph::Direction::Outgoing);
        for edge in edges {
            if let Connection::Epsilon = edge.weight() {
                let target = edge.target();
                if !nodes.contains(&target) {
                    epsilon_closure(graph, vec![target], nodes);
                }
            }
        }
    }
}

fn add_powerset_to_dfa(
    nfa: &Graph<State, Connection>,
    dfa: &mut Graph<HashSet<NodeIndex>, usize>,
    nodes: Vec<NodeIndex>,
) -> NodeIndex {
    let mut closure = HashSet::new(); // TODO: test perf of different data structures
    epsilon_closure(&nfa, nodes, &mut closure);

    // find an existing node with the same powerset
    let node_dfa_opt: Option<NodeIndex> = dfa
        .node_references()
        .find(|(_, w)| w == &&closure)
        .map(|(i, _)| i);
    if let Some(node_dfa) = node_dfa_opt {
        // if the powerset exists, no need to recompute
        node_dfa
    } else {
        // if the powerset is new, add it to the graph and recurse
        let node_dfa = dfa.add_node(closure.clone());

        let mut target_multi_map: HashMap<usize, Vec<NodeIndex>> = HashMap::new();
        for node in closure {
            let edges = nfa.edges_directed(node, petgraph::Direction::Outgoing);
            for edge in edges {
                if let Connection::Alphabet(c) = edge.weight() {
                    let target = edge.target();
                    target_multi_map
                        .entry(*c)
                        .or_insert(Vec::new())
                        .push(target);
                }
            }
        }
        for (c, targets) in target_multi_map {
            let target_dfa = add_powerset_to_dfa(nfa, dfa, targets);
            dfa.add_edge(node_dfa, target_dfa, c);
        }
        node_dfa
    }
}

enum DfaState {
    Temporary { id: usize },
    Accepting { accepts: Vec<String> },
}

impl Debug for DfaState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Temporary { id } => write!(f, "S{}", id),
            Self::Accepting { accepts } => write!(f, "{:?}", accepts),
        }
    }
}

fn convert_powerset_to_dfa(
    powerset_dfa: &Graph<HashSet<NodeIndex>, usize>,
    nfa: &Graph<State, Connection>,
    tmp_id: &mut usize,
    dfa: &mut Graph<DfaState, usize>,
    visited: &mut HashMap<NodeIndex, NodeIndex>,
    node: NodeIndex,
) -> NodeIndex {
    let mut accepts = Vec::new();
    let powerset = powerset_dfa.node_weight(node).unwrap();
    for nfa_index in powerset {
        let state = nfa.node_weight(*nfa_index);
        match state {
            Some(State::Final { token }) => {
                accepts.push(token.clone());
            }
            _ => (),
        }
    }
    let start = if !accepts.is_empty() {
        dfa.add_node(DfaState::Accepting { accepts })
    } else {
        *tmp_id += 1;
        dfa.add_node(DfaState::Temporary { id: *tmp_id })
    };
    visited.insert(node, start);

    for edge in powerset_dfa.edges_directed(node, petgraph::Direction::Outgoing) {
        let end = if let Some(end) = visited.get(&edge.target()) {
            *end
        } else {
            convert_powerset_to_dfa(powerset_dfa, nfa, tmp_id, dfa, visited, edge.target())
        };
        dfa.add_edge(start, end, *edge.weight());
    }
    start
}

fn powerset_construction(
    nfa: &Graph<State, Connection>,
    start: NodeIndex,
) -> Graph<DfaState, usize> {
    let mut powerset_dfa: Graph<HashSet<NodeIndex>, usize> = DiGraph::new();

    let start_dfa = add_powerset_to_dfa(nfa, &mut powerset_dfa, vec![start]);

    let mut tmp_id = 0;
    let mut dfa: Graph<DfaState, usize> = DiGraph::new();

    let mut visited = HashMap::new();
    convert_powerset_to_dfa(
        &powerset_dfa,
        nfa,
        &mut tmp_id,
        &mut dfa,
        &mut visited,
        start_dfa,
    );

    dfa
}

pub fn build_dfa(rules: Vec<TokenRule>) {
    let mut nfa = Nfa {
        graph: DiGraph::new(),
        tmp_count: 0,
    };

    let alphabet = generate_alphabet(&rules);

    for (i, range) in alphabet.ranges.iter().enumerate() {
        eprintln!("{} {:?}", i, range)
    }

    let start = nfa.add_tmp();
    for rule in rules {
        let rule_start = nfa.add_tmp();
        let rule_end = nfa.add_final(rule.token());
        nfa.add_edge_epsilon(start, rule_start);
        build_nfa_from_pattern(rule_start, rule_end, &alphabet, &mut nfa, rule.pattern());
    }

    let dfa = powerset_construction(&nfa.graph, start);
    // println!("{:?}", petgraph::dot::Dot::with_config(&nfa.graph, &[]));
    println!("{:?}", petgraph::dot::Dot::with_config(&dfa, &[]));
}
