#[derive(Debug, Clone, PartialEq)]
pub enum GitFileSection {
    Staged,
    Unstaged,
    Untracked,
}

#[derive(Debug, Clone)]
pub struct GitFileEntry {
    pub path: String,
    pub section: GitFileSection,
    pub status_char: char,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiffLineKind {
    Header,
    Add,
    Remove,
    Hunk,
    Context,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct GitStatus {
    pub staged: Vec<GitFileEntry>,
    pub unstaged: Vec<GitFileEntry>,
    pub untracked: Vec<GitFileEntry>,
}

#[derive(Debug, Clone)]
pub enum FlatGitItem {
    SectionHeader(String, GitFileSection),
    File(GitFileEntry),
}

impl GitStatus {
    pub fn is_empty(&self) -> bool {
        self.staged.is_empty() && self.unstaged.is_empty() && self.untracked.is_empty()
    }

    pub fn total_files(&self) -> usize {
        self.staged.len() + self.unstaged.len() + self.untracked.len()
    }

    pub fn flat_list(&self) -> Vec<FlatGitItem> {
        let mut items = Vec::new();

        if !self.staged.is_empty() {
            items.push(FlatGitItem::SectionHeader(
                format!("Staged ({})", self.staged.len()),
                GitFileSection::Staged,
            ));
            for entry in &self.staged {
                items.push(FlatGitItem::File(entry.clone()));
            }
        }

        if !self.unstaged.is_empty() {
            items.push(FlatGitItem::SectionHeader(
                format!("Changes ({})", self.unstaged.len()),
                GitFileSection::Unstaged,
            ));
            for entry in &self.unstaged {
                items.push(FlatGitItem::File(entry.clone()));
            }
        }

        if !self.untracked.is_empty() {
            items.push(FlatGitItem::SectionHeader(
                format!("Untracked ({})", self.untracked.len()),
                GitFileSection::Untracked,
            ));
            for entry in &self.untracked {
                items.push(FlatGitItem::File(entry.clone()));
            }
        }

        items
    }
}

impl FlatGitItem {
    pub fn is_file(&self) -> bool {
        matches!(self, FlatGitItem::File(_))
    }
}
