use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub author: PrAuthor,
    pub url: String,
    pub created_at: String,
    pub updated_at: String,
    pub head_ref_name: String,
    pub base_ref_name: String,
    pub is_draft: bool,
    #[serde(default)]
    pub additions: u64,
    #[serde(default)]
    pub deletions: u64,
    pub review_decision: Option<String>,
    #[serde(default)]
    pub assignees: Vec<PrAssignee>,
    #[serde(default)]
    pub labels: Vec<PrLabel>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PrAuthor {
    pub login: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PrAssignee {
    pub login: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PrLabel {
    pub name: String,
}

#[derive(Debug, Clone)]
pub enum FlatPrItem {
    SectionHeader(String),
    Pr(Box<PullRequest>),
}

impl PullRequest {
    /// Size label based on total changes (additions + deletions).
    pub fn size_label(&self) -> &'static str {
        let total = self.additions + self.deletions;
        match total {
            0..=9 => "XS",
            10..=49 => "S",
            50..=249 => "M",
            250..=999 => "L",
            _ => "XL",
        }
    }

    /// Review status icon.
    pub fn review_icon(&self) -> &'static str {
        match self.review_decision.as_deref() {
            Some("APPROVED") => "[+]",
            Some("CHANGES_REQUESTED") => "[!]",
            Some("REVIEW_REQUIRED") => "[?]",
            _ => "[ ]",
        }
    }
}

// ---------------------------------------------------------------------------
// GitHub Issues
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubIssue {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub url: String,
    pub created_at: String,
    pub updated_at: String,
    pub author: IssueAuthor,
    #[serde(default)]
    pub labels: Vec<IssueLabel>,
    #[serde(default)]
    pub assignees: Vec<IssueAssignee>,
    pub body: Option<String>,
    #[serde(default)]
    pub comments: Vec<IssueComment>,
    #[serde(default)]
    pub milestone: Option<IssueMilestone>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IssueAuthor {
    pub login: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IssueLabel {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IssueAssignee {
    pub login: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueComment {
    pub author: IssueAuthor,
    pub body: String,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IssueMilestone {
    pub title: String,
}

#[derive(Debug, Clone)]
pub enum FlatIssueItem {
    SectionHeader(String),
    Issue(Box<GitHubIssue>),
}

impl GitHubIssue {
    pub fn state_icon(&self) -> &'static str {
        match self.state.as_str() {
            "OPEN" => "[O]",
            "CLOSED" => "[X]",
            _ => "[ ]",
        }
    }
}
