use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinearIssue {
    pub identifier: String,
    pub title: String,
    pub description: Option<String>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub priority_label: String,
    #[serde(default)]
    pub state: LinearState,
    pub assignee: Option<LinearUser>,
    #[serde(default)]
    pub labels: LinearLabels,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub team: Option<LinearTeam>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinearState {
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "type")]
    pub state_type: String,
    #[serde(default)]
    pub color: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinearUser {
    pub name: String,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LinearLabels {
    #[serde(default)]
    pub nodes: Vec<LinearLabel>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LinearLabel {
    pub name: String,
    #[serde(default)]
    pub color: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LinearTeam {
    pub name: String,
    pub key: String,
}

#[derive(Debug, Clone)]
pub enum FlatLinearItem {
    AssignmentHeader(String), // "My Tasks", "Unassigned"
    Issue(Box<LinearIssue>),
}

impl LinearIssue {
    /// Icon based on priority level.
    pub fn priority_icon(&self) -> &'static str {
        match self.priority {
            1 => "!!!",
            2 => "!! ",
            3 => "!  ",
            4 => ".  ",
            _ => "   ",
        }
    }
}
