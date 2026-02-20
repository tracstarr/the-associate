use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq)]
pub enum MarkdownLineKind {
    Heading,
    CodeFence,
    CodeBlock,
    Normal,
}

#[derive(Debug, Clone)]
pub struct MarkdownLine {
    pub kind: MarkdownLineKind,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct PlanFile {
    pub filename: String,
    pub title: String,
    pub modified: SystemTime,
    pub lines: Vec<MarkdownLine>,
}

impl PlanFile {
    pub fn display_name(&self) -> &str {
        self.filename.strip_suffix(".md").unwrap_or(&self.filename)
    }
}
