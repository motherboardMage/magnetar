use nom::bytes::complete::{tag, take};
use nom::character::complete::{digit1, u64 as nom_u64};
use nom::combinator::{opt, peek, recognize};
use nom::multi::many0_count;
use nom::number::complete::u8 as nom_u8;
use nom::sequence::delimited;
use nom::{IResult, Parser};

pub fn skip_bencode_value(input: &[u8]) -> IResult<&[u8], ()> {
    let (input, first_byte) = peek(nom_u8).parse(input)?;

    let (remaining, _) = match first_byte {
        b'i' => skip_integer(input),
        b'0'..=b'9' => skip_string(input),
        b'l' => skip_list(input),
        b'd' => skip_dictionary(input),
        b'e' => {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )));
        }
        _ => {
            return Err(nom::Err::Failure(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )));
        }
    }?;

    Ok((remaining, ()))
}

fn skip_integer(input: &[u8]) -> IResult<&[u8], ()> {
    let (remaining, _) = (tag("i"), opt(tag("-")), digit1, tag("e")).parse(input)?;

    Ok((remaining, ()))
}

fn skip_string(input: &[u8]) -> IResult<&[u8], ()> {
    let (input, len) = nom_u64(input)?;
    let (input, _) = tag(":")(input)?;
    let (remaining, _) = recognize(take(len)).parse(input)?;

    Ok((remaining, ()))
}

fn skip_list(input: &[u8]) -> IResult<&[u8], ()> {
    let (remaining, _) =
        delimited(tag("l"), many0_count(skip_bencode_value), tag("e")).parse(input)?;

    Ok((remaining, ()))
}

fn skip_dictionary(input: &[u8]) -> IResult<&[u8], ()> {
    let (remaining, _) = delimited(
        tag("d"),
        many0_count((skip_bencode_value, skip_bencode_value)),
        tag("e"),
    )
    .parse(input)?;

    Ok((remaining, ()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- skip_bencode_value function tests ---

    #[test]
    fn test_empty_input() {
        assert!(skip_bencode_value(b"".as_slice()).is_err());
    }

    #[test]
    fn test_invalid_tag() {
        assert!(skip_bencode_value(b"x23124e".as_slice()).is_err());
    }

    // --- skip_integer function tests ---

    #[test]
    fn test_skip_positive_integer() {
        assert_eq!(
            skip_bencode_value(b"i23456e5:Hello".as_slice()),
            Ok((b"5:Hello".as_slice(), ()))
        );
    }

    #[test]
    fn test_skip_negative_integer() {
        assert_eq!(
            skip_bencode_value(b"i-23456e5:Hello".as_slice()),
            Ok((b"5:Hello".as_slice(), ()))
        );
    }

    // --- skip_list function tests ---

    #[test]
    fn test_skip_integer_list() {
        assert_eq!(
            skip_bencode_value(b"li432ei231155ei654345ee5:Hello".as_slice()),
            Ok((b"5:Hello".as_slice(), ()))
        );
    }

    #[test]
    fn test_skip_string_list() {
        assert_eq!(
            skip_bencode_value(b"l5:Hello3:Bye3:hiie".as_slice()),
            Ok((b"".as_slice(), ()))
        );
    }

    #[test]
    fn test_skip_invalid_members() {
        assert!(skip_bencode_value(b"lx32435ee".as_slice()).is_err());
    }

    #[test]
    fn test_skip_nested_list() {
        assert_eq!(
            skip_bencode_value(
                b"lli432ei231155ei654345eel5:Hello3:Bye3:hiil5:abcdei12345eeee5:Hello".as_slice()
            ),
            Ok((b"5:Hello".as_slice(), ()))
        );
    }

    #[test]
    fn test_invalid_tag_in_list() {
        assert!(skip_bencode_value(b"l5:Hello3:Byeli2324ex:5432ee".as_slice()).is_err());
    }

    // --- skip_dictionary function tests ---

    #[test]
    fn test_skip_dictionary_multi() {
        assert_eq!(
            skip_bencode_value(
                b"d5:Helloi23e3:Byelli432ei231155ei654345eel5:Hello3:Bye3:hiil5:abcdei12345eeeee"
                    .as_slice()
            ),
            Ok((b"".as_slice(), ()))
        );
    }

    #[test]
    fn test_odd_number_of_entries() {
        assert!(skip_bencode_value(b"d3:Abci12e5:Helloe".as_slice()).is_err());
    }

    #[test]
    fn test_non_string_key() {
        assert!(skip_bencode_value(b"di244e3:Bye5:Helloi534ee".as_slice()).is_err());
    }
}
