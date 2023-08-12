use std::fmt::{Display, Formatter};
use std::ops::Range;

use lapex_input::{
    Characters, EntryRule, LapexInputParser, LapexParsingError, Pattern, ProductionPattern,
    ProductionRule, Rule, RuleSet, TokenPattern, TokenRule,
};
use nom::character::complete::{multispace0, multispace1};
use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_while1, take_while_m_n},
    character::complete::space1,
    combinator::{map, opt},
    multi::{many1, separated_list1},
    IResult,
};

fn parse_char_unescaped(input: &[u8]) -> IResult<&[u8], char> {
    let (input, ch) = take_while_m_n(1, 1, |c: u8| {
        let ch: char = c.into();
        ch.is_ascii()
            && ch != ']'
            && ch != '/'
            && ch != '\\'
            && ch != ')'
            && ch != '|'
            && ch != '+'
            && ch != '*'
            && ch != '?'
            && ch != '\t'
            && ch != '\r'
            && ch != '\n'
    })(input)?;
    let ch: char = ch[0].into();
    Ok((input, ch))
}

fn parse_char_escaped(input: &[u8]) -> IResult<&[u8], char> {
    let (input, _) = tag("\\")(input)?;
    let (input, ch) = take(1_usize)(input)?;
    let ch: char = ch[0].into();
    let ch = match ch {
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        'u' => {
            let (input, _) = tag("{")(input)?;
            let (input, code) = take_while_m_n(4, 6, |ch: u8| {
                let ch = Into::<char>::into(ch);
                ('0'..='9').contains(&ch) || ('a'..='f').contains(&ch) || ('A'..='F').contains(&ch)
            })(input)?;
            let (input, _) = tag("}")(input)?;
            if let Ok(code_str) = std::str::from_utf8(code) {
                if let Ok(codepoint) = u32::from_str_radix(code_str, 16) {
                    if let Some(ch) = std::char::from_u32(codepoint) {
                        return Ok((input, ch));
                    }
                }
            }
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )));
        }
        _ => {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )))
        }
    };
    Ok((input, ch))
}

fn parse_char(input: &[u8]) -> IResult<&[u8], char> {
    alt((parse_char_unescaped, parse_char_escaped))(input)
}

fn parse_char_range(input: &[u8]) -> IResult<&[u8], Range<char>> {
    let (input, c1) = parse_char(input)?;
    let (input, _) = tag("-")(input)?;
    let (input, c2) = parse_char(input)?;
    Ok((input, c1..c2))
}

fn parse_char_or_range(input: &[u8]) -> IResult<&[u8], Characters> {
    alt((
        map(parse_char_range, |range| {
            Characters::Range(range.start, range.end)
        }),
        map(parse_char, Characters::Single),
    ))(input)
}

fn parse_char_set(input: &[u8]) -> IResult<&[u8], Pattern> {
    let (input, _) = tag("[")(input)?;
    let (input, negation_res) = opt(tag("^"))(input)?;
    let negated = negation_res.is_some();
    let (input, chars) = many1(parse_char_or_range)(input)?;
    let (input, _) = tag("]")(input)?;
    Ok((input, Pattern::CharSet { chars, negated }))
}

fn parse_regex_group(input: &[u8]) -> IResult<&[u8], Pattern> {
    let (input, _) = tag("(")(input)?;
    let (input, mut seqs) = separated_list1(tag("|"), parse_regex_sequence)(input)?;
    let (input, _) = tag(")")(input)?;
    if seqs.len() == 1 {
        Ok((input, seqs.remove(0)))
    } else {
        Ok((input, Pattern::Alternative { elements: seqs }))
    }
}

fn parse_regex_element(input: &[u8]) -> IResult<&[u8], Pattern> {
    alt((
        parse_regex_group,
        parse_char_set,
        map(parse_char, |ch| Pattern::Char {
            chars: Characters::Single(ch),
        }),
    ))(input)
}

fn parse_regex_repetition(input: &[u8]) -> IResult<&[u8], Pattern> {
    let (input, inner) = parse_regex_element(input)?;
    let (input, rep_kind) = parse_repetition_kind(input)?;
    let pattern = if let Some(rep) = rep_kind {
        match rep {
            0 => Pattern::ZeroOrMany {
                inner: Box::new(inner),
            },
            1 => Pattern::OneOrMany {
                inner: Box::new(inner),
            },
            2 => Pattern::Optional {
                inner: Box::new(inner),
            },
            _ => unreachable!(),
        }
    } else {
        inner
    };
    Ok((input, pattern))
}

fn parse_regex_sequence(input: &[u8]) -> IResult<&[u8], Pattern> {
    let (input, elements) = many1(parse_regex_repetition)(input)?;
    Ok((input, Pattern::Sequence { elements }))
}

fn parse_regex_pattern(input: &[u8]) -> IResult<&[u8], TokenPattern> {
    let (input, _) = tag("/")(input)?;
    let (input, seq) = parse_regex_sequence(input)?;
    let (input, _) = tag("/")(input)?;
    Ok((input, TokenPattern::Pattern { pattern: seq }))
}

fn parse_literal_pattern(input: &[u8]) -> IResult<&[u8], TokenPattern> {
    let (input, _) = tag("\"")(input)?;
    let (input, chars) = take_while1(|c| {
        let ch = Into::<char>::into(c);
        ch != '"' && ch.is_ascii()
    })(input)?;
    let (input, _) = tag("\"")(input)?;
    let characters: Vec<char> = chars.iter().map(|c| Into::<char>::into(*c)).collect();
    Ok((input, TokenPattern::Literal { characters }))
}

fn parse_pattern(input: &[u8]) -> IResult<&[u8], TokenPattern> {
    let (input, pattern) = alt((parse_literal_pattern, parse_regex_pattern))(input)?;
    Ok((input, pattern))
}

fn parse_token_rule(input: &[u8]) -> IResult<&[u8], TokenRule> {
    let (input, _) = tag("token")(input)?;
    let (input, _) = space1(input)?;
    let (input, name) = parse_symbol_name(input)?;
    let (input, _) = space1(input)?;
    let (input, _) = tag("=")(input)?;
    let (input, _) = space1(input)?;
    let (input, pattern) = parse_pattern(input)?;
    let (input, _) = tag(";")(input)?;
    Ok((
        input,
        TokenRule {
            name: std::str::from_utf8(name).unwrap(),
            pattern,
        },
    ))
}

fn parse_rule_name(input: &[u8]) -> IResult<&[u8], ProductionPattern> {
    let (input, name) = parse_symbol_name(input)?;
    Ok((
        input,
        ProductionPattern::Rule {
            rule_name: String::from_utf8(name.to_vec()).unwrap(),
        },
    ))
}

fn parse_production_group(input: &[u8]) -> IResult<&[u8], ProductionPattern> {
    let (input, _) = tag("(")(input)?;
    let (input, mut seqs) = separated_list1(tag(" | "), parse_production_pattern)(input)?;
    let (input, _) = tag(")")(input)?;
    if seqs.len() == 1 {
        Ok((input, seqs.remove(0)))
    } else {
        Ok((input, ProductionPattern::Alternative { elements: seqs }))
    }
}

fn parse_production_element(input: &[u8]) -> IResult<&[u8], ProductionPattern> {
    alt((parse_production_group, parse_rule_name))(input)
}

fn parse_repetition_kind(input: &[u8]) -> IResult<&[u8], Option<i32>> {
    opt(alt((
        map(tag("*"), |_| 0),
        map(tag("+"), |_| 1),
        map(tag("?"), |_| 2),
    )))(input)
}

fn parse_production_regex_repetition(input: &[u8]) -> IResult<&[u8], ProductionPattern> {
    let (input, inner) = parse_production_element(input)?;
    let (input, rep_kind) = parse_repetition_kind(input)?;
    let pattern = if let Some(rep) = rep_kind {
        match rep {
            0 => ProductionPattern::ZeroOrMany {
                inner: Box::new(inner),
            },
            1 => ProductionPattern::OneOrMany {
                inner: Box::new(inner),
            },
            2 => ProductionPattern::Optional {
                inner: Box::new(inner),
            },
            _ => unreachable!(),
        }
    } else {
        inner
    };
    Ok((input, pattern))
}

fn parse_production_pattern(input: &[u8]) -> IResult<&[u8], ProductionPattern> {
    let (input, elements) = separated_list1(space1, parse_production_regex_repetition)(input)?;
    Ok((input, ProductionPattern::Sequence { elements }))
}

fn parse_production_rule(input: &[u8]) -> IResult<&[u8], ProductionRule> {
    let (input, _) = tag("prod")(input)?;
    let (input, _) = space1(input)?;
    let (input, name) = parse_symbol_name(input)?;
    let (input, _) = space1(input)?;
    let (input, _) = tag("=")(input)?;
    let (input, _) = space1(input)?;
    let (input, pattern) = parse_production_pattern(input)?;
    let (input, _) = tag(";")(input)?;
    Ok((
        input,
        ProductionRule {
            name: std::str::from_utf8(name).unwrap(),
            pattern,
        },
    ))
}

fn parse_symbol_name(input: &[u8]) -> IResult<&[u8], &[u8]> {
    take_while1(|c: u8| Into::<char>::into(c).is_ascii_alphabetic() || c == '_' as u8)(input)
}

fn parse_entry_rule(input: &[u8]) -> IResult<&[u8], EntryRule> {
    let (input, _) = tag("entry")(input)?;
    let (input, _) = space1(input)?;
    let (input, name) = parse_symbol_name(input)?;
    let (input, _) = tag(";")(input)?;
    Ok((
        input,
        EntryRule {
            name: std::str::from_utf8(name).unwrap(),
        },
    ))
}
fn parse_rule(input: &[u8]) -> IResult<&[u8], Rule> {
    alt((
        map(parse_token_rule, Rule::TokenRule),
        map(parse_production_rule, Rule::ProductionRule),
        map(parse_entry_rule, Rule::EntryRule),
    ))(input)
}

fn parse_lapex_file_raw(input: &[u8]) -> IResult<&[u8], Vec<Rule>> {
    let (input, _) = multispace0(input)?;
    let (input, rules) = separated_list1(multispace1, parse_rule)(input)?;
    let (input, _) = multispace0(input)?;
    Ok((input, rules))
}

pub struct BootstrapLapexInputParser;

impl LapexInputParser for BootstrapLapexInputParser {
    fn parse_lapex<'src>(&self, source: &'src str) -> Result<RuleSet<'src>, LapexParsingError> {
        parse_lapex_file(source.as_bytes())
    }
}

fn parse_lapex_file(input: &[u8]) -> Result<RuleSet, LapexParsingError> {
    let (remaining, rules) = parse_lapex_file_raw(input).unwrap();
    if !remaining.is_empty() {
        return Err(LapexParsingError::IncompleteParsing(
            String::from_utf8_lossy(&remaining).to_string(),
        ));
    }
    let mut token_rules = Vec::new();
    let mut prod_rules = Vec::new();
    let mut entry_rules = Vec::new();
    for rule in rules {
        match rule {
            Rule::TokenRule(tr) => token_rules.push(tr),
            Rule::ProductionRule(pr) => prod_rules.push(pr),
            Rule::EntryRule(er) => entry_rules.push(er),
        }
    }
    if entry_rules.len() == 0 {
        return Err(LapexParsingError::NoEntryRule);
    }
    if entry_rules.len() != 1 {
        return Err(LapexParsingError::TooManyEntryRules);
    }
    let rule_set = RuleSet {
        entry_rule: entry_rules.remove(0),
        token_rules,
        production_rules: prod_rules,
    };
    Ok(rule_set)
}

#[cfg(test)]
mod tests;
