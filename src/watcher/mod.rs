use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::Result;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};

use crate::config::{TabsConfig, DEBOUNCE_MS};
use crate::event::{AppEvent, FileChange};

/// Start the file watcher, sending FileChanged events to the given sender.
/// Directories for disabled tabs are not watched.
pub fn start_watcher(
    claude_home: PathBuf,
    encoded_project: String,
    project_cwd: PathBuf,
    tx: mpsc::Sender<AppEvent>,
    tabs_config: &TabsConfig,
) -> Result<notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>> {
    let sessions_enabled = tabs_config.sessions();
    let teams_enabled = tabs_config.teams();
    let todos_enabled = tabs_config.todos();
    let git_enabled = tabs_config.git();
    let plans_enabled = tabs_config.plans();

    let project_dir = claude_home.join("projects").join(&encoded_project);
    let teams_dir = claude_home.join("teams");
    let tasks_dir = claude_home.join("tasks");
    let todos_dir = claude_home.join("todos");

    let tx_clone = tx.clone();
    let encoded_clone = encoded_project.clone();

    let mut debouncer = new_debouncer(
        Duration::from_millis(DEBOUNCE_MS),
        move |res: Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>| {
            let events = match res {
                Ok(events) => events,
                Err(_) => return,
            };

            for event in events {
                if event.kind != DebouncedEventKind::Any {
                    continue;
                }

                let path = &event.path;
                let path_str = path.to_string_lossy().to_string();

                // Determine what kind of file changed
                let change = classify_change(&path_str, &encoded_clone, path);
                if let Some(change) = change {
                    let _ = tx_clone.send(AppEvent::FileChanged(change));
                }
            }
        },
    )?;

    let watcher = debouncer.watcher();

    // Watch project directory (recursive to catch subagent transcripts)
    if sessions_enabled && project_dir.exists() {
        let _ = watcher.watch(&project_dir, notify::RecursiveMode::Recursive);
    }

    // Watch teams directory
    if teams_enabled && teams_dir.exists() {
        let _ = watcher.watch(&teams_dir, notify::RecursiveMode::Recursive);
    }

    // Watch tasks directory
    if teams_enabled && tasks_dir.exists() {
        let _ = watcher.watch(&tasks_dir, notify::RecursiveMode::Recursive);
    }

    // Watch todos directory
    if todos_enabled && todos_dir.exists() {
        let _ = watcher.watch(&todos_dir, notify::RecursiveMode::Recursive);
    }

    // Watch plans directory
    let plans_dir = claude_home.join("plans");
    if plans_enabled && plans_dir.exists() {
        let _ = watcher.watch(&plans_dir, notify::RecursiveMode::NonRecursive);
    }

    // Watch .git directory for git status changes
    let git_dir = project_cwd.join(".git");
    if git_enabled && git_dir.exists() {
        let _ = watcher.watch(&git_dir, notify::RecursiveMode::NonRecursive);
    }

    Ok(debouncer)
}

fn classify_change(
    path_str: &str,
    encoded_project: &str,
    path: &std::path::Path,
) -> Option<FileChange> {
    let normalized = path_str.replace('\\', "/");

    // Git changes — check first before other matchers
    if normalized.contains("/.git/") || normalized.ends_with("/.git") {
        if normalized.ends_with("/index")
            || normalized.ends_with("/HEAD")
            || normalized.contains("/refs/")
        {
            return Some(FileChange::GitChange);
        }
        return None; // ignore noisy .git internals
    }

    // Session index
    if normalized.contains(&format!("projects/{}/sessions-index.json", encoded_project)) {
        return Some(FileChange::SessionIndex);
    }

    // Subagent transcript files (in subagents/ subdirectory)
    if normalized.contains(&format!("projects/{}/", encoded_project))
        && normalized.contains("/subagents/")
        && normalized.ends_with(".jsonl")
    {
        return Some(FileChange::SubagentTranscript(path.to_path_buf()));
    }

    // Transcript files (*.jsonl in project dir)
    if normalized.contains(&format!("projects/{}/", encoded_project))
        && normalized.ends_with(".jsonl")
    {
        return Some(FileChange::Transcript(path.to_path_buf()));
    }

    // Team config
    if normalized.contains("teams/") && normalized.ends_with("config.json") {
        let team_name = extract_segment(&normalized, "teams/");
        if let Some(name) = team_name {
            return Some(FileChange::TeamConfig(name));
        }
    }

    // Team inbox
    if normalized.contains("teams/") && normalized.contains("inboxes/") {
        let team_name = extract_segment(&normalized, "teams/");
        if let Some(name) = team_name {
            return Some(FileChange::TeamInbox(name, String::new()));
        }
    }

    // Task files
    if normalized.contains("tasks/") && normalized.ends_with(".json") {
        let team_name = extract_segment(&normalized, "tasks/");
        if let Some(name) = team_name {
            return Some(FileChange::TaskFile(name));
        }
    }

    // Todo files
    if normalized.contains("todos/") && normalized.ends_with(".json") {
        return Some(FileChange::TodoFile(path.to_path_buf()));
    }

    // Plan files
    if normalized.contains("plans/") && normalized.ends_with(".md") {
        return Some(FileChange::PlanFile(path.to_path_buf()));
    }

    None
}

/// Extract the first path segment after a prefix like "teams/" or "tasks/".
fn extract_segment(path: &str, prefix: &str) -> Option<String> {
    let idx = path.find(prefix)?;
    let rest = &path[idx + prefix.len()..];
    let end = rest.find('/').unwrap_or(rest.len());
    let segment = &rest[..end];
    if segment.is_empty() {
        None
    } else {
        Some(segment.to_string())
    }
}
