use std::{collections::HashMap, io::Write};

use lapex_codegen::GeneratedCodeWriter;
use lapex_parser::{
    grammar::{Grammar, Rule, Symbol},
    lr_parser::{ActionGotoTable, LRParserCodeGen, TableEntry},
};
use quote::{__private::TokenStream, quote};

use crate::{get_non_terminal_enum_name, get_token_enum_name, RustLRParserCodeGen};

struct CodeWriter<'grammar> {
    grammar: &'grammar Grammar<'grammar>,
    parser_table: &'grammar ActionGotoTable<'grammar>,
    rule_index_map: HashMap<*const Rule, usize>,
    rules_by_non_terminal: HashMap<Symbol, Vec<&'grammar Rule>>,
}

impl<'grammar> CodeWriter<'grammar> {
    fn new(grammar: &'grammar Grammar, parser_table: &'grammar ActionGotoTable) -> Self {
        let mut rules_by_non_terminal = HashMap::new();
        for rule in grammar.rules() {
            if let Some(non_terminal) = rule.lhs() {
                rules_by_non_terminal
                    .entry(non_terminal)
                    .or_insert(Vec::new())
                    .push(rule);
            }
        }
        let rule_index_map: HashMap<*const Rule, usize> = grammar
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

impl<'grammar> CodeWriter<'grammar> {
    fn get_non_terminal_name(&self, non_terminal: &Symbol) -> String {
        let non_terminal_name = self
            .grammar
            .is_named_non_terminal(*non_terminal)
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
                if let Some(entry) = entry {
                    self.make_goto(symbol, state, entry, &mut gotos);
                }
            }
        }
        gotos
    }

    fn make_goto(
        &self,
        symbol: Symbol,
        state: usize,
        entry: &TableEntry<'_>,
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
                        #condition => Goto::State { state_id: #target },
                    });
                }
                TableEntry::Accept => {
                    gotos.push(quote! {
                        #condition => Goto::Accept,
                    });
                }
                _ => (),
            }
        }
    }

    fn make_actions(&self) -> Vec<TokenStream> {
        let mut actions: Vec<TokenStream> = Vec::new();
        for state in 0..self.parser_table.states() {
            let mut expected_symbols = Vec::new();
            for (symbol, entry) in self.parser_table.iter_state_terminals(state, self.grammar) {
                if let Some(entry) = entry {
                    self.extract_expected_symbols(entry, symbol, &mut expected_symbols);
                    self.make_action(symbol, state, entry, &mut actions);
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
                (#state, _) => Err(ParserError::UnexpectedToken { got: next_token, expected: vec![#(TokenType::#expected),*] }),
            });
        }
        actions
    }

    fn make_action(
        &self,
        symbol: Symbol,
        state: usize,
        entry: &TableEntry<'_>,
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
            match entry {
                TableEntry::Shift { target: _ } => {
                    actions.push(quote! {
                        #condition => Ok(Action::Shift),
                    });
                }
                TableEntry::Reduce { rule } => {
                    let rule_ptr = (*rule) as *const Rule;
                    let rule_index = self.rule_index_map.get(&rule_ptr).unwrap();
                    let rule_name: TokenStream = format!("Rule{}", rule_index).parse().unwrap();
                    actions.push(quote! {
                        #condition => Ok(Action::Reduce { rule: ReducedRule::#rule_name }),
                    });
                }
                _ => (),
            }
        }
    }

    fn extract_expected_symbols(
        &self,
        entry: &TableEntry<'grammar>,
        symbol: Symbol,
        expected_symbols: &mut Vec<Option<u32>>,
    ) {
        match entry {
            TableEntry::Shift { target: _ } | TableEntry::Reduce { rule: _ } => match symbol {
                Symbol::Terminal(token_index) => {
                    expected_symbols.push(Some(token_index));
                }
                Symbol::End => {
                    expected_symbols.push(None);
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
                State { state_id: usize },
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

            #[derive(Debug)]
            pub enum ParserError {
                UnexpectedToken {
                    got: TokenType,
                    expected: Vec<TokenType>
                }
            }

            impl std::error::Error for ParserError {}

            impl std::fmt::Display for ParserError {
                fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    match self {
                        ParserError::UnexpectedToken { got, expected } => write!(
                            f,
                            "Unexpected token {:?}, expected one of: {:?}",
                            got, expected
                        ),
                    }
                }
            }

            impl<T, F: FnMut() -> (TokenType, T), V: Visitor<T>> Parser<T, F, V> {
                pub fn new(token_function: F, visitor: V) -> Self {
                    Parser {
                        token_function,
                        visitor,
                    }
                }

                fn next_action(&self, state: usize, next_token: TokenType) -> Result<Action, ParserError> {
                    match (state, next_token) {
                        #(#actions)*
                        (_, _) => unreachable!()
                    }
                }

                fn next_goto(&self, state: usize, symbol: StackSymbol) -> Goto {
                    match (state, symbol) {
                        #(#gotos)*
                        (_, _) => unreachable!()
                    }
                }

                fn reduce_stack_and_visit(&mut self, rule: ReducedRule, stack: &mut Vec<StackSymbol>) {
                    let (to_pop, reduced) = match rule {
                        #(#rule_reductions),*
                    };
                    for _ in 0..to_pop {
                        stack.pop().unwrap();
                        stack.pop().unwrap();
                    }
                    stack.push(reduced);
                    match rule {
                        #(#rule_visits),*
                    }
                }

                pub fn parse(&mut self) -> Result<(), ParserError> {
                    let mut lookahead = std::collections::VecDeque::new();
                    lookahead.push_back((self.token_function)());

                    let mut stack = Vec::new();
                    stack.push(StackSymbol::State { state_id: #entry });

                    while !stack.is_empty() {
                        let (next_token, _) = lookahead.front().unwrap();
                        let state = match stack.last().unwrap() {
                            StackSymbol::State { state_id } => *state_id,
                            _ => unreachable!()
                        };
                        let action = self.next_action(state, *next_token)?;
                        match action {
                            Action::Shift => {
                                let (next_token, next_data) = lookahead.pop_front().unwrap();
                                stack.push(StackSymbol::Terminal { token: next_token });
                                self.visitor.shift(next_token, next_data);

                                lookahead.push_back((self.token_function)());
                            }
                            Action::Reduce { rule: reduced_rule } => {
                                self.reduce_stack_and_visit(reduced_rule, &mut stack);
                            }
                        }
                        let current_symbol = stack.last().unwrap();
                        let state = match &stack[stack.len() - 2] {
                            StackSymbol::State { state_id } => *state_id,
                            _ => unreachable!()
                        };
                        let goto = self.next_goto(state, *current_symbol);
                        match goto {
                            Goto::Accept => {
                                stack.pop();
                                stack.pop();
                            }
                            Goto::State { state_id } => {
                                stack.push(StackSymbol::State { state_id })
                            }
                        }
                    }
                    Ok(())
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
        self.write_parser(output)?;
        Ok(())
    }
}

fn get_rule_from_pointer(rule: &*const Rule) -> &Rule {
    // We created the hashmap from a known list of rules. The rule pointers are derived from the grammar rules, and the grammar outlives this struct.
    // Therefore, this operation is safe.
    let rule = unsafe { rule.as_ref() }.unwrap();
    rule
}

impl LRParserCodeGen for RustLRParserCodeGen {
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
