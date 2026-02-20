use std::path::PathBuf;

/// Represents a Claude Code process spawned from a ticket.
#[derive(Debug, Clone)]
pub struct SpawnedProcess {
    /// Unique identifier for this process.
    pub id: usize,
    /// Short label (e.g. "PROJ-123" or "GH #42").
    pub label: String,
    /// The ticket summary / title.
    pub title: String,
    /// Source of the ticket.
    pub source: TicketSource,
    /// Current status.
    pub status: ProcessStatus,
    /// The prompt that was sent to Claude Code.
    pub prompt: String,
    /// Working directory where the process was spawned.
    pub cwd: PathBuf,
    /// Captured stdout lines.
    pub output_lines: Vec<String>,
    /// Captured stderr lines.
    pub error_lines: Vec<String>,
}

/// Where the ticket came from.
#[derive(Debug, Clone, PartialEq)]
pub enum TicketSource {
    GitHubPR,
    Jira,
}

/// Status of a spawned process.
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessStatus {
    Running,
    Completed,
    Failed,
}

/// Info needed to generate a prompt from a ticket.
#[derive(Debug, Clone)]
pub struct TicketInfo {
    pub source: TicketSource,
    pub key: String,
    pub title: String,
    pub description: String,
    pub labels: Vec<String>,
    pub url: String,
    pub extra_fields: Vec<(String, String)>,
}
