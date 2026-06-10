use nom::IResult;
use nom::bytes::complete::{tag, take};

pub fn parse_int(input: &[u8]) -> IResult<&[u8], &[u8]> {}
