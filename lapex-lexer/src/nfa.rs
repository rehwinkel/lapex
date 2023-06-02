use std::collections::HashSet;
use std::fmt::Debug;

use petgraph::{graph::EdgeIndex, graph::NodeIndex, prelude::DiGraph, Graph};

use lapex_input::{Characters, Pattern, TokenRule};

use crate::alphabet::Alphabet;

pub enum NfaEdge {
    Epsilon,
    Alphabet(usize),
}

pub enum NfaState {
    Final { token: String },
    Temporary { id: usize },
}

impl Debug for NfaEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Epsilon => write!(f, "ε"),
            Self::Alphabet(b) => write!(f, "{}", b),
        }
    }
}

impl Debug for NfaState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Final { token } => write!(f, "{}", token),
            Self::Temporary { id } => write!(f, "S{}", id),
        }
    }
}

struct Nfa {
    graph: Graph<NfaState, NfaEdge>,
    tmp_count: usize,
}

impl Nfa {
    fn add_tmp(&mut self) -> NodeIndex {
        self.tmp_count += 1;
        self.graph
            .add_node(NfaState::Temporary { id: self.tmp_count })
    }

    fn add_final(&mut self, token: &str) -> NodeIndex {
        self.graph.add_node(NfaState::Final {
            token: String::from(token),
        })
    }

    fn add_edge_epsilon(&mut self, start: NodeIndex, end: NodeIndex) -> EdgeIndex {
        self.graph.add_edge(start, end, NfaEdge::Epsilon)
    }

    fn add_edge_byte(&mut self, start: NodeIndex, end: NodeIndex, index: usize) -> EdgeIndex {
        self.graph.add_edge(start, end, NfaEdge::Alphabet(index))
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
            if !elements.is_empty() {
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
                for i in 0..alphabet.get_ranges().len() {
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

pub fn generate_nfa(
    alpha: &Alphabet,
    rules: &[TokenRule],
) -> (NodeIndex, Graph<NfaState, NfaEdge>) {
    let mut nfa = Nfa {
        graph: DiGraph::new(),
        tmp_count: 0,
    };

    let start = nfa.add_tmp();
    for rule in rules {
        let rule_start = nfa.add_tmp();
        let rule_end = nfa.add_final(rule.token());
        nfa.add_edge_epsilon(start, rule_start);
        build_nfa_from_pattern(rule_start, rule_end, alpha, &mut nfa, rule.pattern());
    }
    (start, nfa.graph)
}
