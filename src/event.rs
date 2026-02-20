use std::path::PathBuf;

use crate::model::git::{DiffLine, GitStatus};
use crate::model::github::{GitHubIssue, PullRequest};
use crate::model::jira::JiraIssue;
use crate::model::linear::LinearIssue;

/// All events the app loop handles.
#[derive(Debug)]
pub enum AppEvent {
    /// A watched file was created or modified.
    FileChanged(FileChange),
    /// Pane send completed: None = success, Some = error message.
    PaneSendComplete(Option<String>),
    /// Background load of GitHub PRs completed.
    GitHubPrsLoaded(Result<Vec<PullRequest>, String>),
    /// Background load of GitHub Issues completed.
    GitHubIssuesLoaded(Result<Vec<GitHubIssue>, String>),
    /// Background load of Jira issues completed.
    JiraIssuesLoaded(Result<Vec<JiraIssue>, String>),
    /// Background load of Linear issues completed.
    LinearIssuesLoaded(Result<Vec<LinearIssue>, String>),
    /// Background load of git status completed.
    GitStatusLoaded(Result<GitStatus, String>),
    /// Background load of git diff completed.
    GitDiffLoaded(Result<Vec<DiffLine>, String>),
}

/// Categorized file change from the watcher.
#[derive(Debug, Clone)]
pub enum FileChange {
    SessionIndex,
    Transcript(PathBuf),
    SubagentTranscript(PathBuf),
    TeamConfig(String),
    TeamInbox(String, String),
    TaskFile(String),
    TodoFile(PathBuf),
    GitChange,
    PlanFile(PathBuf),
}
