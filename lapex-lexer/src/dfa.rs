use crate::alphabet;
use crate::nfa::{self, NfaEdge, NfaState};
use lapex_input::TokenRule;
use petgraph::prelude::DiGraph;
use petgraph::visit::{EdgeRef, IntoNodeReferences};
use petgraph::{graph::NodeIndex, Graph};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::ops::RangeInclusive;

fn epsilon_closure(
    graph: &Graph<NfaState, NfaEdge>,
    start_nodes: Vec<NodeIndex>,
    nodes: &mut HashSet<NodeIndex>,
) {
    for start_node in start_nodes {
        nodes.insert(start_node);
        let edges = graph.edges_directed(start_node, petgraph::Direction::Outgoing);
        for edge in edges {
            if let NfaEdge::Epsilon = edge.weight() {
                let target = edge.target();
                if !nodes.contains(&target) {
                    epsilon_closure(graph, vec![target], nodes);
                }
            }
        }
    }
}

fn add_powerset_to_dfa(
    nfa: &Graph<NfaState, NfaEdge>,
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
                if let NfaEdge::Alphabet(c) = edge.weight() {
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

pub enum DfaState {
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
    nfa: &Graph<NfaState, NfaEdge>,
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
            Some(NfaState::Final { token }) => {
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
    nfa: &Graph<NfaState, NfaEdge>,
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

pub fn generate_dfa(rules: Vec<TokenRule>) -> (Vec<RangeInclusive<u32>>, Graph<DfaState, usize>) {
    let alpha = alphabet::generate_alphabet(&rules);

    let (start, nfa) = nfa::generate_nfa(&alpha, &rules);

    (alpha.to_ranges(), powerset_construction(&nfa, start))
}
