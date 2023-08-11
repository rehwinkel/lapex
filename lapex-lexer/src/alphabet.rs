use std::{collections::BTreeSet, ops::RangeInclusive};

use lapex_input::{Characters, Pattern, TokenPattern, TokenRule};

#[derive(Debug)]
pub struct Alphabet {
    ranges: Vec<RangeInclusive<u32>>,
}

impl Alphabet {
    pub fn find_range(&self, ch: u32) -> Option<usize> {
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

    pub fn into_ranges(self) -> Vec<RangeInclusive<u32>> {
        self.ranges
    }

    pub fn get_ranges(&self) -> &Vec<RangeInclusive<u32>> {
        &self.ranges
    }
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

pub fn generate_alphabet(rules: &[TokenRule]) -> Alphabet {
    let mut chars = BTreeSet::new();
    for rule in rules {
        match rule.pattern() {
            TokenPattern::Literal { characters } => {
                get_chars_from_pattern(&mut chars, &Pattern::from_chars(characters))
            }
            TokenPattern::Pattern { pattern } => get_chars_from_pattern(&mut chars, pattern),
        }
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
