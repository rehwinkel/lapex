use std::{
    collections::{BTreeSet, HashSet},
    fmt::Debug,
    ops::RangeInclusive,
};

use lapex_input::{Characters, Pattern, TokenRule};
use petgraph::{graph::EdgeIndex, graph::NodeIndex, prelude::DiGraph, Graph};

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

fn build_pattern(
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
                    build_pattern(start, end, alphabet, nfa, pat);
                    start = end;
                }
                build_pattern(start, end, alphabet, nfa, elements.last().unwrap());
            }
        }
        Pattern::Alternative { elements } => {
            for elem in elements {
                let inner_start = nfa.add_tmp();
                let inner_end = nfa.add_tmp();
                build_pattern(inner_start, inner_end, alphabet, nfa, elem);
                nfa.add_edge_epsilon(start, inner_start);
                nfa.add_edge_epsilon(inner_end, end);
            }
        }
        Pattern::Optional { inner } => {
            let inner_start = nfa.add_tmp();
            let inner_end = nfa.add_tmp();
            build_pattern(inner_start, inner_end, alphabet, nfa, inner);
            nfa.add_edge_epsilon(start, end);
            nfa.add_edge_epsilon(start, inner_start);
            nfa.add_edge_epsilon(inner_end, end);
        }
        Pattern::OneOrMany { inner } => {
            let inner_start = nfa.add_tmp();
            let inner_end = nfa.add_tmp();
            build_pattern(inner_start, inner_end, alphabet, nfa, inner);
            nfa.add_edge_epsilon(start, inner_start);
            nfa.add_edge_epsilon(inner_end, end);
            nfa.add_edge_epsilon(inner_end, inner_start);
        }
        Pattern::ZeroOrMany { inner } => {
            let inner_start = nfa.add_tmp();
            let inner_end = nfa.add_tmp();
            build_pattern(inner_start, inner_end, alphabet, nfa, inner);
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
        let end = nfa.add_final(rule.token());
        build_pattern(start, end, &alphabet, &mut nfa, rule.pattern());
    }

    println!("{:?}", petgraph::dot::Dot::with_config(&nfa.graph, &[]));
}
