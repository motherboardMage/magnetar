use nom::bytes::complete::{tag, take};
use nom::character::complete::{i64 as nom_i64, u64 as nom_u64};
use nom::combinator::peek;
use nom::error::ErrorKind;
use nom::multi::{many0, many1};
use nom::sequence::delimited;
use nom::{IResult, Parser};

use crate::torrent_file::FileV1;
use crate::torrent_file::skip_values::skip_bencode_value;

fn fail(input: &[u8], code: ErrorKind) -> nom::Err<nom::error::Error<&[u8]>> {
    nom::Err::Failure(nom::error::Error::new(input, code))
}

fn parse_bencode_int(input: &[u8]) -> IResult<&[u8], i64> {
    let mut p = delimited(tag("i"), nom_i64, tag("e"));
    let (input, num) = p.parse(input)?;

    Ok((input, num))
}

fn parse_bencode_string(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (input, len) = nom_u64(input)?;
    let (input, _) = tag(":").parse(input)?;
    let (input, parsed) = take(len).parse(input)?;

    Ok((input, parsed))
}

fn parse_string_list(input: &[u8]) -> IResult<&[u8], Vec<&[u8]>> {
    let (input, list) = delimited(tag("l"), many0(parse_bencode_string), tag("e")).parse(input)?;

    Ok((input, list))
}

fn parse_file_entry<'a>(input: &'a [u8]) -> IResult<&'a [u8], FileV1<'a>> {
    let (input, _) = tag("d").parse(input)?;

    let mut input = input;
    let mut len = None;
    let mut path = None;

    loop {
        let next;
        (input, next) = peek(take(1usize)).parse(input)?;

        if next == b"e" {
            break;
        }

        let key;

        (input, key) = parse_bencode_string(input)?;
        match key {
            b"length" => {
                (input, len) = parse_bencode_int(input)
                    .map(|x| (x.0, if x.1 < 0 { None } else { Some(x.1 as usize) }))?
            }
            b"path" => (input, path) = parse_string_list(input).map(|x| (x.0, Some(x.1)))?,
            _ => (input, _) = skip_bencode_value(input)?,
        }
    }
    (input, _) = tag("e").parse(input)?;

    let length = len.ok_or(fail(input, ErrorKind::Fail))?;
    let path = path.ok_or(fail(input, ErrorKind::Fail))?;

    Ok((input, FileV1 { length, path }))
}

fn parse_files_list<'a>(input: &'a [u8]) -> IResult<&'a [u8], Vec<FileV1<'a>>> {
    let (input, list) = delimited(tag("l"), many1(parse_file_entry), tag("e")).parse(input)?;

    Ok((input, list))
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

    // --- String List Parser Tests ---

    #[test]
    fn test_parse_string_list_standard() {
        let expected: Vec<&[u8]> = vec![b"Hello", b"Bye", b"Hi"];
        assert_eq!(
            parse_string_list(b"l5:Hello3:Bye2:Hie"),
            Ok((b"".as_slice(), expected))
        );
    }

    #[test]
    fn test_string_list_remaining_input() {
        let expected: Vec<&[u8]> = vec![b"Hello", b"Bye", b"Hi"];
        assert_eq!(
            parse_string_list(b"l5:Hello3:Bye2:Hie7:remains"),
            Ok((b"7:remains".as_slice(), expected))
        );
    }

    #[test]
    fn test_string_list_empty_list() {
        assert_eq!(parse_string_list(b"le"), Ok((b"".as_slice(), vec![])));
    }

    #[test]
    fn test_string_list_non_string_elements() {
        assert!(parse_string_list(b"li123e5:Helloe").is_err());
    }

    // --- File entry parser tests ---

    #[test]
    fn test_file_entry_standard() {
        let expected_path: Vec<&[u8]> = vec![b"docs", b"notes", b"ab.txt"];
        assert_eq!(
            parse_file_entry(b"d6:lengthi345e4:pathl4:docs5:notes6:ab.txtee"),
            Ok((
                b"".as_slice(),
                FileV1 {
                    length: 345usize,
                    path: expected_path
                }
            ))
        );
    }

    #[test]
    fn test_file_entry_unordered() {
        let expected_path: Vec<&[u8]> = vec![b"docs", b"notes", b"ab.txt"];
        assert_eq!(
            parse_file_entry(b"d4:pathl4:docs5:notes6:ab.txte6:lengthi345ee"),
            Ok((
                b"".as_slice(),
                FileV1 {
                    length: 345usize,
                    path: expected_path
                }
            ))
        );
    }
}
