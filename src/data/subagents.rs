use std::path::{Path, PathBuf};

/// Info about a subagent transcript file.
#[derive(Debug, Clone)]
pub struct SubagentInfo {
    pub agent_id: String,
    pub path: PathBuf,
}

/// Scan a session directory for subagent transcripts.
/// Looks for `<project_dir>/<session_id>/subagents/agent-*.jsonl`.
pub fn find_subagents(project_dir: &Path, session_id: &str) -> Vec<SubagentInfo> {
    let subagents_dir = project_dir.join(session_id).join("subagents");
    if !subagents_dir.exists() {
        return Vec::new();
    }

    let mut results = Vec::new();

    let entries = match std::fs::read_dir(&subagents_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }

        let filename = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        // Extract agent_id from filenames like "agent-a5c425e"
        let agent_id = if filename.starts_with("agent-") {
            filename[6..].to_string()
        } else {
            filename.clone()
        };

        if !agent_id.is_empty() {
            results.push(SubagentInfo {
                agent_id,
                path: path.clone(),
            });
        }
    }

    results.sort_by(|a, b| a.agent_id.cmp(&b.agent_id));
    results
}
