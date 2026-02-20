use std::path::PathBuf;

/// All events the app loop handles.
#[derive(Debug)]
pub enum AppEvent {
    /// A watched file was created or modified.
    FileChanged(FileChange),
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
