use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Base path for all Claude Code data.
pub fn claude_home() -> PathBuf {
    dirs_base().join(".claude")
}

fn dirs_base() -> PathBuf {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// How often the tick event fires (ms).
pub const TICK_RATE_MS: u64 = 250;

/// File watcher debounce interval (ms).
pub const DEBOUNCE_MS: u64 = 200;

/// How many lines to load from end of JSONL on initial read.
pub const JSONL_TAIL_LINES: usize = 200;

// ---------------------------------------------------------------------------
// Project config (.assoc.toml)
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
pub struct ProjectConfig {
    pub github: Option<GithubConfig>,
    pub jira: Option<JiraConfig>,
    pub display: Option<DisplayConfig>,
}

#[derive(Debug, Deserialize)]
pub struct GithubConfig {
    pub repo: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct JiraConfig {
    pub project: Option<String>,
    pub jql: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DisplayConfig {
    pub tick_rate: Option<u64>,
    pub tail_lines: Option<usize>,
}

impl ProjectConfig {
    pub fn tick_rate(&self) -> u64 {
        self.display
            .as_ref()
            .and_then(|d| d.tick_rate)
            .unwrap_or(TICK_RATE_MS)
    }

    pub fn tail_lines(&self) -> usize {
        self.display
            .as_ref()
            .and_then(|d| d.tail_lines)
            .unwrap_or(JSONL_TAIL_LINES)
    }

    pub fn github_repo(&self) -> Option<&str> {
        self.github.as_ref().and_then(|g| g.repo.as_deref())
    }

    pub fn jira_project(&self) -> Option<&str> {
        self.jira.as_ref().and_then(|j| j.project.as_deref())
    }

    pub fn jira_jql(&self) -> Option<&str> {
        self.jira.as_ref().and_then(|j| j.jql.as_deref())
    }
}

/// Load project config from `.assoc.toml` in the given directory.
/// Returns default config if the file doesn't exist or can't be parsed.
pub fn load_project_config(cwd: &Path) -> ProjectConfig {
    let path = cwd.join(".assoc.toml");
    if path.exists() {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        toml::from_str(&content).unwrap_or_default()
    } else {
        ProjectConfig::default()
    }
}
