use std::{fmt::Display, hash::Hash};

use lapex_input::{ProductionRule, Spanned};

use crate::grammar::{Grammar, Rule, Symbol};

#[derive(Debug, Clone)]
pub struct Item<'grammar, 'rules, const N: usize> {
    rule: &'grammar Rule<'rules>,
    dot_position: usize,
    lookahead: [Symbol; N],
}

pub struct ItemDisplay<'item, 'grammar, 'rules, const N: usize> {
    item: &'item Item<'grammar, 'rules, N>,
    grammar: &'grammar Grammar<'item>,
}

impl<'grammar, 'rules, const N: usize> Item<'grammar, 'rules, N> {
    pub fn display<'item>(
        &'item self,
        grammar: &'grammar Grammar<'grammar>,
    ) -> ItemDisplay<'item, 'grammar, 'rules, N> {
        ItemDisplay {
            item: self,
            grammar: grammar,
        }
    }
}

impl<'rule, 'grammar, 'rules, const N: usize> Display for ItemDisplay<'rule, 'grammar, 'rules, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let rhs_sequence_pre_dot: Vec<String> = self
            .item
            .rule
            .rhs()
            .into_iter()
            .take(self.item.dot_position)
            .map(|s| self.grammar.get_symbol_name(s))
            .collect();
        let rhs_sequence_post_dot: Vec<String> = self
            .item
            .rule
            .rhs()
            .into_iter()
            .skip(self.item.dot_position)
            .map(|s| self.grammar.get_symbol_name(s))
            .collect();
        if let Some(lhs) = &self.item.rule.lhs() {
            write!(
                f,
                "{} -> {} • {}",
                self.grammar.get_symbol_name(lhs),
                rhs_sequence_pre_dot.join(" "),
                rhs_sequence_post_dot.join(" ")
            )
        } else {
            write!(
                f,
                "{} • {}",
                rhs_sequence_pre_dot.join(" "),
                rhs_sequence_post_dot.join(" ")
            )
        }
    }
}

impl<'grammar, 'rules, const N: usize> Item<'grammar, 'rules, N> {
    pub fn new(rule: &'grammar Rule<'rules>, lookahead: [Symbol; N]) -> Self {
        Item {
            dot_position: 0,
            rule,
            lookahead: lookahead,
        }
    }

    pub fn lookahead(&self) -> &[Symbol; N] {
        &self.lookahead
    }

    pub fn production(&self) -> &'grammar Spanned<ProductionRule<'rules>> {
        self.rule.rule()
    }
}

impl<'grammar, 'rules, const N: usize> Item<'grammar, 'rules, N> {
    pub fn symbol_after_dot_offset(&self, offset: usize) -> Option<Symbol> {
        self.rule.rhs().get(self.dot_position + offset).map(|s| *s)
    }

    pub fn symbol_after_dot(&self) -> Option<Symbol> {
        self.symbol_after_dot_offset(0)
    }

    pub fn symbols_following_symbol_after_dot(&self) -> impl Iterator<Item = Symbol> + 'grammar {
        self.rule
            .rhs()
            .iter()
            .skip(self.dot_position + 1)
            .map(|s| *s)
    }

    pub fn advance_dot(&mut self) -> bool {
        if self.dot_position < self.rule.rhs().len() {
            self.dot_position += 1;
            true
        } else {
            false
        }
    }

    pub fn rule(&self) -> &'grammar Rule<'rules> {
        self.rule
    }
}

impl<'grammar, 'rules, const N: usize> Display for Item<'grammar, 'rules, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?} -> ", self.rule.lhs())?;
        write!(f, "{:?}", &self.rule.rhs()[0..self.dot_position])?;
        write!(f, " . ")?;
        write!(f, "{:?}", &self.rule.rhs()[self.dot_position..])?;
        write!(f, " {:?}", &self.lookahead)
    }
}

impl<'grammar, 'rules, const N: usize> PartialEq for Item<'grammar, 'rules, N> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.rule, other.rule)
            && self.dot_position == other.dot_position
            && self.lookahead == other.lookahead
    }
}

impl<'grammar, 'rules, const N: usize> Eq for Item<'grammar, 'rules, N> {}

impl<'grammar, 'rules, const N: usize> Hash for Item<'grammar, 'rules, N> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(self.rule, state);
        self.dot_position.hash(state);
        self.lookahead.hash(state);
    }
}

impl<'grammar, 'rules, const N: usize> PartialOrd for Item<'grammar, 'rules, N> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(
            (self.rule as *const Rule)
                .partial_cmp(&(other.rule as *const Rule))?
                .then(self.dot_position.cmp(&other.dot_position)),
        )
    }
}

impl<'grammar, 'rules, const N: usize> Ord for Item<'grammar, 'rules, N> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.rule as *const Rule)
            .cmp(&(other.rule as *const Rule))
            .then(self.dot_position.cmp(&other.dot_position))
            .then(self.lookahead.cmp(&other.lookahead))
    }
}
