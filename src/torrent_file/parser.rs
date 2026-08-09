use nom::bytes::complete::{tag, take};
use nom::character::complete::{i64 as nom_i64, u64 as nom_u64};
use nom::multi::many0;
use nom::sequence::delimited;
use nom::{IResult, Parser};

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
}
