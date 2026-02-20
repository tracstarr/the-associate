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
    pub linear: Option<LinearConfig>,
    pub display: Option<DisplayConfig>,
    pub tabs: Option<TabsConfig>,
}

/// Per-tab enable/disable configuration.
/// All tabs default to enabled (`true`). Set a tab to `false` to disable it
/// entirely — its data won't be loaded, watched, or polled.
#[derive(Debug, Clone, Deserialize)]
pub struct TabsConfig {
    pub sessions: Option<bool>,
    pub teams: Option<bool>,
    pub todos: Option<bool>,
    pub git: Option<bool>,
    pub plans: Option<bool>,
    pub github_prs: Option<bool>,
    pub github_issues: Option<bool>,
    pub jira: Option<bool>,
    pub linear: Option<bool>,
}

impl Default for TabsConfig {
    fn default() -> Self {
        Self {
            sessions: None,
            teams: None,
            todos: None,
            git: None,
            plans: None,
            github_prs: None,
            github_issues: None,
            jira: None,
            linear: None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GithubConfig {
    pub repo: Option<String>,
    pub issues: Option<GithubIssuesConfig>,
}

#[derive(Debug, Deserialize)]
pub struct GithubIssuesConfig {
    /// Set to false to disable the Issues tab even when gh is available.
    pub enabled: Option<bool>,
    /// Override the repo for fetching issues (e.g. "owner/repo").
    pub repo: Option<String>,
    /// Issue state filter: "open", "closed", or "all". Default: "open".
    pub state: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct JiraConfig {
    pub project: Option<String>,
    pub jql: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LinearConfig {
    pub api_key: Option<String>,
    pub username: Option<String>,
    pub team: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DisplayConfig {
    pub tick_rate: Option<u64>,
    pub tail_lines: Option<usize>,
}

impl ProjectConfig {
    pub fn tabs_config(&self) -> TabsConfig {
        self.tabs.clone().unwrap_or_default()
    }

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

    /// Whether the Issues tab is explicitly disabled in config.
    pub fn github_issues_enabled(&self) -> bool {
        self.github
            .as_ref()
            .and_then(|g| g.issues.as_ref())
            .and_then(|i| i.enabled)
            .unwrap_or(true)
    }

    /// Override repo for issues (falls back to github.repo / git remote).
    pub fn github_issues_repo(&self) -> Option<&str> {
        self.github
            .as_ref()
            .and_then(|g| g.issues.as_ref())
            .and_then(|i| i.repo.as_deref())
    }

    /// Issue state filter. Default: "open".
    pub fn github_issues_state(&self) -> &str {
        self.github
            .as_ref()
            .and_then(|g| g.issues.as_ref())
            .and_then(|i| i.state.as_deref())
            .unwrap_or("open")
    }

    pub fn jira_project(&self) -> Option<&str> {
        self.jira.as_ref().and_then(|j| j.project.as_deref())
    }

    pub fn jira_jql(&self) -> Option<&str> {
        self.jira.as_ref().and_then(|j| j.jql.as_deref())
    }

    pub fn linear_api_key(&self) -> Option<&str> {
        self.linear.as_ref().and_then(|l| l.api_key.as_deref())
    }

    pub fn linear_username(&self) -> Option<&str> {
        self.linear.as_ref().and_then(|l| l.username.as_deref())
    }

    pub fn linear_team(&self) -> Option<&str> {
        self.linear.as_ref().and_then(|l| l.team.as_deref())
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
