use std::path::PathBuf;

use crate::model::plan::MarkdownLine;

#[derive(Debug, Clone, PartialEq)]
pub enum EntryKind {
    Directory,
    File,
}

#[derive(Debug, Clone)]
pub struct FileBrowserEntry {
    pub name: String,
    pub path: PathBuf,
    pub kind: EntryKind,
    pub size: u64,
    pub depth: usize,
}

#[derive(Debug, Clone)]
pub enum FileContent {
    Text(Vec<String>),
    Markdown(Vec<MarkdownLine>),
    Binary,
    TooLarge,
}
