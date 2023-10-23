use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
};

use lapex_codegen::GeneratedCodeWriter;
use lapex_parser::{
    grammar::{Grammar, Rule, Symbol},
    lr_parser::{ActionGotoTable, LRParserCodeGen, TableEntry},
};
use quote::{__private::TokenStream, quote};

use crate::RustGLRParserCodeGen;
use crate::{get_non_terminal_enum_name, get_token_enum_name};

struct CodeWriter<'grammar, 'rules> {
    grammar: &'grammar Grammar<'grammar>,
    parser_table: &'grammar ActionGotoTable<'grammar, 'rules>,
    rule_index_map: BTreeMap<*const Rule<'rules>, usize>,
    rules_by_non_terminal: BTreeMap<Symbol, Vec<&'grammar Rule<'rules>>>,
}

impl<'grammar: 'rules, 'rules> CodeWriter<'grammar, 'rules> {
    fn new(grammar: &'grammar Grammar, parser_table: &'grammar ActionGotoTable) -> Self {
        let mut rules_by_non_terminal = BTreeMap::new();
        for rule in grammar.rules() {
            if let Some(non_terminal) = rule.lhs() {
                rules_by_non_terminal
                    .entry(non_terminal)
                    .or_insert(Vec::new())
                    .push(rule);
            }
        }
        let rule_index_map: BTreeMap<*const Rule, usize> = grammar
            .rules()
            .iter()
            .enumerate()
            .map(|(i, r)| (r as *const Rule, i))
            .collect();
        CodeWriter {
            grammar,
            parser_table,
            rule_index_map,
            rules_by_non_terminal,
        }
    }
}

impl<'grammar, 'rules> CodeWriter<'grammar, 'rules> {
    fn get_non_terminal_name(&self, non_terminal: &Symbol) -> String {
        let non_terminal_name = self
            .grammar
            .get_production_name(non_terminal)
            .map(|s| String::from(s))
            .unwrap_or_else(|| {
                if let Symbol::NonTerminal(index) = non_terminal {
                    format!("anon{}", index)
                } else {
                    unreachable!()
                }
            });
        non_terminal_name
    }

    fn write_visitor(&self, output: &mut dyn Write) -> std::io::Result<()> {
        let mut reduce_functions: Vec<TokenStream> = Vec::new();

        for (non_terminal, rules) in &self.rules_by_non_terminal {
            let non_terminal_name = self.get_non_terminal_name(non_terminal);
            if rules.len() != 1 {
                for (i, rule) in rules.iter().enumerate() {
                    let comment: TokenStream = format!("///{}", rule.display(self.grammar))
                        .parse()
                        .unwrap();
                    let function: TokenStream = format!("reduce_{}_{}", non_terminal_name, i + 1)
                        .parse()
                        .unwrap();
                    reduce_functions.push(quote! {
                        #comment
                        fn #function (&mut self);
                    });
                }
            } else {
                let comment: TokenStream = format!("///{}", rules[0].display(self.grammar))
                    .parse()
                    .unwrap();
                let function: TokenStream =
                    format!("reduce_{}", non_terminal_name).parse().unwrap();
                reduce_functions.push(quote! {
                    #comment
                    fn #function (&mut self);
                });
            }
        }

        let tokens = quote! {
            pub trait Visitor<T> {
                fn shift(&mut self, token: TokenType, data: T);
                #(#reduce_functions)*
            }
        };
        write!(output, "{}", tokens)
    }

    fn write_debug_visitor(&self, output: &mut dyn Write) -> std::io::Result<()> {
        let mut reduce_functions: Vec<TokenStream> = Vec::new();

        for (non_terminal, rules) in &self.rules_by_non_terminal {
            let non_terminal_name = self.get_non_terminal_name(non_terminal);
            if rules.len() != 1 {
                for (i, rule) in rules.iter().enumerate() {
                    let comment = format!("{}", rule.display(self.grammar));
                    let function: TokenStream = format!("reduce_{}_{}", non_terminal_name, i + 1)
                        .parse()
                        .unwrap();
                    reduce_functions.push(quote! {
                        fn #function(&mut self) {
                            println!(#comment);
                        }
                    });
                }
            } else {
                let comment = format!("{}", rules[0].display(self.grammar));
                let function: TokenStream =
                    format!("reduce_{}", non_terminal_name).parse().unwrap();
                reduce_functions.push(quote! {
                    fn #function(&mut self) {
                        println!(#comment);
                    }
                });
            }
        }

        let tokens = quote! {
            pub struct DebugVisitor {}

            impl Visitor<()> for DebugVisitor {
                fn shift(&mut self, token: TokenType, _data: ()) {
                    println!("shift {:?}", token);
                }

                #(#reduce_functions)*
            }
        };
        write!(output, "{}", tokens)
    }

    fn make_gotos(&self) -> Vec<TokenStream> {
        let mut gotos: Vec<TokenStream> = Vec::new();
        for state in 0..self.parser_table.states() {
            for (symbol, entry) in self
                .parser_table
                .iter_state_terminals(state, self.grammar)
                .chain(
                    self.parser_table
                        .iter_state_non_terminals(state, self.grammar),
                )
            {
                match entry.map(|v| v.as_slice()) {
                    Some([entry]) => {
                        self.make_goto(symbol, state, entry, &mut gotos);
                    }
                    Some(entries) => {
                        for entry in entries {
                            self.make_goto(symbol, state, entry, &mut gotos);
                        }
                    }
                    None => (),
                }
            }
        }
        gotos
    }

    fn make_goto(
        &self,
        symbol: Symbol,
        state: usize,
        entry: &TableEntry,
        gotos: &mut Vec<TokenStream>,
    ) {
        let condition = match symbol {
            Symbol::Terminal(token_index) => {
                let token: TokenStream =
                    get_token_enum_name(self.grammar.get_token_name(token_index))
                        .parse()
                        .unwrap();
                Some(quote! {
                    (#state, StackSymbol::Terminal { token: TokenType::#token })
                })
            }
            Symbol::NonTerminal(nt_index) => {
                let token: TokenStream =
                    get_non_terminal_enum_name(self.grammar, Symbol::NonTerminal(nt_index))
                        .parse()
                        .unwrap();
                Some(quote! {
                    (#state, StackSymbol::NonTerminal { non_terminal: NonTerminalType::#token })
                })
            }
            _ => None,
        };
        if let Some(condition) = condition {
            match entry {
                TableEntry::Shift { target } => {
                    gotos.push(quote! {
                        #condition => Some(Goto::State { state_id: #target }),
                    });
                }
                TableEntry::Accept => {
                    gotos.push(quote! {
                        #condition => Some(Goto::Accept),
                    });
                }
                _ => (),
            }
        }
    }

    fn make_actions(&self) -> Vec<TokenStream> {
        let mut actions: Vec<TokenStream> = Vec::new();
        for state in 0..self.parser_table.states() {
            let mut expected_symbols = BTreeSet::new();
            for (symbol, entry) in self.parser_table.iter_state_terminals(state, self.grammar) {
                match entry.map(|v| v.as_slice()) {
                    Some(entries) => {
                        for entry in entries {
                            self.extract_expected_symbols(entry, symbol, &mut expected_symbols);
                        }
                        self.make_action(symbol, state, entries, &mut actions);
                    }
                    None => (),
                }
            }
            let expected: Vec<TokenStream> = expected_symbols
                .into_iter()
                .map(|sym| {
                    if let Some(token_index) = sym {
                        get_token_enum_name(self.grammar.get_token_name(token_index))
                            .parse()
                            .unwrap()
                    } else {
                        quote! { EndOfFile }
                    }
                })
                .collect();
            actions.push(quote! {
                (#state, _) => Err(ParserError::UnexpectedToken {
                    got: next_token,
                    got_data: next_data,
                    expected: vec![#(TokenType::#expected),*],
                }),
            });
        }
        actions
    }

    fn make_action(
        &self,
        symbol: Symbol,
        state: usize,
        entries: &[TableEntry],
        actions: &mut Vec<TokenStream>,
    ) {
        let condition = match symbol {
            Symbol::Terminal(token_index) => {
                let token: TokenStream =
                    get_token_enum_name(self.grammar.get_token_name(token_index))
                        .parse()
                        .unwrap();
                Some(quote! {
                    (#state, TokenType::#token)
                })
            }
            Symbol::End => Some(quote! {
                (#state, TokenType::EndOfFile)
            }),
            _ => None,
        };
        if let Some(condition) = condition {
            let mut actions_for_entry = Vec::new();
            for entry in entries {
                match entry {
                    TableEntry::Shift { target: _ } => {
                        actions_for_entry.push(quote! {
                           Action::Shift
                        });
                    }
                    TableEntry::Reduce { rule } => {
                        let rule_ptr = (*rule) as *const Rule;
                        let rule_index = self.rule_index_map.get(&rule_ptr).unwrap();
                        let rule_name: TokenStream = format!("Rule{}", rule_index).parse().unwrap();
                        actions_for_entry.push(quote! {
                            Action::Reduce { rule: ReducedRule::#rule_name }
                        });
                    }
                    _ => (),
                }
            }
            if !actions_for_entry.is_empty() {
                actions.push(quote! {
                    #condition => Ok(&[#(#actions_for_entry,)*]),
                });
            }
        }
    }

    fn extract_expected_symbols(
        &self,
        entry: &TableEntry,
        symbol: Symbol,
        expected_symbols: &mut BTreeSet<Option<u32>>,
    ) {
        match entry {
            TableEntry::Shift { target: _ } | TableEntry::Reduce { rule: _ } => match symbol {
                Symbol::Terminal(token_index) => {
                    expected_symbols.insert(Some(token_index));
                }
                Symbol::End => {
                    expected_symbols.insert(None);
                }
                _ => (),
            },
            _ => (),
        }
    }

    fn make_rule_reductions(&self) -> Vec<TokenStream> {
        let mut rule_reductions: Vec<TokenStream> = Vec::new();
        for (rule, rule_index) in &self.rule_index_map {
            let rule = get_rule_from_pointer(rule);
            let symbols_to_reduce = rule
                .rhs()
                .iter()
                .filter(|s| if let Symbol::Epsilon = s { false } else { true })
                .count();
            let non_terminal: TokenStream =
                get_non_terminal_enum_name(self.grammar, rule.lhs().unwrap())
                    .parse()
                    .unwrap();
            let rule_name: TokenStream = format!("Rule{}", rule_index).parse().unwrap();
            rule_reductions.push(quote!{
                ReducedRule::#rule_name => (#symbols_to_reduce, StackSymbol::NonTerminal { non_terminal: NonTerminalType::#non_terminal })
            });
        }
        rule_reductions
    }

    fn make_rule_visits(&self) -> Vec<TokenStream> {
        let mut rule_visits: Vec<TokenStream> = Vec::new();

        for (non_terminal, rules) in &self.rules_by_non_terminal {
            let non_terminal_name = self.get_non_terminal_name(non_terminal);
            if rules.len() != 1 {
                for (i, rule) in rules.iter().enumerate() {
                    let rule_index = self.rule_index_map.get(&(*rule as *const Rule)).unwrap();
                    let rule_name: TokenStream = format!("Rule{}", rule_index).parse().unwrap();
                    let function: TokenStream = format!("reduce_{}_{}", non_terminal_name, i + 1)
                        .parse()
                        .unwrap();
                    rule_visits.push(quote! {
                        ReducedRule::#rule_name => self.visitor.#function ()
                    });
                }
            } else {
                let rule = rules[0];
                let rule_index = self.rule_index_map.get(&(rule as *const Rule)).unwrap();
                let rule_name: TokenStream = format!("Rule{}", rule_index).parse().unwrap();
                let function: TokenStream =
                    format!("reduce_{}", non_terminal_name).parse().unwrap();
                rule_visits.push(quote! {
                    ReducedRule::#rule_name => self.visitor.#function ()
                });
            }
        }
        rule_visits
    }

    fn write_parser(&self, output: &mut dyn Write) -> std::io::Result<()> {
        let entry = self.parser_table.entry_state();
        let actions = self.make_actions();
        let gotos = self.make_gotos();
        let rules: Vec<TokenStream> = self
            .rule_index_map
            .values()
            .map(|i| format!("Rule{}", i).parse().unwrap())
            .collect();
        let non_terminals: Vec<TokenStream> = self
            .grammar
            .non_terminals()
            .map(|nt| {
                get_non_terminal_enum_name(self.grammar, nt)
                    .parse()
                    .unwrap()
            })
            .collect();
        let rule_reductions: Vec<TokenStream> = self.make_rule_reductions();
        let rule_visits: Vec<TokenStream> = self.make_rule_visits();

        let tokens = quote! {
            pub struct Parser<T, F: FnMut() -> (TokenType, T), V: Visitor<T>> {
                token_function: F,
                visitor: V,
            }

            #[derive(Debug, Clone, Copy)]
            enum NonTerminalType {
                #(#non_terminals),*
            }

            #[derive(Debug, Clone, Copy)]
            enum StackSymbol {
                Terminal { token: TokenType },
                NonTerminal { non_terminal: NonTerminalType },
            }

            #[derive(Clone, Copy)]
            enum ReducedRule {
                #(#rules),*
            }

            enum Action {
                Shift,
                Reduce { rule: ReducedRule }
            }

            enum Goto {
                Accept,
                State { state_id: usize }
            }

            type StateId = usize;

            #[derive(Debug)]
            pub enum ParserError<T> {
                UnexpectedToken {
                    got: TokenType,
                    got_data: T,
                    expected: Vec<TokenType>,
                },
                UnexpectedTokens {
                    got: Vec<(TokenType, T)>,
                    expected: Vec<Vec<TokenType>>,
                },
            }

            impl<T: std::fmt::Debug> std::error::Error for ParserError<T> {}

            impl<T> std::fmt::Display for ParserError<T> {
                fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    match self {
                        ParserError::UnexpectedToken {
                            got,
                            got_data: _,
                            expected,
                        } => write!(
                            f,
                            "Unexpected token {:?}, expected one of: {:?}",
                            got, expected
                        ),
                        ParserError::UnexpectedTokens { got, expected } => {
                            let errors: Vec<String> = got
                                .iter()
                                .zip(expected.iter())
                                .map(|((got, _got_data), expected)| {
                                    format!(
                                        "Unexpected token {:?}, expected one of: {:?}",
                                        got, expected
                                    )
                                })
                                .collect();
                            write!(
                                f,
                                "Multiple diverging parse stacks reached unexpected ends:\n{}",
                                errors.join("\n")
                            )
                        }
                    }
                }
            }

            #[derive(Clone)]
            enum RecordedVisit<T> {
                Reduce { rule: ReducedRule },
                Shift { token: TokenType, data: T },
            }

            impl<T: Clone, F: FnMut() -> (TokenType, T), V: Visitor<T>> Parser<T, F, V> {
                pub fn new(token_function: F, visitor: V) -> Self {
                    Parser {
                        token_function,
                        visitor,
                    }
                }

                fn next_actions(&self, state: usize, next_token: TokenType, next_data: T) -> Result<&'static [Action], ParserError<T>> {
                    match (state, next_token) {
                        #(#actions)*
                        (_, _) => unreachable!()
                    }
                }

                fn next_goto(&self, state: &usize, symbol: &StackSymbol) -> Option<Goto> {
                    match (state, symbol) {
                        #(#gotos)*
                        (_, _) => None,
                    }
                }

                fn get_rule_reduction(&self, rule: &ReducedRule) -> (usize, StackSymbol) {
                    match rule {
                        #(#rule_reductions),*
                    }
                }

                fn do_visit(&mut self, rule: &ReducedRule) {
                    match rule {
                        #(#rule_visits),*
                    }
                }

                pub fn parse(&mut self) -> Result<(), ParserError<T>> {
                    let mut lookahead = std::collections::VecDeque::new();
                    lookahead.push_back((self.token_function)());

                    let root = GraphNode::root();
                    let stack = root.push(Some(#entry), None);
                    let mut stacks = vec![stack];

                    while !(stacks.len() == 1 && stacks[0].is_root()) {
                        let (next_token, next_data) = lookahead.front().unwrap();
                        let reduced = self
                            .apply_reduces(stacks, next_token, next_data)
                            .map_err(combine_errors)?;

                        let (next_token, next_data) = lookahead.pop_front().unwrap();
                        let new_symbol = StackSymbol::Terminal { token: next_token };
                        lookahead.push_back((self.token_function)());

                        let mut new_stacks = if reduced.iter().any(|s| s.top().is_none()) {
                            reduced
                        } else {
                            let mut new_stacks = Vec::new();
                            for stack in reduced {
                                let state = *stack.top().unwrap();
                                match self.next_goto(&state, &new_symbol) {
                                    Some(Goto::State { state_id }) => {
                                        stack.record(RecordedVisit::Shift {
                                            token: next_token,
                                            data: next_data.clone(),
                                        });
                                        let new_node = stack.push(Some(state_id), Some(new_symbol));
                                        new_stacks.push(new_node);
                                    }
                                    Some(Goto::Accept) => unreachable!(),
                                    None => (),
                                }
                            }
                            new_stacks
                        };
                        if new_stacks.len() == 1 {
                            let stack = new_stacks.pop().unwrap();
                            let recorded = stack.pop_recorded();
                            for record in recorded {
                                match record {
                                    RecordedVisit::Reduce { rule } => self.do_visit(&rule),
                                    RecordedVisit::Shift { token, data } => self.visitor.shift(token, data),
                                }
                            }
                            stacks = vec![stack];
                        } else {
                            stacks = new_stacks;
                        }
                    }
                    Ok(())
                }

                fn apply_reduces(
                    &mut self,
                    stacks: Vec<GraphNode<usize, StackSymbol, RecordedVisit<T>>>,
                    next_token: &TokenType,
                    next_data: &T
                ) -> Result<Vec<GraphNode<usize, StackSymbol, RecordedVisit<T>>>, Vec<ParserError<T>>> {
                    let mut to_reduce = stacks;
                    let mut reduced = Vec::new();
                    while !to_reduce.is_empty() {
                        let mut errors = Vec::new();
                        let all_error_count = to_reduce.len();
                        let mut new_to_reduce = Vec::new();
                        for stack in to_reduce {
                            let state = *stack.top().unwrap();
                            match self.next_actions(state, next_token.clone(), next_data.clone()) {
                                Ok(actions) => {
                                    for action in actions {
                                        match action {
                                            Action::Reduce { rule: reduced_rule } => {
                                                self.apply_reduce(
                                                    reduced_rule,
                                                    &stack,
                                                    &mut reduced,
                                                    &mut new_to_reduce,
                                                );
                                            }
                                            Action::Shift => {
                                                reduced.push(stack.clone_and_fork_record());
                                            }
                                        };
                                    }
                                }
                                Err(e) => {
                                    errors.push(e);
                                }
                            }
                        }
                        // if all reduces errored, the parser must have encountered an error
                        if reduced.is_empty() && errors.len() == all_error_count {
                            return Err(errors);
                        }
                        to_reduce = new_to_reduce;
                    }
                    Ok(reduced)
                }

                fn apply_reduce(
                    &mut self,
                    reduced_rule: &ReducedRule,
                    stack: &GraphNode<usize, StackSymbol, RecordedVisit<T>>,
                    accepted: &mut Vec<GraphNode<StateId, StackSymbol, RecordedVisit<T>>>,
                    new_to_reduce: &mut Vec<GraphNode<usize, StackSymbol, RecordedVisit<T>>>,
                ) {
                    let (to_pop, reduced_symbol) = self.get_rule_reduction(&reduced_rule);
                    let stacks_to_push = stack.unwind_stacks(to_pop);
                    for mut stack in stacks_to_push {
                        stack.record(RecordedVisit::Reduce {
                            rule: reduced_rule.clone(),
                        });
                        // remove reduced symbols
                        for _ in 0..to_pop {
                            let (_edge, new_stack) = stack.pop();
                            stack = new_stack;
                        }
                        let state = *stack.top().unwrap();
                        match self.next_goto(&state, &reduced_symbol) {
                            Some(Goto::State { state_id }) => {
                                // push new non-terminal
                                let new_node = stack.push(Some(state_id), Some(reduced_symbol));
                                new_to_reduce.push(new_node);
                            }
                            Some(Goto::Accept) => {
                                let (_edge, root) = stack.pop();
                                accepted.push(root);
                            }
                            None => (),
                        }
                    }
                }
            }

            fn combine_errors<T>(mut errors: Vec<ParserError<T>>) -> ParserError<T> {
                match errors.len() {
                    1 => errors.pop().unwrap(),
                    0 => unreachable!(),
                    _ => {
                        let (got, expected): (Vec<(TokenType, T)>, Vec<Vec<TokenType>>) = errors
                            .into_iter()
                            .map(|e| match e {
                                ParserError::UnexpectedToken {
                                    got,
                                    got_data,
                                    expected,
                                } => ((got, got_data), expected),
                                _ => unreachable!(),
                            })
                            .unzip();
                        ParserError::UnexpectedTokens { got, expected }
                    }
                }
            }

            use gss::GraphNode;

            mod gss {
                use std::{
                    cell::{Ref, RefCell},
                    rc::Rc,
                };

                pub struct GraphNode<N, E, R> {
                    inner: Rc<RefCell<GraphNodeInner<N, E, R>>>,
                    recorded: Rc<RefCell<Vec<R>>>,
                }

                impl<N: Clone, E: Clone, R: Clone> GraphNode<N, E, R> {
                    pub fn clone_and_fork_record(&self) -> Self {
                        GraphNode {
                            inner: self.inner.clone(),
                            recorded: Rc::new(RefCell::new(self.recorded.borrow().clone())),
                        }
                    }

                    pub fn unwind_stacks(&self, depth: usize) -> Vec<Self> {
                        if depth == 0 {
                            return vec![self.clone_and_fork_record()];
                        }
                        let mut resulting_parents = Vec::new();
                        let value = self.top().map(|r| r.clone());
                        for (edge, neighbor) in self.neighbors().iter() {
                            let new_parents = neighbor.unwind_stacks(depth - 1);
                            for parent in new_parents {
                                let mut new_node = parent.push(value.clone(), edge.clone());
                                new_node.recorded = self.recorded.clone();
                                resulting_parents.push(new_node.clone_and_fork_record());
                            }
                        }
                        resulting_parents
                    }
                }

                impl<N, E, R> GraphNode<N, E, R> {
                    pub fn root() -> Self {
                        GraphNode {
                            inner: Rc::new(RefCell::new(GraphNodeInner {
                                node_value: None,
                                neighbors: vec![],
                            })),
                            recorded: Rc::new(RefCell::new(Vec::new())),
                        }
                    }

                    fn add_edge(&mut self, value: Option<E>, predecessor: GraphNode<N, E, R>) {
                        self.inner.borrow_mut().neighbors.push((value, predecessor));
                    }

                    pub fn top(&self) -> Option<Ref<N>> {
                        let opt = Ref::filter_map(self.inner.borrow(), |i| i.node_value.as_ref());
                        match opt {
                            Ok(r) => Some(r),
                            Err(_) => None,
                        }
                    }

                    fn neighbors(&self) -> Ref<[(Option<E>, GraphNode<N, E, R>)]> {
                        Ref::map(self.inner.borrow(), |i| i.neighbors.as_slice())
                    }

                    pub fn pop(self) -> (Option<E>, Self) {
                        let neighbors = &mut self.inner.borrow_mut().neighbors;
                        assert_eq!(
                            neighbors.len(),
                            1,
                            "Tried to pop from stack branch with more/less than one predecessor"
                        );
                        if let Some((e, mut node)) = neighbors.pop() {
                            node.recorded = self.recorded;
                            (e, node)
                        } else {
                            panic!("Tried to pop from stack branch with zero predecessors");
                        }
                    }

                    pub fn pop_recorded(&self) -> Vec<R> {
                        return self.recorded.borrow_mut().split_off(0);
                    }

                    pub fn record(&self, record: R) {
                        self.recorded.borrow_mut().push(record);
                    }

                    pub fn is_root(&self) -> bool {
                        self.inner.borrow().node_value.is_none()
                    }

                    pub fn push(self, value: Option<N>, edge: Option<E>) -> GraphNode<N, E, R> {
                        let mut new_node = self.new_with_same_record(value);
                        new_node.add_edge(edge, self);
                        new_node
                    }

                    fn new_with_same_record(&self, node_value: Option<N>) -> Self {
                        GraphNode {
                            inner: Rc::new(RefCell::new(GraphNodeInner {
                                node_value,
                                neighbors: vec![],
                            })),
                            recorded: self.recorded.clone(),
                        }
                    }
                }

                struct GraphNodeInner<N, E, R> {
                    node_value: Option<N>,
                    neighbors: Vec<(Option<E>, GraphNode<N, E, R>)>,
                }
            }
        };
        write!(output, "{}", tokens)
    }

    fn write_visitor_and_parser(&self, output: &mut dyn Write) -> std::io::Result<()> {
        write!(
            output,
            "{}",
            quote! {
                use super::tokens::TokenType;
            }
        )?;
        self.write_visitor(output)?;
        self.write_debug_visitor(output)?;
        self.write_parser(output)?;
        Ok(())
    }
}

fn get_rule_from_pointer<'a, 'rules>(rule: &*const Rule<'rules>) -> &'a Rule<'rules> {
    // We created the hashmap from a known list of rules. The rule pointers are derived from the grammar rules, and the grammar outlives this struct.
    // Therefore, this operation is safe.
    let rule = unsafe { rule.as_ref() }.unwrap();
    rule
}

impl LRParserCodeGen for RustGLRParserCodeGen {
    fn generate_code(
        &self,
        grammar: &Grammar,
        parser_table: &ActionGotoTable,
        gen: &mut GeneratedCodeWriter,
    ) {
        let writer = CodeWriter::new(grammar, parser_table);
        gen.generate_code("parser.rs", |output| {
            writer.write_visitor_and_parser(output)
        })
        .unwrap();
    }
}
