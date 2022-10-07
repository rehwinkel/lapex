use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
};

use lapex_input::{EntryRule, ProductionRule, TokenRule};

mod grammar;
use grammar::{Grammar, GrammarError};

//imod bnf;
//use bnf::{Bnf, BnfRule, Symbol};
//use petgraph::{data::Build, dot::Dot, graph::NodeIndex, prelude::DiGraph, Graph};

pub fn generate_table(entry: &EntryRule, tokens: &[TokenRule], rules: &[ProductionRule]) -> Result<(), grammar::GrammarError> {
    let grammar = Grammar::from_rules(entry, tokens, rules)?;
    println!("{:?}", grammar);
    Ok(())
}
