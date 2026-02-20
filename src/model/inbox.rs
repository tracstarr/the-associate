use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxMessage {
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub timestamp: Option<DateTime<Utc>>,
    #[serde(default)]
    pub read: Option<bool>,
    #[serde(default)]
    pub color: Option<String>,
}

impl InboxMessage {
    /// Return a displayable summary of the message text.
    /// If text is JSON-encoded (task_assignment, idle_notification, etc.),
    /// extract a human-readable summary. Otherwise return as-is.
    pub fn display_text(&self) -> String {
        // Try parsing text as JSON for structured messages
        if self.text.starts_with('{') {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&self.text) {
                return format_structured_message(&val);
            }
        }
        self.text.clone()
    }

    /// Return the structured message type if text is JSON, or None.
    pub fn message_type(&self) -> Option<String> {
        if self.text.starts_with('{') {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&self.text) {
                return val
                    .get("type")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string());
            }
        }
        None
    }

    /// Format timestamp for display, or empty string.
    pub fn display_time(&self) -> String {
        match self.timestamp {
            Some(ts) => ts.format("%m/%d %H:%M").to_string(),
            None => String::new(),
        }
    }
}

/// Format a structured JSON message into readable text.
fn format_structured_message(val: &serde_json::Value) -> String {
    let msg_type = val.get("type").and_then(|t| t.as_str()).unwrap_or("");
    match msg_type {
        "task_assignment" => {
            let subject = val
                .get("subject")
                .and_then(|s| s.as_str())
                .unwrap_or("(no subject)");
            let task_id = val.get("taskId").and_then(|s| s.as_str()).unwrap_or("?");
            format!("[Task #{}] {}", task_id, subject)
        }
        "idle_notification" => {
            let agent = val
                .get("from")
                .and_then(|s| s.as_str())
                .or_else(|| val.get("agentName").and_then(|s| s.as_str()))
                .unwrap_or("agent");
            format!("{} is idle", agent)
        }
        "shutdown_request" => "Shutdown requested".to_string(),
        "shutdown_approved" => "Shutdown approved".to_string(),
        "plan_approval_request" => {
            let from = val.get("from").and_then(|s| s.as_str()).unwrap_or("agent");
            format!("Plan approval requested by {}", from)
        }
        "plan_approval_response" => {
            let approved = val
                .get("approve")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let content = val.get("content").and_then(|s| s.as_str()).unwrap_or("");
            if approved {
                "Plan approved".to_string()
            } else if content.is_empty() {
                "Plan rejected".to_string()
            } else {
                format!("Plan rejected: {}", content)
            }
        }
        "task_completed" => {
            let task_id = val.get("taskId").and_then(|s| s.as_str()).unwrap_or("?");
            format!("Task #{} completed", task_id)
        }
        "message" => val
            .get("content")
            .and_then(|s| s.as_str())
            .unwrap_or(&val.to_string())
            .to_string(),
        _ => {
            // Unknown type - show type + any content/subject
            let content = val
                .get("content")
                .or_else(|| val.get("subject"))
                .and_then(|s| s.as_str());
            match content {
                Some(c) => format!("[{}] {}", msg_type, c),
                None => val.to_string(),
            }
        }
    }
}
