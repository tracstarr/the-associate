use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct JiraIssue {
    pub key: String,
    pub summary: String,
    #[serde(rename = "statusName", alias = "status_name", default)]
    pub status_name: String,
    #[serde(rename = "statusCategory", alias = "status_category", default)]
    pub status_category: String,
    #[serde(rename = "issueType", alias = "issue_type", default)]
    pub issue_type: String,
    #[serde(default)]
    pub priority: String,
    #[serde(default)]
    pub labels: Vec<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone)]
pub enum FlatJiraItem {
    StatusHeader(String, String), // (status_name, status_category)
    Issue(JiraIssue),
}

#[derive(Debug, Clone)]
pub struct JiraTransition {
    pub name: String,
}

impl JiraIssue {
    /// Icon based on issue type.
    pub fn type_icon(&self) -> &'static str {
        match self.issue_type.to_lowercase().as_str() {
            "bug" => "B",
            "story" => "S",
            "task" => "T",
            "epic" => "E",
            "sub-task" | "subtask" => "s",
            _ => "?",
        }
    }
}
