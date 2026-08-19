use std::collections::BTreeMap;

pub mod parser;
pub mod skip_values;

#[derive(Debug, Clone)]
pub struct Torrent<'a> {
    pub announce: &'a [u8],
    pub info: Info<'a>,
    pub piece_layers: Option<BTreeMap<&'a [u8], &'a [u8]>>,

    /// Optional fields
    pub announce_list: Option<Vec<Vec<&'a [u8]>>>,
    pub comment: Option<&'a [u8]>,
    pub created_by: Option<&'a [u8]>,
    pub creation_date: Option<i64>,

    // Raw info bytes for hashing
    pub raw_info: &'a [u8],
}

#[derive(Debug, Clone)]
pub struct Info<'a> {
    pub name: &'a [u8],
    pub piece_length: usize,
    pub meta_version: Option<i64>,
    pub file_layout: FileLayout<'a>,
}

#[derive(Debug, Clone)]
pub enum FileLayout<'a> {
    V1SingleFile {
        pieces: &'a [u8],
        length: usize,
    },
    V1MultiFile {
        pieces: &'a [u8],
        files: Vec<FileV1<'a>>,
    },
    V2 {
        file_tree: BTreeMap<&'a [u8], FileTreeNode<'a>>,
    },
    Hybrid {
        pieces: &'a [u8],
        files: Vec<FileV1<'a>>,
        file_tree: BTreeMap<&'a [u8], FileTreeNode<'a>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileV1<'a> {
    pub length: usize,
    pub path: Vec<&'a [u8]>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileTreeNode<'a> {
    Directory(BTreeMap<&'a [u8], FileTreeNode<'a>>),
    File {
        length: usize,
        pieces_root: Option<&'a [u8]>,
    },
}
