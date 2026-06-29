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
