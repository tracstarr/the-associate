use ratatui::style::{Color, Modifier, Style};

// Tab bar
pub const TAB_ACTIVE: Style = Style::new().fg(Color::Black).bg(Color::Cyan);
pub const TAB_INACTIVE: Style = Style::new().fg(Color::Gray).bg(Color::DarkGray);

// Status bar
pub const STATUS_BAR: Style = Style::new().fg(Color::White).bg(Color::DarkGray);

// List items
pub const LIST_SELECTED: Style = Style::new()
    .fg(Color::White)
    .bg(Color::DarkGray)
    .add_modifier(Modifier::BOLD);
pub const LIST_NORMAL: Style = Style::new().fg(Color::White);

// Transcript kinds
pub const TX_USER: Style = Style::new().fg(Color::Green).add_modifier(Modifier::BOLD);
pub const TX_ASSISTANT: Style = Style::new().fg(Color::Cyan);
pub const TX_TOOL: Style = Style::new().fg(Color::Yellow);
pub const TX_RESULT: Style = Style::new().fg(Color::DarkGray);
pub const TX_SYSTEM: Style = Style::new().fg(Color::Magenta);
pub const TX_PROGRESS: Style = Style::new().fg(Color::DarkGray);

// Task status
pub const TASK_PENDING: Style = Style::new().fg(Color::Yellow);
pub const TASK_IN_PROGRESS: Style = Style::new().fg(Color::Cyan);
pub const TASK_COMPLETED: Style = Style::new().fg(Color::Green);

// Borders
pub const BORDER_ACTIVE: Style = Style::new().fg(Color::Cyan);
pub const BORDER_INACTIVE: Style = Style::new().fg(Color::DarkGray);

// Help overlay
pub const HELP_TITLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
pub const HELP_KEY: Style = Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);
pub const HELP_DESC: Style = Style::new().fg(Color::White);

// Follow mode indicator
pub const FOLLOW_ACTIVE: Style = Style::new()
    .fg(Color::Black)
    .bg(Color::Green)
    .add_modifier(Modifier::BOLD);

// Agent status
pub const AGENT_STARTING: Style = Style::new().fg(Color::Yellow);
pub const AGENT_WORKING: Style = Style::new().fg(Color::Green);
pub const AGENT_IDLE: Style = Style::new().fg(Color::DarkGray);
pub const AGENT_SHUTDOWN: Style = Style::new().fg(Color::Red);
pub const AGENT_LEAD: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);

// Subagent indicator
pub const SUBAGENT_BADGE: Style = Style::new().fg(Color::Magenta);
pub const SUBAGENT_TAB_ACTIVE: Style = Style::new()
    .fg(Color::Black)
    .bg(Color::Magenta)
    .add_modifier(Modifier::BOLD);
pub const SUBAGENT_TAB_INACTIVE: Style = Style::new().fg(Color::Magenta);

// Footer hints
pub const HINT_KEY: Style = Style::new().fg(Color::Yellow).bg(Color::DarkGray);
pub const HINT_DESC: Style = Style::new().fg(Color::Gray).bg(Color::DarkGray);

// Empty state
pub const EMPTY_STATE: Style = Style::new().fg(Color::DarkGray);

// Branch label
pub const BRANCH_LABEL: Style = Style::new().fg(Color::Yellow);

// Git diff
pub const DIFF_ADD: Style = Style::new().fg(Color::Green);
pub const DIFF_REMOVE: Style = Style::new().fg(Color::Red);
pub const DIFF_HUNK: Style = Style::new().fg(Color::Cyan);
pub const DIFF_HEADER: Style = Style::new().fg(Color::White).add_modifier(Modifier::DIM);

// Markdown styles
pub const MD_HEADING: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
pub const MD_CODE_FENCE: Style = Style::new().fg(Color::DarkGray);
pub const MD_CODE_BLOCK: Style = Style::new().fg(Color::Yellow);
pub const MD_NORMAL: Style = Style::new().fg(Color::White);

// Git section headers
pub const GIT_STAGED: Style = Style::new().fg(Color::Green).add_modifier(Modifier::BOLD);
pub const GIT_UNSTAGED: Style = Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);
pub const GIT_UNTRACKED: Style = Style::new()
    .fg(Color::DarkGray)
    .add_modifier(Modifier::BOLD);

// File browser
pub const FB_DIR: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
pub const FB_FILE: Style = Style::new().fg(Color::White);
pub const FB_LINE_NUMBER: Style = Style::new().fg(Color::DarkGray);
pub const FB_EDIT_BORDER: Style = Style::new().fg(Color::Yellow);

// GitHub PRs
pub const PR_APPROVED: Style = Style::new().fg(Color::Green);
pub const PR_CHANGES_REQUESTED: Style = Style::new().fg(Color::Red);
pub const PR_PENDING_REVIEW: Style = Style::new().fg(Color::Yellow);
pub const PR_DRAFT: Style = Style::new().fg(Color::DarkGray);
pub const PR_SIZE: Style = Style::new().fg(Color::Magenta);
pub const PR_SECTION: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
pub const PR_BADGE: Style = Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);

// Jira
pub const JIRA_TODO: Style = Style::new()
    .fg(Color::DarkGray)
    .add_modifier(Modifier::BOLD);
pub const JIRA_IN_PROGRESS: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
pub const JIRA_DONE: Style = Style::new().fg(Color::Green).add_modifier(Modifier::BOLD);
pub const JIRA_BUG: Style = Style::new().fg(Color::Red);
pub const JIRA_STORY: Style = Style::new().fg(Color::Green);
pub const JIRA_TASK: Style = Style::new().fg(Color::Blue);
pub const JIRA_SEARCH_INPUT: Style = Style::new().fg(Color::Yellow);
pub const JIRA_TRANSITION_POPUP: Style = Style::new().fg(Color::White).bg(Color::DarkGray);
