use std::path::Path;

use anyhow::Result;

use crate::model::inbox::InboxMessage;

/// Load inbox messages for a specific agent in a team.
pub fn load_inbox(
    claude_home: &Path,
    team_name: &str,
    agent_name: &str,
) -> Result<Vec<InboxMessage>> {
    let inbox_path = claude_home
        .join("teams")
        .join(team_name)
        .join("inboxes")
        .join(format!("{}.json", agent_name));

    if !inbox_path.exists() {
        return Ok(vec![]);
    }

    let data = std::fs::read_to_string(&inbox_path)?;
    let raw_values: Vec<serde_json::Value> = serde_json::from_str(&data)?;
    let mut messages: Vec<InboxMessage> = raw_values
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();

    // Sort by timestamp descending (most recent first)
    messages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(messages)
}
