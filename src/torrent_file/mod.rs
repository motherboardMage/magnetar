use std::collections::BTreeMap;

pub mod integer;

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
}

#[derive(Debug, Clone)]
pub struct Info<'a> {
    pub name: &'a [u8],
    pub piece_length: i64,
    pub meta_version: Option<i64>,
    pub file_tree: FileTreeNode<'a>,
}

#[derive(Debug, Clone)]
pub enum FileTreeNode<'a> {
    Directory(BTreeMap<&'a [u8], FileTreeNode<'a>>),
    File { length: i64, pieces_root: &'a [u8] },
}
