use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::space1,
    IResult, multi::many1,
};

#[derive(Debug)]
pub enum Characters {
    Single(char),
    Range(char, char),
}

#[derive(Debug)]
pub enum Pattern {
    Sequence { elements: Vec<Pattern> },
    Alternative { elements: Vec<Pattern> },
    OneOrMany { inner: Box<Pattern> },
    ZeroOrMany { inner: Box<Pattern> },
    CharSet { chars: Vec<Characters> },
    Char { chars: Characters },
}

#[derive(Debug)]
pub struct TokenRule<'src> {
    name: &'src str,
    pattern: Pattern,
}

fn parse_regex_pattern<'src>(input: &'src [u8]) -> IResult<&'src [u8], Pattern> {
    let (input, _) = tag("/")(input)?;
    let (input, chars) = take_while1(|c| c != '/' as u8)(input)?;
    let (input, _) = tag("/")(input)?;
    Ok((input, Pattern::Sequence { elements: Vec::new() }))
}

fn parse_literal_pattern<'src>(input: &'src [u8]) -> IResult<&'src [u8], Pattern> {
    let (input, _) = tag("\"")(input)?;
    let (input, chars) = take_while1(|c| c != '\"' as u8)(input)?;
    let (input, _) = tag("\"")(input)?;
    Ok((input, Pattern::Sequence { elements: Vec::new() }))
}

fn parse_pattern<'src>(input: &'src [u8]) -> IResult<&'src [u8], Pattern> {
    let (input, pattern) = alt((
        parse_literal_pattern,
        parse_regex_pattern
    ))(input)?;
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
    let (input, rules) = many1(parse_token_rule)(input)?;
    Ok((input, rules))
}
