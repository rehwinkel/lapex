use std::{
    fmt::{Debug, Display},
    hash::Hash,
    ops::Deref,
};

use lapex_input::{ProductionRule, Spanned};

use crate::grammar::{Grammar, Rule, Symbol};

type DotIdx = u8;

struct RuleRef<'grammar, 'rules>(pub &'grammar Rule<'rules>);

impl<'grammar, 'rules> Clone for RuleRef<'grammar, 'rules> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl<'grammar, 'rules> Deref for RuleRef<'grammar, 'rules> {
    type Target = Rule<'rules>;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'grammar, 'rules> Debug for RuleRef<'grammar, 'rules> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", (self.0) as *const Rule)
    }
}

impl<'grammar, 'rules> PartialEq for RuleRef<'grammar, 'rules> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl<'grammar, 'rules> Eq for RuleRef<'grammar, 'rules> {}

impl<'grammar, 'rules> PartialOrd for RuleRef<'grammar, 'rules> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (self.0 as *const Rule).partial_cmp(&(other.0 as *const Rule))
    }
}

impl<'grammar, 'rules> Ord for RuleRef<'grammar, 'rules> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.0 as *const Rule).cmp(&(other.0 as *const Rule))
    }
}

impl<'grammar, 'rules> Hash for RuleRef<'grammar, 'rules> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(self.0, state);
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Item<'grammar, 'rules, const N: usize> {
    rule: RuleRef<'grammar, 'rules>,
    dot_position: DotIdx,
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
            .take(self.item.dot_position as usize)
            .map(|s| self.grammar.get_symbol_name(s))
            .collect();
        let rhs_sequence_post_dot: Vec<String> = self
            .item
            .rule
            .rhs()
            .into_iter()
            .skip(self.item.dot_position as usize)
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
            rule: RuleRef(rule),
            lookahead: lookahead,
        }
    }

    pub fn to_lr0(&self) -> Item<'grammar, 'rules, 0> {
        Item {
            dot_position: self.dot_position,
            rule: RuleRef(self.rule.0),
            lookahead: [],
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
    pub fn symbol_after_dot_offset(&self, offset: DotIdx) -> Option<Symbol> {
        self.rule
            .rhs()
            .get(self.dot_position.checked_add(offset).unwrap() as usize)
            .map(|s| *s)
    }

    pub fn symbol_after_dot(&self) -> Option<Symbol> {
        self.symbol_after_dot_offset(0)
    }

    pub fn symbols_following_symbol_after_dot(&self) -> impl Iterator<Item = Symbol> + 'grammar {
        self.rule
            .0
            .rhs()
            .iter()
            .skip(self.dot_position.checked_add(1).unwrap() as usize)
            .map(|s| *s)
    }

    pub fn advance_dot(&mut self) -> bool {
        if (self.dot_position as usize) < self.rule.rhs().len() {
            self.dot_position = self.dot_position.checked_add(1).unwrap();
            true
        } else {
            false
        }
    }

    pub fn rule(&self) -> &'grammar Rule<'rules> {
        self.rule.0
    }
}

impl<'grammar, 'rules, const N: usize> Display for Item<'grammar, 'rules, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?} -> ", self.rule.lhs())?;
        write!(f, "{:?}", &self.rule.rhs()[0..(self.dot_position as usize)])?;
        write!(f, " . ")?;
        write!(f, "{:?}", &self.rule.rhs()[(self.dot_position as usize)..])?;
        write!(f, " {:?}", &self.lookahead)
    }
}
