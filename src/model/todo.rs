use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct TodoItem {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default, rename = "activeForm")]
    pub active_form: Option<String>,
}

impl TodoItem {
    pub fn display_text(&self) -> &str {
        self.content.as_deref().unwrap_or("(empty)")
    }

    pub fn status_icon(&self) -> &str {
        match self.status.as_deref() {
            Some("completed") => "[X]",
            Some("in_progress") => "[=]",
            _ => "[ ]",
        }
    }
}

/// A todo file with its items.
#[derive(Debug, Clone)]
pub struct TodoFile {
    pub filename: String,
    pub items: Vec<TodoItem>,
}

impl TodoFile {
    pub fn display_name(&self) -> String {
        // Truncate the UUID-heavy filenames
        let name = &self.filename;
        if name.chars().count() > 30 {
            let truncated: String = name.chars().take(27).collect();
            format!("{}...", truncated)
        } else {
            name.to_string()
        }
    }
}
