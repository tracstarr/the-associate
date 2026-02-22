use std::collections::VecDeque;
use std::path::PathBuf;

/// Maximum number of output/error lines retained per process.
pub const MAX_PROCESS_OUTPUT_LINES: usize = 10_000;

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
    /// Captured stdout lines (raw, kept for debug). Capped at MAX_PROCESS_OUTPUT_LINES.
    pub output_lines: VecDeque<String>,
    /// Captured stderr lines. Capped at MAX_PROCESS_OUTPUT_LINES.
    pub error_lines: VecDeque<String>,
    /// Session ID extracted from stream-json init event.
    pub session_id: Option<String>,
    /// Human-readable parsed progress lines for the UI.
    pub progress_lines: Vec<String>,
}

/// Where the ticket came from.
#[derive(Debug, Clone, PartialEq)]
pub enum TicketSource {
    GitHubPR,
    GitHubIssue,
    Linear,
    Jira,
    /// Spawned directly from the Terminals tab (not from a ticket tracker).
    Manual,
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
