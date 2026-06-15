use std::str::from_utf8;

use nom::bytes::complete::{tag, take};
use nom::character::complete::digit1;
use nom::sequence::delimited;
use nom::{IResult, Parser};

fn parse_bencode_int(input: &[u8]) -> IResult<&[u8], i64> {
    // capture start of erroneous integer, if it is one
    let original_input = input;

    let mut p = delimited(tag("i"), digit1, tag("e"));
    let (input, int_bytes) = p.parse(input)?;

    let int_str = match from_utf8(int_bytes) {
        Ok(s) => s,
        Err(_) => {
            return Err(nom::Err::Failure(nom::error::Error::new(
                original_input,
                nom::error::ErrorKind::Fail,
            )));
        }
    };

    match int_str.parse::<i64>() {
        Ok(num) => Ok((input, num)),
        Err(_) => Err(nom::Err::Error(nom::error::Error::new(
            original_input,
            nom::error::ErrorKind::Digit,
        ))),
    }
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

    #[test]
    fn test_bencode_int() {
        assert_eq!(
            parse_bencode_int(b"i234451e"),
            Ok((b"".as_slice(), 234451i64))
        );
        assert_eq!(parse_bencode_int(b"i0e"), Ok((b"".as_slice(), 0)));
        assert!(parse_bencode_int(b"i9999999999999999999999999e").is_err());
    }

    #[test]
    fn test_bencode_string() {
        assert_eq!(
            parse_bencode_string(b"8:abcdefgh"),
            Ok((b"".as_slice(), b"abcdefgh".as_slice()))
        );
        assert_eq!(
            parse_bencode_string(b"0:"),
            Ok((b"".as_slice(), b"".as_slice()))
        );
        assert!(parse_bencode_string(b"7:abcde".as_slice()).is_err());
    }
}
