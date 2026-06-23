use std::str::from_utf8;

use nom::bytes::complete::{tag, take};
use nom::character::complete::{digit1, i64 as nom_i64};
use nom::sequence::delimited;
use nom::{IResult, Parser};

fn skip_bencode_value(input: &[u8]) -> IResult<&[u8], ()> {}

fn parse_bencode_int(input: &[u8]) -> IResult<&[u8], i64> {
    // capture start of erroneous integer, if it is one
    let original_input = input;

    let mut p = delimited(tag("i"), nom_i64, tag("e"));
    let (input, num) = p.parse(input)?;

    Ok((input, num))
}

fn parse_bencode_string(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (input, len_bytes) = digit1(input)?;
    let (input, _) = tag(":")(input)?;

    let len_str = match from_utf8(len_bytes) {
        Ok(s) => s,
        Err(_) => {
            return Err(nom::Err::Failure(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Fail,
            )));
        }
    };

    let len = match len_str.parse::<usize>() {
        Ok(n) => n,
        Err(_) => {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Digit,
            )));
        }
    };

    let (input, parsed) = take(len)(input)?;

    Ok((input, parsed))
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Integer Parser Tests ---

    #[test]
    fn test_parse_int_standard() {
        assert_eq!(
            parse_bencode_int(b"i234451e"),
            Ok((b"".as_slice(), 234451i64))
        );
    }

    #[test]
    fn test_parse_int_zero() {
        assert_eq!(parse_bencode_int(b"i0e"), Ok((b"".as_slice(), 0)));
    }

    #[test]
    fn test_parse_int_overflow_protection() {
        assert!(parse_bencode_int(b"i9999999999999999999999999e").is_err());
    }

    // --- String Parser Tests ---

    #[test]
    fn test_parse_string_standard() {
        assert_eq!(
            parse_bencode_string(b"8:abcdefgh"),
            Ok((b"".as_slice(), b"abcdefgh".as_slice()))
        );
    }

    #[test]
    fn test_parse_string_empty_boundary() {
        assert_eq!(
            parse_bencode_string(b"0:"),
            Ok((b"".as_slice(), b"".as_slice()))
        );
    }

    #[test]
    fn test_parse_string_truncation_handling() {
        assert!(parse_bencode_string(b"7:abcde".as_slice()).is_err());
    }
}
