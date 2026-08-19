use std::collections::BTreeMap;

use nom::bytes::complete::{tag, take};
use nom::character::complete::{i64 as nom_i64, u64 as nom_u64};
use nom::combinator::peek;
use nom::error::ErrorKind;
use nom::multi::{many0, many1};
use nom::sequence::delimited;
use nom::{IResult, Parser};

use crate::torrent_file::skip_values::skip_bencode_value;
use crate::torrent_file::{FileTreeNode, FileV1};

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

fn parse_file_tree_node<'a>(input: &'a [u8]) -> IResult<&'a [u8], FileTreeNode<'a>> {
    let mut input = input;

    let mut length = None;
    let mut pieces_root = None;

    let name;
    (_, (_, name)) = peek((tag("d"), parse_bencode_string)).parse(input)?;

    if name == b"" {
        (input, _) = (tag("d"), parse_bencode_string, tag("d")).parse(input)?;

        let mut property;

        loop {
            let next;
            (input, next) = peek(take(1usize)).parse(input)?;

            if next == b"e" {
                break;
            }

            (input, property) = parse_bencode_string(input)?;
            match property {
                b"length" => {
                    (input, length) = parse_bencode_int(input)
                        .map(|x| (x.0, if x.1 < 0 { None } else { Some(x.1 as usize) }))?
                }

                b"pieces root" => {
                    (input, pieces_root) = parse_bencode_string(input).map(|x| (x.0, Some(x.1)))?
                }

                _ => (input, _) = skip_bencode_value(input)?,
            }
        }

        (input, _) = (tag("e"), tag("e")).parse(input)?;

        let length = length.ok_or(fail(input, ErrorKind::Fail))?;

        if let Some(pr) = pieces_root {
            if pr.len() != 32 {
                return Err(fail(input, ErrorKind::Fail));
            }
        }

        Ok((
            input,
            FileTreeNode::File {
                length,
                pieces_root,
            },
        ))
    } else {
        let file_tree;
        (input, file_tree) = parse_file_tree(input)?;
        Ok((input, FileTreeNode::Directory(file_tree)))
    }
}

fn parse_file_tree<'a>(input: &'a [u8]) -> IResult<&'a [u8], BTreeMap<&'a [u8], FileTreeNode<'a>>> {
    let (mut input, _) = tag("d").parse(input)?;
    let mut map: BTreeMap<&'a [u8], FileTreeNode<'a>> = BTreeMap::new();

    loop {
        let next;
        (input, next) = peek(take(1usize)).parse(input)?;

        if next == b"e" {
            break;
        }

        let name;
        (input, name) = parse_bencode_string(input)?;

        let node;
        (input, node) = parse_file_tree_node(input)?;

        map.insert(name, node);
    }

    (input, _) = tag("e").parse(input)?;

    Ok((input, map))
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

    // --- File tree parser tests ---

    #[test]
    fn test_file_tree_single_file() {
        let mut expected_tree = BTreeMap::new();
        expected_tree.insert(
            b"frac.rs".as_slice(),
            FileTreeNode::File {
                length: 556,
                pieces_root: Some(b"aaaaaaaaaabbbbbbbbbbccccccccccdd"),
            },
        );

        assert_eq!(
            parse_file_tree(
                b"d7:frac.rsd0:d6:lengthi556e11:pieces root32:aaaaaaaaaabbbbbbbbbbccccccccccddeee"
                    .as_slice()
            ),
            Ok((b"".as_slice(), expected_tree))
        );
    }

    #[test]
    fn test_parse_file_tree_nested() {
        let mut file_a = BTreeMap::new();
        let mut dir2 = BTreeMap::new();
        let mut dir1 = BTreeMap::new();

        file_a.insert(
            b"fileA.txt".as_slice(),
            FileTreeNode::File {
                length: 1024,
                pieces_root: Some(b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            },
        );

        dir2.insert(b"dir2".as_slice(), FileTreeNode::Directory(file_a));
        dir1.insert(b"dir1".as_slice(), FileTreeNode::Directory(dir2));

        assert_eq!(parse_file_tree(b"d4:dir1d4:dir2d9:fileA.txtd0:d6:lengthi1024e11:pieces root32:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaeeeee".as_slice()), Ok((b"".as_slice(), dir1)));
    }

    #[test]
    fn test_parse_file_tree_multi_nested() {
        let hash_a: &[u8] = &[b'a'; 32];
        let hash_b: &[u8] = &[b'b'; 32];
        let hash_c: &[u8] = &[b'c'; 32];
        let hash_f: &[u8] = &[b'f'; 32];

        let readme = FileTreeNode::File {
            length: 128,
            pieces_root: Some(hash_a),
        };
        let guide = FileTreeNode::File {
            length: 1024,
            pieces_root: Some(hash_b),
        };
        let main_rs = FileTreeNode::File {
            length: 512,
            pieces_root: Some(hash_c),
        };
        let utils_rs = FileTreeNode::File {
            length: 256,
            pieces_root: Some(hash_f),
        };

        let mut docs_map = BTreeMap::new();
        docs_map.insert(b"guide.txt".as_slice(), guide);

        let mut src_map = BTreeMap::new();
        src_map.insert(b"main.rs".as_slice(), main_rs);
        src_map.insert(b"utils.rs".as_slice(), utils_rs);

        let mut expected = BTreeMap::new();
        expected.insert(b"README.md".as_slice(), readme);
        expected.insert(b"docs".as_slice(), FileTreeNode::Directory(docs_map));
        expected.insert(b"src".as_slice(), FileTreeNode::Directory(src_map));

        let input = b"d9:README.mdd0:d6:lengthi128e11:pieces root32:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaee4:docsd9:guide.txtd0:d6:lengthi1024e11:pieces root32:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbeee3:srcd7:main.rsd0:d6:lengthi512e11:pieces root32:ccccccccccccccccccccccccccccccccee8:utils.rsd0:d6:lengthi256e11:pieces root32:ffffffffffffffffffffffffffffffffeeee";

        assert_eq!(
            parse_file_tree(input.as_slice()),
            Ok((b"".as_slice(), expected))
        );
    }
}
