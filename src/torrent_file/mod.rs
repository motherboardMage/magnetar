use std::collections::BTreeMap;

pub mod parser;

#[derive(Debug, Clone)]
pub struct Torrent<'a> {
    pub announce: String,
    pub info: Info<'a>,
    pub piece_layers: Option<BTreeMap<&'a [u8], &'a [u8]>>,

    /// Optional fields
    pub announce_list: Option<Vec<Vec<String>>>,
    pub comment: Option<String>,
    pub created_by: Option<String>,
    pub creation_date: Option<i64>,

    // Raw info bytes for hashing
    pub raw_info: &'a [u8],
}

#[derive(Debug, Clone)]
pub struct Info<'a> {
    pub name: String,
    pub piece_length: i64,
    pub meta_version: Option<i64>,
    pub file_layout: FileLayout<'a>,
}

#[derive(Debug, Clone)]
pub enum FileLayout<'a> {
    V1 {
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

#[derive(Debug, Clone)]
pub struct FileV1<'a> {
    pub length: i64,
    pub path: Option<Vec<&'a [u8]>>,
}

#[derive(Debug, Clone)]
pub enum FileTreeNode<'a> {
    Directory(BTreeMap<&'a [u8], FileTreeNode<'a>>),
    File { length: i64, pieces_root: &'a [u8] },
}
