use std::{
    collections::{HashMap, HashSet},
    fmt::{Debug, Display},
    hash::Hash,
};

use petgraph::{
    graph::EdgeIndex,
    graph::NodeIndex,
    prelude::DiGraph,
    visit::{EdgeRef, IntoEdgesDirected, IntoNodeReferences},
    Direction::Outgoing,
    Graph,
};

pub enum AutomatonState<StateType> {
    Accepting(StateType),
    Intermediate(usize),
}

enum NfaEdge<TransitionType> {
    Epsilon,
    Transition(TransitionType),
}

impl<TransitionType: Debug> Debug for NfaEdge<TransitionType> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Epsilon => write!(f, "ε"),
            Self::Transition(t) => write!(f, "{:?}", t),
        }
    }
}

impl<TransitionType: Display> Display for NfaEdge<TransitionType> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Epsilon => write!(f, "ε"),
            Self::Transition(t) => write!(f, "{}", t),
        }
    }
}

pub struct Nfa<StateType, TransitionType> {
    graph: Graph<AutomatonState<StateType>, NfaEdge<TransitionType>>,
    intermediate_counter: usize,
}

impl<StateType, TransitionType> Nfa<StateType, TransitionType> {
    pub fn new() -> Self {
        Nfa {
            graph: DiGraph::new(),
            intermediate_counter: 0,
        }
    }

    pub fn add_intermediate_state(&mut self) -> NodeIndex {
        let added_node = self
            .graph
            .add_node(AutomatonState::Intermediate(self.intermediate_counter));
        // let intermediate_id = self.intermediate_counter; // TODO: return?
        self.intermediate_counter += 1;
        added_node
    }

    pub fn add_accepting_state(&mut self, state: StateType) -> NodeIndex {
        self.graph.add_node(AutomatonState::Accepting(state))
    }

    pub fn add_epsilon_transition(&mut self, start: NodeIndex, end: NodeIndex) -> EdgeIndex {
        self.graph.add_edge(start, end, NfaEdge::Epsilon)
    }

    pub fn add_transition(
        &mut self,
        start: NodeIndex,
        end: NodeIndex,
        transition: TransitionType,
    ) -> EdgeIndex {
        self.graph
            .add_edge(start, end, NfaEdge::Transition(transition))
    }
}

pub struct Dfa<StateType, TransitionType> {
    graph: Graph<AutomatonState<StateType>, TransitionType>,
    intermediate_counter: usize,
}

impl<StateType, TransitionType> Dfa<StateType, TransitionType> {
    pub fn new() -> Self {
        Dfa {
            graph: DiGraph::new(),
            intermediate_counter: 0,
        }
    }

    pub fn add_intermediate_state(&mut self) -> NodeIndex {
        let added_node = self
            .graph
            .add_node(AutomatonState::Intermediate(self.intermediate_counter));
        // let intermediate_id = self.intermediate_counter; // TODO: return?
        self.intermediate_counter += 1;
        added_node
    }

    pub fn add_accepting_state(&mut self, state: StateType) -> NodeIndex {
        self.graph.add_node(AutomatonState::Accepting(state))
    }

    pub fn add_transition(
        &mut self,
        start: NodeIndex,
        end: NodeIndex,
        transition: TransitionType,
    ) -> EdgeIndex {
        self.graph.add_edge(start, end, transition)
    }

    pub fn states(&self) -> impl Iterator<Item = (NodeIndex, &AutomatonState<StateType>)> {
        self.graph.node_references()
    }

    pub fn transitions_from(
        &self,
        node: NodeIndex,
    ) -> impl Iterator<Item = (&TransitionType, NodeIndex)> {
        self.graph
            .edges_directed(node, Outgoing)
            .map(|eref| (eref.weight(), eref.target()))
    }
}

impl<StateType: Clone, TransitionType: Clone + Eq + Hash> Nfa<StateType, TransitionType> {
    fn epsilon_closure(&self, start_nodes: Vec<NodeIndex>, closure: &mut HashSet<NodeIndex>) {
        for start_node in start_nodes {
            closure.insert(start_node);
            let edges = self
                .graph
                .edges_directed(start_node, petgraph::Direction::Outgoing);
            for edge in edges {
                if let NfaEdge::Epsilon = edge.weight() {
                    let target = edge.target();
                    if !closure.contains(&target) {
                        self.epsilon_closure(vec![target], closure);
                    }
                }
            }
        }
    }

    fn add_powerset_to_dfa(
        &self,
        dfa: &mut Graph<HashSet<NodeIndex>, TransitionType>,
        nodes: Vec<NodeIndex>,
    ) -> NodeIndex {
        let mut closure = HashSet::new(); // TODO: test perf of different data structures
        self.epsilon_closure(nodes, &mut closure);

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

            let mut target_multi_map: HashMap<TransitionType, Vec<NodeIndex>> = HashMap::new();
            for node in closure {
                let edges = self
                    .graph
                    .edges_directed(node, petgraph::Direction::Outgoing);
                for edge in edges {
                    if let NfaEdge::Transition(t) = edge.weight() {
                        let target = edge.target();
                        target_multi_map
                            .entry(t.clone())
                            .or_insert(Vec::new())
                            .push(target);
                    }
                }
            }
            for (t, targets) in target_multi_map {
                let target_dfa = self.add_powerset_to_dfa(dfa, targets);
                dfa.add_edge(node_dfa, target_dfa, t);
            }
            node_dfa
        }
    }

    fn convert_powerset_to_dfa(
        &self,
        powerset_dfa: &Graph<HashSet<NodeIndex>, TransitionType>,
        tmp_id: &mut usize,
        dfa: &mut Dfa<Vec<StateType>, TransitionType>,
        visited: &mut HashMap<NodeIndex, NodeIndex>,
        node: NodeIndex,
    ) -> NodeIndex {
        let mut accepts = Vec::new();
        let powerset = powerset_dfa.node_weight(node).unwrap();
        for nfa_index in powerset {
            let state = self.graph.node_weight(*nfa_index);
            if let Some(AutomatonState::Accepting(s)) = state {
                accepts.push(s.clone());
            }
        }
        let start = if !accepts.is_empty() {
            dfa.add_accepting_state(accepts)
        } else {
            dfa.add_intermediate_state()
        };
        visited.insert(node, start);

        for edge in powerset_dfa.edges_directed(node, petgraph::Direction::Outgoing) {
            let end = if let Some(end) = visited.get(&edge.target()) {
                *end
            } else {
                self.convert_powerset_to_dfa(powerset_dfa, tmp_id, dfa, visited, edge.target())
            };
            dfa.add_transition(start, end, edge.weight().clone());
        }
        start
    }

    pub fn powerset_construction(
        &self,
        entrypoint: NodeIndex,
    ) -> Dfa<Vec<StateType>, TransitionType> {
        let mut powerset_dfa: Graph<HashSet<NodeIndex>, TransitionType> = DiGraph::new();

        let start_dfa = self.add_powerset_to_dfa(&mut powerset_dfa, vec![entrypoint]);

        let mut tmp_id = 0;
        let mut dfa = Dfa::new();

        let mut visited = HashMap::new();
        self.convert_powerset_to_dfa(
            &powerset_dfa,
            &mut tmp_id,
            &mut dfa,
            &mut visited,
            start_dfa,
        );

        dfa
    }
}
