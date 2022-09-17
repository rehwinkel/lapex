use std::ops::Range;

use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_while1, take_while_m_n},
    character::complete::{newline, space1},
    combinator::{map, opt},
    multi::{many1, separated_list1},
    IResult,
};

#[derive(Debug, Clone)]
pub enum Characters {
    Single(char),
    Range(char, char),
}

#[derive(Debug)]
pub enum Pattern {
    Sequence {
        elements: Vec<Pattern>,
    },
    Alternative {
        elements: Vec<Pattern>,
    },
    OneOrMany {
        inner: Box<Pattern>,
    },
    ZeroOrMany {
        inner: Box<Pattern>,
    },
    Optional {
        inner: Box<Pattern>,
    },
    CharSet {
        chars: Vec<Characters>,
        negated: bool,
    },
    Char {
        chars: Characters,
    },
}

#[derive(Debug)]
pub struct TokenRule<'src> {
    name: &'src str,
    pattern: Pattern,
}

impl<'src> TokenRule<'src> {
    pub fn token(&self) -> &str {
        self.name
    }

    pub fn pattern(&self) -> &Pattern {
        &self.pattern
    }
}

fn parse_char_unescpaed<'src>(input: &'src [u8]) -> IResult<&'src [u8], char> {
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
    })(input)?;
    let ch: char = ch[0].into();
    Ok((input, ch))
}

fn parse_char_escaped<'src>(input: &'src [u8]) -> IResult<&'src [u8], char> {
    let (input, _) = tag("\\")(input)?;
    let (input, ch) = take(1_usize)(input)?;
    let ch: char = ch[0].into();
    Ok((input, ch))
}

fn parse_char<'src>(input: &'src [u8]) -> IResult<&'src [u8], char> {
    alt((parse_char_unescpaed, parse_char_escaped))(input)
}

fn parse_char_range<'src>(input: &'src [u8]) -> IResult<&'src [u8], Range<char>> {
    let (input, c1) = parse_char(input)?;
    let (input, _) = tag("-")(input)?;
    let (input, c2) = parse_char(input)?;
    Ok((input, c1..c2))
}

fn parse_char_or_range<'src>(input: &'src [u8]) -> IResult<&'src [u8], Characters> {
    alt((
        map(parse_char_range, |range| {
            Characters::Range(range.start, range.end)
        }),
        map(parse_char, Characters::Single),
    ))(input)
}

fn parse_char_set<'src>(input: &'src [u8]) -> IResult<&'src [u8], Pattern> {
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
    let (input, rep_kind) = opt(alt((
        map(tag("*"), |_| 0),
        map(tag("+"), |_| 1),
        map(tag("?"), |_| 2),
    )))(input)?;
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

fn parse_regex_pattern<'src>(input: &'src [u8]) -> IResult<&'src [u8], Pattern> {
    let (input, _) = tag("/")(input)?;
    let (input, seq) = parse_regex_sequence(input)?;
    let (input, _) = tag("/")(input)?;
    Ok((input, seq))
}

fn parse_literal_pattern<'src>(input: &'src [u8]) -> IResult<&'src [u8], Pattern> {
    let (input, _) = tag("\"")(input)?;
    let (input, chars) = take_while1(|c| {
        let ch = Into::<char>::into(c);
        ch != '"' && ch.is_ascii()
    })(input)?;
    let (input, _) = tag("\"")(input)?;
    let patterns: Vec<Pattern> = chars
        .iter()
        .map(|c| Into::<char>::into(*c))
        .map(|c| Pattern::Char {
            chars: Characters::Single(c),
        })
        .collect();
    Ok((input, Pattern::Sequence { elements: patterns }))
}

fn parse_pattern<'src>(input: &'src [u8]) -> IResult<&'src [u8], Pattern> {
    let (input, pattern) = alt((parse_literal_pattern, parse_regex_pattern))(input)?;
    Ok((input, pattern))
}

fn parse_token_rule<'src>(input: &'src [u8]) -> IResult<&'src [u8], TokenRule> {
    let (input, _) = tag("token")(input)?;
    let (input, _) = space1(input)?;
    let (input, name) = take_while1(|c: u8| Into::<char>::into(c).is_ascii_alphabetic())(input)?;
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

pub fn parse_lapex<'src>(input: &'src [u8]) -> IResult<&'src [u8], Vec<TokenRule<'src>>> {
    let (input, rules) = nom::multi::separated_list1(newline, parse_token_rule)(input)?;
    Ok((input, rules))
}
