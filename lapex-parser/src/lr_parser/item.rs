use std::{fmt::Display, hash::Hash};

use crate::grammar::{Grammar, Rule, Symbol};

#[derive(Debug, Clone)]
pub struct Item<'grammar> {
    rule: &'grammar Rule,
    dot_position: usize,
}

pub struct ItemDisplay<'item, 'grammar> {
    item: &'item Item<'grammar>,
    grammar: &'grammar Grammar<'item>,
}

impl<'grammar> Item<'grammar> {
    pub fn display<'item>(
        &'item self,
        grammar: &'grammar Grammar<'grammar>,
    ) -> ItemDisplay<'item, 'grammar> {
        ItemDisplay {
            item: self,
            grammar: grammar,
        }
    }
}

impl<'rule, 'grammar> Display for ItemDisplay<'rule, 'grammar> {
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

impl<'grammar> From<&'grammar Rule> for Item<'grammar> {
    fn from(rule: &'grammar Rule) -> Self {
        Item {
            dot_position: 0,
            rule,
        }
    }
}

impl<'grammar> Item<'grammar> {
    pub fn symbol_after_dot(&self) -> Option<Symbol> {
        self.rule.rhs().get(self.dot_position).map(|s| *s)
    }

    pub fn advance_dot(&mut self) -> bool {
        if self.dot_position < self.rule.rhs().len() {
            self.dot_position += 1;
            true
        } else {
            false
        }
    }

    pub fn rule(&self) -> &Rule {
        self.rule
    }
}

impl<'grammar> Display for Item<'grammar> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?} -> ", self.rule.lhs())?;
        write!(f, "{:?}", &self.rule.rhs()[0..self.dot_position])?;
        write!(f, " . ")?;
        write!(f, "{:?}", &self.rule.rhs()[self.dot_position..])
    }
}

impl<'grammar> PartialEq for Item<'grammar> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.rule, other.rule) && self.dot_position == other.dot_position
    }
}

impl<'grammar> Eq for Item<'grammar> {}

impl<'grammar> Hash for Item<'grammar> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(self.rule, state);
        self.dot_position.hash(state);
    }
}

impl<'grammar> PartialOrd for Item<'grammar> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(
            (self.rule as *const Rule)
                .partial_cmp(&(other.rule as *const Rule))?
                .then(self.dot_position.cmp(&other.dot_position)),
        )
    }
}

impl<'grammar> Ord for Item<'grammar> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.rule as *const Rule)
            .cmp(&(other.rule as *const Rule))
            .then(self.dot_position.cmp(&other.dot_position))
    }
}
