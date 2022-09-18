use nom::IResult;

use crate::{parse_char_escaped, parse_char_unescaped};

#[test]
fn test_parse_char_unescaped() {
    for ch_code in 0_u8..=255 {
        if let Some(ch) = std::char::from_u32(ch_code.into()) {
            let input = ch.to_string();
            let ires: IResult<&[u8], char> = if ch.is_ascii()
                && ch != ')'
                && ch != ']'
                && ch != '*'
                && ch != '+'
                && ch != '?'
                && ch != '/'
                && ch != '\\'
                && ch != '|'
                && ch != '\n'
                && ch != '\t'
                && ch != '\r'
            {
                Ok((b"", ch))
            } else {
                Err(nom::Err::Error(nom::error::Error::new(
                    input.as_bytes(),
                    nom::error::ErrorKind::TakeWhileMN,
                )))
            };
            assert_eq!(ires, parse_char_unescaped(input.as_bytes()))
        }
    }
}

#[test]
fn test_parse_char_escaped_unicode() {
    for ch_code in 0..=std::char::MAX.into() {
        if let Some(ch) = std::char::from_u32(ch_code) {
            let ires: IResult<&[u8], char> = Ok((b"", ch));
            let input = format!("\\u{{{:04X}}}", ch_code);
            assert_eq!(ires, parse_char_escaped(input.as_bytes()));
        }
    }
}

#[test]
fn test_parse_char_escaped() {
    let ires: IResult<&[u8], char> = Ok((b"", '\t'));
    assert_eq!(ires, parse_char_escaped(b"\\t"));
    let ires: IResult<&[u8], char> = Ok((b"", '\r'));
    assert_eq!(ires, parse_char_escaped(b"\\r"));
    let ires: IResult<&[u8], char> = Ok((b"", '\n'));
    assert_eq!(ires, parse_char_escaped(b"\\n"));
}
