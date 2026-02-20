use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Child;
use std::sync::mpsc;
use std::time::Instant;

use crate::config::{self, ProjectConfig};
use crate::data::{
    cli_detect, filebrowser, git, github, inboxes, jira, linear, path_encoding, plans,
    process_runner::{self, ProcessOutput},
    prompt_builder, sessions, subagents, tasks, teams, todos, transcripts,
};
use crate::event::FileChange;
use crate::model::agent_status::{self, AgentStatus};
use crate::model::filebrowser::{FileBrowserEntry, FileContent};
use crate::model::git::{DiffLine, FlatGitItem, GitStatus};
use crate::model::github::{FlatIssueItem, FlatPrItem, GitHubIssue, PullRequest};
use crate::model::inbox::InboxMessage;
use crate::model::jira::{FlatJiraItem, JiraIssue, JiraTransition};
use crate::model::linear::{FlatLinearItem, LinearIssue};
use crate::model::plan::{MarkdownLine, PlanFile as PlanFileModel};
use crate::model::process::{ProcessStatus, SpawnedProcess, TicketInfo};
use crate::model::session::SessionEntry;
use crate::model::task::Task;
use crate::model::team::{Team, TeamMember};
use crate::model::todo::{TodoFile, TodoItem};
use crate::model::transcript::TranscriptItem;

#[derive(Debug, Clone, PartialEq)]
pub enum ActiveTab {
    Sessions,
    Teams,
    Todos,
    Git,
    Plans,
    GitHubPRs,
    GitHubIssues,
    Jira,
    Linear,
    Processes,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessesPane {
    List,
    Output,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GitPane {
    Files,
    Diff,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GitMode {
    Status,
    Browse,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileBrowserPane {
    Tree,
    Content,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlansPane {
    List,
    Content,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SessionsPane {
    List,
    Transcript,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TeamsPane {
    Teams,
    Members,
    Tasks,
    Detail,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GitHubPane {
    List,
    Detail,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IssuesPane {
    List,
    Detail,
}

/// Mode for the issues editor overlay (create or edit).
#[derive(Debug, Clone, PartialEq)]
pub enum IssueEditMode {
    Create,
    Edit(u64), // issue number
    Comment(u64),
}

/// Which field is focused in the issue editor.
#[derive(Debug, Clone, PartialEq)]
pub enum IssueEditField {
    Title,
    Body,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JiraPane {
    List,
    Detail,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LinearPane {
    List,
    Detail,
}

pub struct App {
    pub should_quit: bool,
    pub active_tab: ActiveTab,
    pub show_help: bool,

    // Config
    pub project_config: ProjectConfig,

    // Paths
    pub project_cwd: PathBuf,
    pub claude_home: PathBuf,
    pub encoded_project: String,

    // Sessions tab
    pub sessions: Vec<SessionEntry>,
    pub session_list_index: usize,
    pub sessions_pane: SessionsPane,
    pub transcript_reader: transcripts::TranscriptReader,
    pub transcript_items: Vec<TranscriptItem>,
    pub transcript_scroll: usize,
    pub follow_mode: bool,
    pub loaded_session_id: Option<String>,

    // Subagent transcripts
    pub subagents: Vec<subagents::SubagentInfo>,
    pub subagent_index: usize,
    pub subagent_transcript: Vec<TranscriptItem>,
    pub subagent_reader: transcripts::TranscriptReader,
    pub viewing_subagent: bool,

    // Teams tab
    pub teams: Vec<Team>,
    pub team_list_index: usize,
    pub member_list_index: usize,
    pub task_list_index: usize,
    pub teams_pane: TeamsPane,
    pub tasks: Vec<Task>,
    pub inbox_messages: Vec<InboxMessage>,
    pub agent_statuses: HashMap<String, AgentStatus>,
    pub detail_scroll: usize,

    // Todos tab
    pub todo_files: Vec<TodoFile>,
    pub todo_file_index: usize,
    pub todo_item_index: usize,
    pub todos_pane_left: bool,

    // Plans tab
    pub plan_files: Vec<PlanFileModel>,
    pub plan_file_index: usize,
    pub plans_pane: PlansPane,
    pub plan_content_scroll: usize,

    // Git tab
    pub git_status: GitStatus,
    pub git_flat_list: Vec<FlatGitItem>,
    pub git_file_index: usize,
    pub git_pane: GitPane,
    pub git_diff_lines: Vec<DiffLine>,
    pub diff_scroll: usize,

    // File browser (Git tab browse mode)
    pub git_mode: GitMode,
    pub fb_entries: Vec<FileBrowserEntry>,
    pub fb_index: usize,
    pub fb_expanded: HashSet<PathBuf>,
    pub fb_content: Option<FileContent>,
    pub fb_content_path: Option<PathBuf>,
    pub fb_content_scroll: usize,
    pub fb_pane: FileBrowserPane,
    pub fb_editing: bool,
    pub fb_editor: Option<tui_textarea::TextArea<'static>>,

    // GitHub PRs tab
    pub has_gh: bool,
    pub gh_repo: Option<String>,
    pub gh_user: Option<String>,
    pub gh_prs: Vec<PullRequest>,
    pub gh_flat_list: Vec<FlatPrItem>,
    pub gh_pr_index: usize,
    pub gh_pane: GitHubPane,
    pub gh_detail_scroll: usize,
    pub gh_last_poll: Instant,
    pub gh_prev_updated: HashMap<u64, String>,
    pub gh_new_activity: bool,

    // GitHub Issues tab
    pub gh_issues_enabled: bool,
    pub gh_issues_repo: Option<String>,
    pub gh_issues: Vec<GitHubIssue>,
    pub gh_issues_flat_list: Vec<FlatIssueItem>,
    pub gh_issues_index: usize,
    pub gh_issues_pane: IssuesPane,
    pub gh_issues_detail_scroll: usize,
    pub gh_issues_last_poll: Instant,
    pub gh_issues_editing: bool,
    pub gh_issues_edit_mode: Option<IssueEditMode>,
    pub gh_issues_edit_field: IssueEditField,
    pub gh_issues_title_editor: Option<tui_textarea::TextArea<'static>>,
    pub gh_issues_body_editor: Option<tui_textarea::TextArea<'static>>,

    // Jira tab
    pub has_jira: bool,
    pub jira_issues: Vec<JiraIssue>,
    pub jira_flat_list: Vec<FlatJiraItem>,
    pub jira_index: usize,
    pub jira_pane: JiraPane,
    pub jira_detail_scroll: usize,
    pub jira_detail: Option<JiraIssue>,
    pub jira_search_mode: bool,
    pub jira_search_input: String,
    pub jira_show_transitions: bool,
    pub jira_transitions: Vec<JiraTransition>,
    pub jira_last_poll: Instant,

    // Linear tab
    pub has_linear: bool,
    pub linear_issues: Vec<LinearIssue>,
    pub linear_flat_list: Vec<FlatLinearItem>,
    pub linear_index: usize,
    pub linear_pane: LinearPane,
    pub linear_detail_scroll: usize,
    pub linear_last_poll: Instant,

    // Delete confirmation
    pub confirm_delete: bool,
    pub delete_target_name: String,

    // Processes tab
    pub has_claude: bool,
    pub processes: Vec<SpawnedProcess>,
    pub process_children: Vec<(usize, Child)>,
    pub process_index: usize,
    pub process_output_scroll: usize,
    pub processes_pane: ProcessesPane,
    pub process_tx: Option<mpsc::Sender<ProcessOutput>>,
    pub process_rx: Option<mpsc::Receiver<ProcessOutput>>,
    pub next_process_id: usize,

    // Prompt modal
    pub show_prompt_modal: bool,
    pub prompt_editor: Option<tui_textarea::TextArea<'static>>,
    pub prompt_ticket_info: Option<TicketInfo>,

    // Status
    pub last_update: Instant,
    pub last_error: Option<String>,
}

impl App {
    pub fn new(project_cwd: PathBuf) -> Self {
        let claude_home = config::claude_home();
        let encoded_project = path_encoding::encode_project_path(&project_cwd);
        let project_config = config::load_project_config(&project_cwd);

        let has_gh = cli_detect::is_available("gh");
        let has_jira = cli_detect::is_available("acli");
        let has_linear = project_config.linear_api_key().is_some();
        let has_claude = cli_detect::is_available("claude");
        // Config github.repo overrides git remote detection
        let gh_repo = project_config.github_repo().map(String::from).or_else(|| {
            if has_gh {
                cli_detect::detect_gh_repo(&project_cwd)
            } else {
                None
            }
        });
        let gh_user = if has_gh {
            cli_detect::detect_gh_user()
        } else {
            None
        };

        // Determine issues repo: config issues.repo > config github.repo > git remote
        let gh_issues_repo = project_config
            .github_issues_repo()
            .map(String::from)
            .or_else(|| gh_repo.clone());

        // Show Issues tab if gh is available, repo is known, and config doesn't disable it.
        // We don't pre-check hasIssuesEnabled — if issues can't be fetched, the tab shows an error.
        let gh_issues_enabled = has_gh
            && gh_issues_repo.is_some()
            && project_config.github_issues_enabled();

        let tail_lines = project_config.tail_lines();

        App {
            should_quit: false,
            active_tab: ActiveTab::Sessions,
            show_help: false,

            project_config,
            project_cwd,
            claude_home,
            encoded_project,

            sessions: Vec::new(),
            session_list_index: 0,
            sessions_pane: SessionsPane::List,
            transcript_reader: transcripts::TranscriptReader::with_tail_lines(tail_lines),
            transcript_items: Vec::new(),
            transcript_scroll: 0,
            follow_mode: true,
            loaded_session_id: None,

            subagents: Vec::new(),
            subagent_index: 0,
            subagent_transcript: Vec::new(),
            subagent_reader: transcripts::TranscriptReader::with_tail_lines(tail_lines),
            viewing_subagent: false,

            teams: Vec::new(),
            team_list_index: 0,
            member_list_index: 0,
            task_list_index: 0,
            teams_pane: TeamsPane::Teams,
            tasks: Vec::new(),
            inbox_messages: Vec::new(),
            agent_statuses: HashMap::new(),
            detail_scroll: 0,

            todo_files: Vec::new(),
            todo_file_index: 0,
            todo_item_index: 0,
            todos_pane_left: true,

            plan_files: Vec::new(),
            plan_file_index: 0,
            plans_pane: PlansPane::List,
            plan_content_scroll: 0,

            git_status: GitStatus::default(),
            git_flat_list: Vec::new(),
            git_file_index: 0,
            git_pane: GitPane::Files,
            git_diff_lines: Vec::new(),
            diff_scroll: 0,

            git_mode: GitMode::Status,
            fb_entries: Vec::new(),
            fb_index: 0,
            fb_expanded: HashSet::new(),
            fb_content: None,
            fb_content_path: None,
            fb_content_scroll: 0,
            fb_pane: FileBrowserPane::Tree,
            fb_editing: false,
            fb_editor: None,

            has_gh,
            gh_repo,
            gh_user,
            gh_prs: Vec::new(),
            gh_flat_list: Vec::new(),
            gh_pr_index: 0,
            gh_pane: GitHubPane::List,
            gh_detail_scroll: 0,
            gh_last_poll: Instant::now(),
            gh_prev_updated: HashMap::new(),
            gh_new_activity: false,

            gh_issues_enabled,
            gh_issues_repo,
            gh_issues: Vec::new(),
            gh_issues_flat_list: Vec::new(),
            gh_issues_index: 0,
            gh_issues_pane: IssuesPane::List,
            gh_issues_detail_scroll: 0,
            gh_issues_last_poll: Instant::now(),
            gh_issues_editing: false,
            gh_issues_edit_mode: None,
            gh_issues_edit_field: IssueEditField::Title,
            gh_issues_title_editor: None,
            gh_issues_body_editor: None,

            has_jira,
            jira_issues: Vec::new(),
            jira_flat_list: Vec::new(),
            jira_index: 0,
            jira_pane: JiraPane::List,
            jira_detail_scroll: 0,
            jira_detail: None,
            jira_search_mode: false,
            jira_search_input: String::new(),
            jira_show_transitions: false,
            jira_transitions: Vec::new(),
            jira_last_poll: Instant::now(),

            has_linear,
            linear_issues: Vec::new(),
            linear_flat_list: Vec::new(),
            linear_index: 0,
            linear_pane: LinearPane::List,
            linear_detail_scroll: 0,
            linear_last_poll: Instant::now(),

            confirm_delete: false,
            delete_target_name: String::new(),

            has_claude,
            processes: Vec::new(),
            process_children: Vec::new(),
            process_index: 0,
            process_output_scroll: 0,
            processes_pane: ProcessesPane::List,
            process_tx: None,
            process_rx: None,
            next_process_id: 1,

            show_prompt_modal: false,
            prompt_editor: None,
            prompt_ticket_info: None,

            last_update: Instant::now(),
            last_error: None,
        }
    }

    /// Return the list of tabs that should be visible based on CLI availability.
    pub fn visible_tabs(&self) -> Vec<ActiveTab> {
        let mut tabs = vec![
            ActiveTab::Sessions,
            ActiveTab::Teams,
            ActiveTab::Todos,
            ActiveTab::Git,
            ActiveTab::Plans,
        ];
        if self.has_gh && self.gh_repo.is_some() {
            tabs.push(ActiveTab::GitHubPRs);
        }
        if self.gh_issues_enabled {
            tabs.push(ActiveTab::GitHubIssues);
        }
        if self.has_jira {
            tabs.push(ActiveTab::Jira);
        }
        if self.has_linear {
            tabs.push(ActiveTab::Linear);
        }
        if !self.processes.is_empty() {
            tabs.push(ActiveTab::Processes);
        }
        tabs
    }

    /// Load all data from disk.
    pub fn load_all(&mut self) {
        self.load_sessions();
        self.load_teams();
        self.load_todos();
        self.load_git_data();
        self.load_plans();
        self.load_github_prs();
        self.load_github_issues();
        self.load_jira_issues();
        self.load_linear_issues();
        self.last_update = Instant::now();
    }

    pub fn load_sessions(&mut self) {
        let project_dir = self
            .claude_home
            .join("projects")
            .join(&self.encoded_project);

        match sessions::load_sessions(&project_dir) {
            Ok(entries) => {
                self.sessions = entries;
                if !self.sessions.is_empty() {
                    if self.loaded_session_id.is_none() {
                        // First load — show most recent session
                        self.load_selected_transcript();
                    } else if self.follow_mode && self.session_list_index == 0 {
                        // Follow mode + viewing top session: auto-switch to new latest
                        let newest_id = &self.sessions[0].session_id;
                        if self.loaded_session_id.as_deref() != Some(newest_id) {
                            self.loaded_session_id = None; // force reload
                            self.load_selected_transcript();
                        }
                    }
                }
                self.last_error = None;
            }
            Err(e) => {
                self.last_error = Some(format!("Sessions: {}", e));
            }
        }
    }

    pub fn load_selected_transcript(&mut self) {
        if self.sessions.is_empty() {
            return;
        }

        let idx = self.session_list_index.min(self.sessions.len() - 1);
        let session = &self.sessions[idx];
        let session_id = session.session_id.clone();

        // Don't reload if same session
        if self.loaded_session_id.as_ref() == Some(&session_id) {
            return;
        }

        let project_dir = self
            .claude_home
            .join("projects")
            .join(&self.encoded_project);
        let transcript_path = project_dir.join(format!("{}.jsonl", session_id));

        self.transcript_reader =
            transcripts::TranscriptReader::with_tail_lines(self.project_config.tail_lines());
        match self.transcript_reader.load_initial(&transcript_path) {
            Ok(()) => {
                self.transcript_items = self.transcript_reader.items.clone();
                self.loaded_session_id = Some(session_id.clone());
                if self.follow_mode {
                    self.transcript_scroll = self.transcript_items.len();
                }
            }
            Err(e) => {
                self.last_error = Some(format!("Transcript: {}", e));
            }
        }

        // Scan for subagents
        self.subagents = subagents::find_subagents(&project_dir, &session_id);
        self.subagent_index = 0;
        self.subagent_transcript.clear();
        self.subagent_reader =
            transcripts::TranscriptReader::with_tail_lines(self.project_config.tail_lines());
        self.viewing_subagent = false;
    }

    pub fn refresh_transcript(&mut self) {
        if let Some(ref session_id) = self.loaded_session_id.clone() {
            let project_dir = self
                .claude_home
                .join("projects")
                .join(&self.encoded_project);
            let transcript_path = project_dir.join(format!("{}.jsonl", session_id));

            match self.transcript_reader.read_new(&transcript_path) {
                Ok(true) => {
                    self.transcript_items = self.transcript_reader.items.clone();
                    if self.follow_mode {
                        self.transcript_scroll = self.transcript_items.len();
                    }
                    self.last_update = Instant::now();
                }
                Ok(false) => {}
                Err(e) => {
                    self.last_error = Some(format!("Transcript update: {}", e));
                }
            }
        }
    }

    pub fn load_teams(&mut self) {
        match teams::load_teams(&self.claude_home, Some(&self.project_cwd)) {
            Ok(t) => {
                self.teams = t;
                self.load_tasks_for_selected_team();
                self.compute_agent_statuses();
                self.last_error = None;
            }
            Err(e) => {
                self.last_error = Some(format!("Teams: {}", e));
            }
        }
    }

    /// Compute agent statuses for the currently selected team.
    fn compute_agent_statuses(&mut self) {
        self.agent_statuses.clear();

        if self.teams.is_empty() {
            return;
        }

        let idx = self.team_list_index.min(self.teams.len() - 1);
        let team = &self.teams[idx];

        // Find the lead agent name to load their inbox
        let lead_name = team.config.members.iter().find_map(|m| {
            if m.is_lead(&team.config) {
                Some(m.name.clone())
            } else {
                None
            }
        });

        // Load the lead's inbox (or first member if no lead identified)
        let lead_inbox_name = lead_name
            .as_deref()
            .or_else(|| team.config.members.first().map(|m| m.name.as_str()));

        let lead_inbox = if let Some(name) = lead_inbox_name {
            inboxes::load_inbox(&self.claude_home, &team.dir_name, name).unwrap_or_default()
        } else {
            Vec::new()
        };

        let member_names: Vec<&str> = team
            .config
            .members
            .iter()
            .map(|m| m.name.as_str())
            .collect();
        self.agent_statuses =
            agent_status::derive_all_statuses(&member_names, &lead_inbox, &self.tasks);
    }

    pub fn load_tasks_for_selected_team(&mut self) {
        if self.teams.is_empty() {
            self.tasks = Vec::new();
            return;
        }
        let idx = self.team_list_index.min(self.teams.len() - 1);
        let team_name = &self.teams[idx].dir_name;

        match tasks::load_tasks(&self.claude_home, team_name) {
            Ok(t) => self.tasks = t,
            Err(e) => {
                self.last_error = Some(format!("Tasks: {}", e));
            }
        }
    }

    pub fn load_inbox_for_selected_member(&mut self) {
        if self.teams.is_empty() {
            self.inbox_messages = Vec::new();
            return;
        }
        let team_idx = self.team_list_index.min(self.teams.len() - 1);
        let team_name = &self.teams[team_idx].dir_name;

        let members = self.current_team_members();
        if members.is_empty() {
            self.inbox_messages = Vec::new();
            return;
        }
        let member_idx = self.member_list_index.min(members.len() - 1);
        let agent_name = &members[member_idx].name;

        match inboxes::load_inbox(&self.claude_home, team_name, agent_name) {
            Ok(msgs) => self.inbox_messages = msgs,
            Err(_) => self.inbox_messages = Vec::new(),
        }
    }

    pub fn load_plans(&mut self) {
        match plans::load_plans(&self.claude_home) {
            Ok(p) => {
                self.plan_files = p;
                if !self.plan_files.is_empty() && self.plan_file_index >= self.plan_files.len() {
                    self.plan_file_index = self.plan_files.len() - 1;
                }
                self.last_error = None;
            }
            Err(e) => {
                self.last_error = Some(format!("Plans: {}", e));
            }
        }
    }

    pub fn current_plan_lines(&self) -> &[MarkdownLine] {
        if self.plan_files.is_empty() {
            return &[];
        }
        let idx = self.plan_file_index.min(self.plan_files.len() - 1);
        &self.plan_files[idx].lines
    }

    pub fn load_todos(&mut self) {
        match todos::load_todos(&self.claude_home) {
            Ok(t) => {
                self.todo_files = t;
                self.last_error = None;
            }
            Err(e) => {
                self.last_error = Some(format!("Todos: {}", e));
            }
        }
    }

    /// Cycle to the next subagent transcript (or back to main).
    pub fn cycle_subagent(&mut self) {
        if self.subagents.is_empty() {
            return;
        }

        if !self.viewing_subagent {
            // Switch to first subagent
            self.viewing_subagent = true;
            self.subagent_index = 0;
        } else {
            // Cycle through subagents, then back to main
            self.subagent_index += 1;
            if self.subagent_index >= self.subagents.len() {
                self.viewing_subagent = false;
                self.subagent_index = 0;
                return;
            }
        }

        self.load_subagent_transcript();
    }

    /// Load the transcript for the currently selected subagent.
    fn load_subagent_transcript(&mut self) {
        if self.subagent_index >= self.subagents.len() {
            return;
        }
        let path = self.subagents[self.subagent_index].path.clone();
        self.subagent_reader =
            transcripts::TranscriptReader::with_tail_lines(self.project_config.tail_lines());
        match self.subagent_reader.load_initial(&path) {
            Ok(()) => {
                self.subagent_transcript = self.subagent_reader.items.clone();
            }
            Err(e) => {
                self.last_error = Some(format!("Subagent transcript: {}", e));
            }
        }
    }

    /// Refresh the subagent transcript if viewing one.
    fn refresh_subagent_transcript(&mut self) {
        if !self.viewing_subagent || self.subagent_index >= self.subagents.len() {
            return;
        }
        let path = self.subagents[self.subagent_index].path.clone();
        match self.subagent_reader.read_new(&path) {
            Ok(true) => {
                self.subagent_transcript = self.subagent_reader.items.clone();
            }
            Ok(false) => {}
            Err(e) => {
                self.last_error = Some(format!("Subagent update: {}", e));
            }
        }
    }

    /// Handle a file change event from the watcher.
    pub fn handle_file_change(&mut self, change: FileChange) {
        match change {
            FileChange::SessionIndex => {
                self.load_sessions();
            }
            FileChange::Transcript(_path) => {
                self.refresh_transcript();
            }
            FileChange::SubagentTranscript(_path) => {
                self.refresh_subagent_transcript();
            }
            FileChange::TeamConfig(_) => {
                self.load_teams();
            }
            FileChange::TeamInbox(_, _) => {
                self.load_inbox_for_selected_member();
                self.compute_agent_statuses();
            }
            FileChange::TaskFile(_team) => {
                self.load_tasks_for_selected_team();
                self.compute_agent_statuses();
            }
            FileChange::TodoFile(_) => {
                self.load_todos();
            }
            FileChange::GitChange => {
                self.load_git_data();
            }
            FileChange::PlanFile(_) => {
                self.load_plans();
            }
        }
        self.last_update = Instant::now();
    }

    // --- Navigation helpers ---

    pub fn next_tab(&mut self) {
        let tabs = self.visible_tabs();
        if let Some(idx) = tabs.iter().position(|t| *t == self.active_tab) {
            let next = (idx + 1) % tabs.len();
            self.on_tab_switch(&tabs[next]);
            self.active_tab = tabs[next].clone();
        }
    }

    pub fn prev_tab(&mut self) {
        let tabs = self.visible_tabs();
        if let Some(idx) = tabs.iter().position(|t| *t == self.active_tab) {
            let prev = if idx == 0 { tabs.len() - 1 } else { idx - 1 };
            self.on_tab_switch(&tabs[prev]);
            self.active_tab = tabs[prev].clone();
        }
    }

    pub fn switch_to_tab(&mut self, tab: ActiveTab) {
        self.on_tab_switch(&tab);
        self.active_tab = tab;
    }

    fn on_tab_switch(&mut self, target: &ActiveTab) {
        // Clear new-activity badge when switching to that tab
        if *target == ActiveTab::GitHubPRs {
            self.gh_new_activity = false;
        }
    }

    pub fn navigate_down(&mut self) {
        match self.active_tab {
            ActiveTab::Sessions => match self.sessions_pane {
                SessionsPane::List => {
                    if !self.sessions.is_empty() {
                        self.session_list_index =
                            (self.session_list_index + 1).min(self.sessions.len() - 1);
                    }
                }
                SessionsPane::Transcript => {
                    self.follow_mode = false;
                    self.transcript_scroll = self
                        .transcript_scroll
                        .saturating_add(1)
                        .min(self.transcript_items.len().saturating_sub(1));
                }
            },
            ActiveTab::Teams => match self.teams_pane {
                TeamsPane::Teams => {
                    if !self.teams.is_empty() {
                        self.team_list_index = (self.team_list_index + 1).min(self.teams.len() - 1);
                        self.member_list_index = 0;
                        self.task_list_index = 0;
                        self.detail_scroll = 0;
                        self.load_tasks_for_selected_team();
                        self.compute_agent_statuses();
                    }
                }
                TeamsPane::Members => {
                    let members = self.current_team_members();
                    if !members.is_empty() {
                        self.member_list_index =
                            (self.member_list_index + 1).min(members.len() - 1);
                        self.detail_scroll = 0;
                        self.load_inbox_for_selected_member();
                    }
                }
                TeamsPane::Tasks => {
                    if !self.tasks.is_empty() {
                        self.task_list_index = (self.task_list_index + 1).min(self.tasks.len() - 1);
                        self.detail_scroll = 0;
                    }
                }
                TeamsPane::Detail => {
                    self.detail_scroll = self.detail_scroll.saturating_add(1);
                }
            },
            ActiveTab::Todos => {
                if self.todos_pane_left {
                    if !self.todo_files.is_empty() {
                        self.todo_file_index =
                            (self.todo_file_index + 1).min(self.todo_files.len() - 1);
                        self.todo_item_index = 0;
                    }
                } else {
                    let items = self.current_todo_items();
                    if !items.is_empty() {
                        self.todo_item_index = (self.todo_item_index + 1).min(items.len() - 1);
                    }
                }
            }
            ActiveTab::Git => {
                if self.git_mode == GitMode::Browse {
                    self.fb_navigate_down();
                } else {
                    match self.git_pane {
                        GitPane::Files => {
                            self.skip_to_next_file();
                            self.load_selected_diff();
                        }
                        GitPane::Diff => {
                            self.diff_scroll = self.diff_scroll.saturating_add(1);
                        }
                    }
                }
            }
            ActiveTab::Plans => match self.plans_pane {
                PlansPane::List => {
                    if !self.plan_files.is_empty() {
                        self.plan_file_index =
                            (self.plan_file_index + 1).min(self.plan_files.len() - 1);
                        self.plan_content_scroll = 0;
                    }
                }
                PlansPane::Content => {
                    self.plan_content_scroll = self.plan_content_scroll.saturating_add(1);
                }
            },
            ActiveTab::GitHubPRs => match self.gh_pane {
                GitHubPane::List => {
                    self.gh_skip_to_next_pr();
                }
                GitHubPane::Detail => {
                    self.gh_detail_scroll = self.gh_detail_scroll.saturating_add(1);
                }
            },
            ActiveTab::GitHubIssues => match self.gh_issues_pane {
                IssuesPane::List => {
                    self.issues_skip_to_next();
                }
                IssuesPane::Detail => {
                    self.gh_issues_detail_scroll = self.gh_issues_detail_scroll.saturating_add(1);
                }
            },
            ActiveTab::Jira => match self.jira_pane {
                JiraPane::List => {
                    self.jira_skip_to_next_issue();
                }
                JiraPane::Detail => {
                    self.jira_detail_scroll = self.jira_detail_scroll.saturating_add(1);
                }
            },
            ActiveTab::Linear => match self.linear_pane {
                LinearPane::List => {
                    self.linear_skip_to_next_issue();
                }
                LinearPane::Detail => {
                    self.linear_detail_scroll = self.linear_detail_scroll.saturating_add(1);
                }
            },
            ActiveTab::Processes => match self.processes_pane {
                ProcessesPane::List => {
                    if !self.processes.is_empty() {
                        self.process_index = (self.process_index + 1).min(self.processes.len() - 1);
                        self.process_output_scroll = 0;
                    }
                }
                ProcessesPane::Output => {
                    self.process_output_scroll = self.process_output_scroll.saturating_add(1);
                }
            },
        }
    }

    pub fn navigate_up(&mut self) {
        match self.active_tab {
            ActiveTab::Sessions => match self.sessions_pane {
                SessionsPane::List => {
                    self.session_list_index = self.session_list_index.saturating_sub(1);
                }
                SessionsPane::Transcript => {
                    self.follow_mode = false;
                    self.transcript_scroll = self.transcript_scroll.saturating_sub(1);
                }
            },
            ActiveTab::Teams => match self.teams_pane {
                TeamsPane::Teams => {
                    if self.team_list_index > 0 {
                        self.team_list_index -= 1;
                        self.member_list_index = 0;
                        self.task_list_index = 0;
                        self.detail_scroll = 0;
                        self.load_tasks_for_selected_team();
                        self.compute_agent_statuses();
                    }
                }
                TeamsPane::Members => {
                    if self.member_list_index > 0 {
                        self.member_list_index -= 1;
                        self.detail_scroll = 0;
                        self.load_inbox_for_selected_member();
                    }
                }
                TeamsPane::Tasks => {
                    if self.task_list_index > 0 {
                        self.task_list_index -= 1;
                        self.detail_scroll = 0;
                    }
                }
                TeamsPane::Detail => {
                    self.detail_scroll = self.detail_scroll.saturating_sub(1);
                }
            },
            ActiveTab::Todos => {
                if self.todos_pane_left {
                    if self.todo_file_index > 0 {
                        self.todo_file_index -= 1;
                        self.todo_item_index = 0;
                    }
                } else {
                    self.todo_item_index = self.todo_item_index.saturating_sub(1);
                }
            }
            ActiveTab::Git => {
                if self.git_mode == GitMode::Browse {
                    self.fb_navigate_up();
                } else {
                    match self.git_pane {
                        GitPane::Files => {
                            self.skip_to_prev_file();
                            self.load_selected_diff();
                        }
                        GitPane::Diff => {
                            self.diff_scroll = self.diff_scroll.saturating_sub(1);
                        }
                    }
                }
            }
            ActiveTab::Plans => match self.plans_pane {
                PlansPane::List => {
                    if self.plan_file_index > 0 {
                        self.plan_file_index -= 1;
                        self.plan_content_scroll = 0;
                    }
                }
                PlansPane::Content => {
                    self.plan_content_scroll = self.plan_content_scroll.saturating_sub(1);
                }
            },
            ActiveTab::GitHubPRs => match self.gh_pane {
                GitHubPane::List => {
                    self.gh_skip_to_prev_pr();
                }
                GitHubPane::Detail => {
                    self.gh_detail_scroll = self.gh_detail_scroll.saturating_sub(1);
                }
            },
            ActiveTab::GitHubIssues => match self.gh_issues_pane {
                IssuesPane::List => {
                    self.issues_skip_to_prev();
                }
                IssuesPane::Detail => {
                    self.gh_issues_detail_scroll = self.gh_issues_detail_scroll.saturating_sub(1);
                }
            },
            ActiveTab::Jira => match self.jira_pane {
                JiraPane::List => {
                    self.jira_skip_to_prev_issue();
                }
                JiraPane::Detail => {
                    self.jira_detail_scroll = self.jira_detail_scroll.saturating_sub(1);
                }
            },
            ActiveTab::Linear => match self.linear_pane {
                LinearPane::List => {
                    self.linear_skip_to_prev_issue();
                }
                LinearPane::Detail => {
                    self.linear_detail_scroll = self.linear_detail_scroll.saturating_sub(1);
                }
            },
            ActiveTab::Processes => match self.processes_pane {
                ProcessesPane::List => {
                    self.process_index = self.process_index.saturating_sub(1);
                    self.process_output_scroll = 0;
                }
                ProcessesPane::Output => {
                    self.process_output_scroll = self.process_output_scroll.saturating_sub(1);
                }
            },
        }
    }

    pub fn navigate_left(&mut self) {
        match self.active_tab {
            ActiveTab::Sessions => {
                self.sessions_pane = SessionsPane::List;
            }
            ActiveTab::Teams => {
                self.teams_pane = match self.teams_pane {
                    TeamsPane::Detail => TeamsPane::Tasks,
                    TeamsPane::Tasks => TeamsPane::Members,
                    TeamsPane::Members => TeamsPane::Teams,
                    TeamsPane::Teams => TeamsPane::Teams,
                };
            }
            ActiveTab::Todos => {
                self.todos_pane_left = true;
            }
            ActiveTab::Git => {
                if self.git_mode == GitMode::Browse {
                    self.fb_pane = FileBrowserPane::Tree;
                } else {
                    self.git_pane = GitPane::Files;
                }
            }
            ActiveTab::Plans => {
                self.plans_pane = PlansPane::List;
            }
            ActiveTab::GitHubPRs => {
                self.gh_pane = GitHubPane::List;
            }
            ActiveTab::GitHubIssues => {
                self.gh_issues_pane = IssuesPane::List;
            }
            ActiveTab::Jira => {
                self.jira_pane = JiraPane::List;
            }
            ActiveTab::Linear => {
                self.linear_pane = LinearPane::List;
            }
            ActiveTab::Processes => {
                self.processes_pane = ProcessesPane::List;
            }
        }
    }

    pub fn navigate_right(&mut self) {
        match self.active_tab {
            ActiveTab::Sessions => {
                self.sessions_pane = SessionsPane::Transcript;
            }
            ActiveTab::Teams => {
                self.teams_pane = match self.teams_pane {
                    TeamsPane::Teams => TeamsPane::Members,
                    TeamsPane::Members => TeamsPane::Tasks,
                    TeamsPane::Tasks => TeamsPane::Detail,
                    TeamsPane::Detail => TeamsPane::Detail,
                };
                self.detail_scroll = 0;
                // Load inbox when entering member detail
                if self.teams_pane == TeamsPane::Detail {
                    self.load_inbox_for_selected_member();
                }
            }
            ActiveTab::Todos => {
                self.todos_pane_left = false;
            }
            ActiveTab::Git => {
                if self.git_mode == GitMode::Browse {
                    self.fb_pane = FileBrowserPane::Content;
                } else {
                    self.git_pane = GitPane::Diff;
                }
            }
            ActiveTab::Plans => {
                self.plans_pane = PlansPane::Content;
            }
            ActiveTab::GitHubPRs => {
                self.gh_pane = GitHubPane::Detail;
            }
            ActiveTab::GitHubIssues => {
                self.gh_issues_pane = IssuesPane::Detail;
            }
            ActiveTab::Jira => {
                self.jira_pane = JiraPane::Detail;
            }
            ActiveTab::Linear => {
                self.linear_pane = LinearPane::Detail;
            }
            ActiveTab::Processes => {
                self.processes_pane = ProcessesPane::Output;
            }
        }
    }

    pub fn select_item(&mut self) {
        match self.active_tab {
            ActiveTab::Sessions => {
                if self.sessions_pane == SessionsPane::List {
                    // Force reload of selected transcript
                    self.loaded_session_id = None;
                    self.load_selected_transcript();
                    self.sessions_pane = SessionsPane::Transcript;
                }
            }
            ActiveTab::Git => {
                if self.git_mode == GitMode::Browse {
                    self.fb_select_item();
                } else if self.git_pane == GitPane::Files {
                    self.load_selected_diff();
                    self.git_pane = GitPane::Diff;
                }
            }
            ActiveTab::Plans => {
                if self.plans_pane == PlansPane::List {
                    self.plans_pane = PlansPane::Content;
                }
            }
            ActiveTab::GitHubPRs => {
                if self.gh_pane == GitHubPane::List {
                    self.gh_pane = GitHubPane::Detail;
                }
            }
            ActiveTab::GitHubIssues => {
                if self.gh_issues_pane == IssuesPane::List {
                    self.gh_issues_pane = IssuesPane::Detail;
                }
            }
            ActiveTab::Jira => {
                if self.jira_pane == JiraPane::List {
                    self.jira_load_detail();
                    self.jira_pane = JiraPane::Detail;
                }
            }
            ActiveTab::Linear => {
                if self.linear_pane == LinearPane::List {
                    self.linear_open_selected();
                }
            }
            ActiveTab::Processes => {
                if self.processes_pane == ProcessesPane::List {
                    self.processes_pane = ProcessesPane::Output;
                }
            }
            _ => {}
        }
    }

    pub fn jump_top(&mut self) {
        match self.active_tab {
            ActiveTab::Sessions => match self.sessions_pane {
                SessionsPane::List => self.session_list_index = 0,
                SessionsPane::Transcript => {
                    self.follow_mode = false;
                    self.transcript_scroll = 0;
                }
            },
            ActiveTab::Teams => match self.teams_pane {
                TeamsPane::Teams => {
                    self.team_list_index = 0;
                    self.member_list_index = 0;
                    self.task_list_index = 0;
                    self.detail_scroll = 0;
                    self.load_tasks_for_selected_team();
                    self.compute_agent_statuses();
                }
                TeamsPane::Members => {
                    self.member_list_index = 0;
                    self.detail_scroll = 0;
                    self.load_inbox_for_selected_member();
                }
                TeamsPane::Tasks => {
                    self.task_list_index = 0;
                    self.detail_scroll = 0;
                }
                TeamsPane::Detail => self.detail_scroll = 0,
            },
            ActiveTab::Todos => {
                if self.todos_pane_left {
                    self.todo_file_index = 0;
                    self.todo_item_index = 0;
                } else {
                    self.todo_item_index = 0;
                }
            }
            ActiveTab::Git => {
                if self.git_mode == GitMode::Browse {
                    self.fb_index = 0;
                    self.fb_content_scroll = 0;
                } else {
                    match self.git_pane {
                        GitPane::Files => {
                            self.git_file_index = 0;
                            self.skip_to_file_entry();
                            self.load_selected_diff();
                        }
                        GitPane::Diff => {
                            self.diff_scroll = 0;
                        }
                    }
                }
            }
            ActiveTab::Plans => match self.plans_pane {
                PlansPane::List => {
                    self.plan_file_index = 0;
                    self.plan_content_scroll = 0;
                }
                PlansPane::Content => {
                    self.plan_content_scroll = 0;
                }
            },
            ActiveTab::GitHubPRs => match self.gh_pane {
                GitHubPane::List => {
                    self.gh_pr_index = 0;
                    self.gh_skip_to_pr_entry();
                }
                GitHubPane::Detail => {
                    self.gh_detail_scroll = 0;
                }
            },
            ActiveTab::GitHubIssues => match self.gh_issues_pane {
                IssuesPane::List => {
                    self.gh_issues_index = 0;
                    self.issues_skip_to_entry();
                }
                IssuesPane::Detail => {
                    self.gh_issues_detail_scroll = 0;
                }
            },
            ActiveTab::Jira => match self.jira_pane {
                JiraPane::List => {
                    self.jira_index = 0;
                    self.jira_skip_to_issue_entry();
                }
                JiraPane::Detail => {
                    self.jira_detail_scroll = 0;
                }
            },
            ActiveTab::Linear => match self.linear_pane {
                LinearPane::List => {
                    self.linear_index = 0;
                    self.linear_skip_to_issue_entry();
                }
                LinearPane::Detail => {
                    self.linear_detail_scroll = 0;
                }
            },
            ActiveTab::Processes => match self.processes_pane {
                ProcessesPane::List => {
                    self.process_index = 0;
                    self.process_output_scroll = 0;
                }
                ProcessesPane::Output => {
                    self.process_output_scroll = 0;
                }
            },
        }
    }

    pub fn jump_bottom(&mut self) {
        match self.active_tab {
            ActiveTab::Sessions => match self.sessions_pane {
                SessionsPane::List => {
                    if !self.sessions.is_empty() {
                        self.session_list_index = self.sessions.len() - 1;
                    }
                }
                SessionsPane::Transcript => {
                    self.follow_mode = true;
                    self.transcript_scroll = self.transcript_items.len();
                }
            },
            ActiveTab::Teams => match self.teams_pane {
                TeamsPane::Teams => {
                    if !self.teams.is_empty() {
                        self.team_list_index = self.teams.len() - 1;
                        self.member_list_index = 0;
                        self.task_list_index = 0;
                        self.detail_scroll = 0;
                        self.load_tasks_for_selected_team();
                        self.compute_agent_statuses();
                    }
                }
                TeamsPane::Members => {
                    let members = self.current_team_members();
                    if !members.is_empty() {
                        self.member_list_index = members.len() - 1;
                        self.detail_scroll = 0;
                        self.load_inbox_for_selected_member();
                    }
                }
                TeamsPane::Tasks => {
                    if !self.tasks.is_empty() {
                        self.task_list_index = self.tasks.len() - 1;
                        self.detail_scroll = 0;
                    }
                }
                TeamsPane::Detail => {
                    self.detail_scroll = usize::MAX;
                }
            },
            ActiveTab::Todos => {
                if self.todos_pane_left {
                    if !self.todo_files.is_empty() {
                        self.todo_file_index = self.todo_files.len() - 1;
                        self.todo_item_index = 0;
                    }
                } else {
                    let items = self.current_todo_items();
                    if !items.is_empty() {
                        self.todo_item_index = items.len() - 1;
                    }
                }
            }
            ActiveTab::Git => {
                if self.git_mode == GitMode::Browse {
                    if !self.fb_entries.is_empty() {
                        self.fb_index = self.fb_entries.len() - 1;
                    }
                    self.fb_content_scroll = usize::MAX;
                } else {
                    match self.git_pane {
                        GitPane::Files => {
                            if !self.git_flat_list.is_empty() {
                                self.git_file_index = self.git_flat_list.len() - 1;
                                while self.git_file_index > 0
                                    && !self.git_flat_list[self.git_file_index].is_file()
                                {
                                    self.git_file_index -= 1;
                                }
                                self.load_selected_diff();
                            }
                        }
                        GitPane::Diff => {
                            self.diff_scroll = usize::MAX;
                        }
                    }
                }
            }
            ActiveTab::Plans => match self.plans_pane {
                PlansPane::List => {
                    if !self.plan_files.is_empty() {
                        self.plan_file_index = self.plan_files.len() - 1;
                        self.plan_content_scroll = 0;
                    }
                }
                PlansPane::Content => {
                    self.plan_content_scroll = usize::MAX;
                }
            },
            ActiveTab::GitHubPRs => match self.gh_pane {
                GitHubPane::List => {
                    if !self.gh_flat_list.is_empty() {
                        self.gh_pr_index = self.gh_flat_list.len() - 1;
                        // Walk backward to find last PR entry
                        while self.gh_pr_index > 0
                            && matches!(
                                self.gh_flat_list[self.gh_pr_index],
                                FlatPrItem::SectionHeader(_)
                            )
                        {
                            self.gh_pr_index -= 1;
                        }
                    }
                }
                GitHubPane::Detail => {
                    self.gh_detail_scroll = usize::MAX;
                }
            },
            ActiveTab::GitHubIssues => match self.gh_issues_pane {
                IssuesPane::List => {
                    if !self.gh_issues_flat_list.is_empty() {
                        self.gh_issues_index = self.gh_issues_flat_list.len() - 1;
                        while self.gh_issues_index > 0
                            && matches!(
                                self.gh_issues_flat_list[self.gh_issues_index],
                                FlatIssueItem::SectionHeader(_)
                            )
                        {
                            self.gh_issues_index -= 1;
                        }
                    }
                }
                IssuesPane::Detail => {
                    self.gh_issues_detail_scroll = usize::MAX;
                }
            },
            ActiveTab::Jira => match self.jira_pane {
                JiraPane::List => {
                    if !self.jira_flat_list.is_empty() {
                        self.jira_index = self.jira_flat_list.len() - 1;
                        while self.jira_index > 0
                            && matches!(
                                self.jira_flat_list[self.jira_index],
                                FlatJiraItem::StatusHeader(_, _)
                            )
                        {
                            self.jira_index -= 1;
                        }
                    }
                }
                JiraPane::Detail => {
                    self.jira_detail_scroll = usize::MAX;
                }
            },
            ActiveTab::Linear => match self.linear_pane {
                LinearPane::List => {
                    if !self.linear_flat_list.is_empty() {
                        self.linear_index = self.linear_flat_list.len() - 1;
                        while self.linear_index > 0
                            && matches!(
                                self.linear_flat_list[self.linear_index],
                                FlatLinearItem::AssignmentHeader(_)
                            )
                        {
                            self.linear_index -= 1;
                        }
                    }
                }
                LinearPane::Detail => {
                    self.linear_detail_scroll = usize::MAX;
                }
            },
            ActiveTab::Processes => match self.processes_pane {
                ProcessesPane::List => {
                    if !self.processes.is_empty() {
                        self.process_index = self.processes.len() - 1;
                        self.process_output_scroll = 0;
                    }
                }
                ProcessesPane::Output => {
                    self.process_output_scroll = usize::MAX;
                }
            },
        }
    }

    pub fn toggle_follow(&mut self) {
        self.follow_mode = !self.follow_mode;
        if self.follow_mode {
            self.transcript_scroll = self.transcript_items.len();
        }
    }

    // --- Git helpers ---

    pub fn load_git_data(&mut self) {
        match git::load_git_status(&self.project_cwd) {
            Ok(status) => {
                self.git_status = status;
                self.git_flat_list = self.git_status.flat_list();
                // Clamp index
                if self.git_flat_list.is_empty() {
                    self.git_file_index = 0;
                } else if self.git_file_index >= self.git_flat_list.len() {
                    self.git_file_index = self.git_flat_list.len() - 1;
                }
                self.skip_to_file_entry();
                self.load_selected_diff();
            }
            Err(e) => {
                self.last_error = Some(format!("Git: {}", e));
            }
        }
    }

    pub fn load_selected_diff(&mut self) {
        self.diff_scroll = 0;
        if self.git_flat_list.is_empty() {
            self.git_diff_lines.clear();
            return;
        }
        let idx = self.git_file_index.min(self.git_flat_list.len() - 1);
        if let FlatGitItem::File(ref entry) = self.git_flat_list[idx] {
            match git::load_diff(&self.project_cwd, entry) {
                Ok(lines) => self.git_diff_lines = lines,
                Err(e) => {
                    self.last_error = Some(format!("Diff: {}", e));
                    self.git_diff_lines.clear();
                }
            }
        }
    }

    /// Advance git_file_index forward past section headers to the next file entry.
    fn skip_to_file_entry(&mut self) {
        if self.git_flat_list.is_empty() {
            return;
        }
        let idx = self.git_file_index.min(self.git_flat_list.len() - 1);
        if !self.git_flat_list[idx].is_file() {
            // Scan forward
            for i in (idx + 1)..self.git_flat_list.len() {
                if self.git_flat_list[i].is_file() {
                    self.git_file_index = i;
                    return;
                }
            }
        }
    }

    fn skip_to_next_file(&mut self) {
        if self.git_flat_list.is_empty() {
            return;
        }
        let start = self.git_file_index + 1;
        for i in start..self.git_flat_list.len() {
            if self.git_flat_list[i].is_file() {
                self.git_file_index = i;
                return;
            }
        }
    }

    fn skip_to_prev_file(&mut self) {
        if self.git_file_index == 0 || self.git_flat_list.is_empty() {
            return;
        }
        let start = self.git_file_index - 1;
        for i in (0..=start).rev() {
            if self.git_flat_list[i].is_file() {
                self.git_file_index = i;
                return;
            }
        }
    }

    // --- Data access helpers ---

    pub fn current_team_members(&self) -> Vec<TeamMember> {
        if self.teams.is_empty() {
            return Vec::new();
        }
        let idx = self.team_list_index.min(self.teams.len() - 1);
        self.teams[idx].config.members.clone()
    }

    pub fn current_todo_items(&self) -> Vec<TodoItem> {
        if self.todo_files.is_empty() {
            return Vec::new();
        }
        let idx = self.todo_file_index.min(self.todo_files.len() - 1);
        self.todo_files[idx].items.clone()
    }

    // --- File browser helpers ---

    pub fn toggle_git_mode(&mut self) {
        self.git_mode = match self.git_mode {
            GitMode::Status => {
                self.load_file_tree();
                GitMode::Browse
            }
            GitMode::Browse => {
                self.fb_editing = false;
                self.fb_editor = None;
                GitMode::Status
            }
        };
    }

    pub fn load_file_tree(&mut self) {
        match filebrowser::build_tree(&self.project_cwd, &self.fb_expanded) {
            Ok(entries) => {
                self.fb_entries = entries;
                if self.fb_index >= self.fb_entries.len() {
                    self.fb_index = 0;
                }
            }
            Err(e) => {
                self.last_error = Some(format!("File browser: {}", e));
            }
        }
    }

    fn fb_navigate_down(&mut self) {
        match self.fb_pane {
            FileBrowserPane::Tree => {
                if !self.fb_entries.is_empty() {
                    self.fb_index = (self.fb_index + 1).min(self.fb_entries.len() - 1);
                }
            }
            FileBrowserPane::Content => {
                self.fb_content_scroll = self.fb_content_scroll.saturating_add(1);
            }
        }
    }

    fn fb_navigate_up(&mut self) {
        match self.fb_pane {
            FileBrowserPane::Tree => {
                self.fb_index = self.fb_index.saturating_sub(1);
            }
            FileBrowserPane::Content => {
                self.fb_content_scroll = self.fb_content_scroll.saturating_sub(1);
            }
        }
    }

    fn fb_select_item(&mut self) {
        if self.fb_entries.is_empty() {
            return;
        }
        let idx = self.fb_index.min(self.fb_entries.len() - 1);
        let entry = self.fb_entries[idx].clone();

        use crate::model::filebrowser::EntryKind;
        match entry.kind {
            EntryKind::Directory => {
                // Toggle expand/collapse
                if self.fb_expanded.contains(&entry.path) {
                    self.fb_expanded.remove(&entry.path);
                } else {
                    self.fb_expanded.insert(entry.path);
                }
                self.load_file_tree();
            }
            EntryKind::File => {
                // Load file content
                match filebrowser::read_file_content(&entry.path) {
                    Ok(content) => {
                        self.fb_content = Some(content);
                        self.fb_content_path = Some(entry.path);
                        self.fb_content_scroll = 0;
                        self.fb_pane = FileBrowserPane::Content;
                    }
                    Err(e) => {
                        self.last_error = Some(format!("Read file: {}", e));
                    }
                }
            }
        }
    }

    pub fn fb_backspace(&mut self) {
        if self.fb_entries.is_empty() {
            return;
        }
        let idx = self.fb_index.min(self.fb_entries.len() - 1);
        let entry = &self.fb_entries[idx];

        use crate::model::filebrowser::EntryKind;
        if entry.kind == EntryKind::Directory && self.fb_expanded.contains(&entry.path) {
            self.fb_expanded.remove(&entry.path);
            self.load_file_tree();
        } else if entry.depth > 0 {
            // Go to parent directory
            if let Some(parent) = entry.path.parent() {
                for (i, e) in self.fb_entries.iter().enumerate() {
                    if e.path == parent {
                        self.fb_index = i;
                        break;
                    }
                }
            }
        }
    }

    pub fn fb_start_edit(&mut self) {
        if self.fb_pane != FileBrowserPane::Content {
            return;
        }
        if let Some(FileContent::Text(ref lines)) = self.fb_content {
            let text = lines.join("\n");
            let mut editor = tui_textarea::TextArea::default();
            editor.insert_str(&text);
            // Move cursor to beginning
            editor.move_cursor(tui_textarea::CursorMove::Top);
            editor.move_cursor(tui_textarea::CursorMove::Head);
            self.fb_editor = Some(editor);
            self.fb_editing = true;
        } else if let Some(FileContent::Markdown(_)) = self.fb_content {
            // Read raw text for editing
            if let Some(ref path) = self.fb_content_path {
                if let Ok(text) = std::fs::read_to_string(path) {
                    let mut editor = tui_textarea::TextArea::default();
                    editor.insert_str(&text);
                    editor.move_cursor(tui_textarea::CursorMove::Top);
                    editor.move_cursor(tui_textarea::CursorMove::Head);
                    self.fb_editor = Some(editor);
                    self.fb_editing = true;
                }
            }
        }
    }

    pub fn fb_save_edit(&mut self) {
        if let (Some(ref editor), Some(ref path)) = (&self.fb_editor, &self.fb_content_path) {
            let content = editor.lines().join("\n");
            if let Err(e) = filebrowser::save_file(path, &content) {
                self.last_error = Some(format!("Save: {}", e));
                return;
            }
            // Reload content
            let path = path.clone();
            self.fb_editing = false;
            self.fb_editor = None;
            match filebrowser::read_file_content(&path) {
                Ok(c) => self.fb_content = Some(c),
                Err(e) => self.last_error = Some(format!("Reload: {}", e)),
            }
        }
    }

    pub fn fb_cancel_edit(&mut self) {
        self.fb_editing = false;
        self.fb_editor = None;
    }

    // --- Delete helpers ---

    /// Show the delete confirmation dialog for the currently selected item.
    pub fn request_delete(&mut self) {
        let name = match self.active_tab {
            ActiveTab::Todos => {
                if !self.todos_pane_left || self.todo_files.is_empty() {
                    return;
                }
                let idx = self.todo_file_index.min(self.todo_files.len() - 1);
                self.todo_files[idx].filename.clone()
            }
            ActiveTab::Plans => {
                if self.plans_pane != PlansPane::List || self.plan_files.is_empty() {
                    return;
                }
                let idx = self.plan_file_index.min(self.plan_files.len() - 1);
                self.plan_files[idx].filename.clone()
            }
            ActiveTab::Sessions => {
                if self.sessions_pane != SessionsPane::List || self.sessions.is_empty() {
                    return;
                }
                let idx = self.session_list_index.min(self.sessions.len() - 1);
                let session = &self.sessions[idx];
                format!("{}.jsonl", session.session_id)
            }
            ActiveTab::Teams => {
                if self.teams_pane != TeamsPane::Teams || self.teams.is_empty() {
                    return;
                }
                let idx = self.team_list_index.min(self.teams.len() - 1);
                self.teams[idx].display_name().to_string()
            }
            _ => return,
        };
        self.delete_target_name = name;
        self.confirm_delete = true;
    }

    /// Execute the delete after confirmation.
    pub fn execute_delete(&mut self) {
        self.confirm_delete = false;
        match self.active_tab {
            ActiveTab::Todos => self.delete_selected_todo(),
            ActiveTab::Plans => self.delete_selected_plan(),
            ActiveTab::Sessions => self.delete_selected_session(),
            ActiveTab::Teams => self.delete_selected_team(),
            _ => {}
        }
    }

    /// Cancel the delete confirmation.
    pub fn cancel_delete(&mut self) {
        self.confirm_delete = false;
        self.delete_target_name.clear();
    }

    fn delete_selected_todo(&mut self) {
        if self.todo_files.is_empty() {
            return;
        }
        let idx = self.todo_file_index.min(self.todo_files.len() - 1);
        let filename = &self.todo_files[idx].filename;
        let path = self.claude_home.join("todos").join(filename);
        if let Err(e) = std::fs::remove_file(&path) {
            self.last_error = Some(format!("Delete todo: {}", e));
            return;
        }
        self.load_todos();
        if self.todo_file_index > 0 && self.todo_file_index >= self.todo_files.len() {
            self.todo_file_index = self.todo_files.len().saturating_sub(1);
        }
        self.todo_item_index = 0;
    }

    fn delete_selected_plan(&mut self) {
        if self.plan_files.is_empty() {
            return;
        }
        let idx = self.plan_file_index.min(self.plan_files.len() - 1);
        let filename = &self.plan_files[idx].filename;
        let path = self.claude_home.join("plans").join(filename);
        if let Err(e) = std::fs::remove_file(&path) {
            self.last_error = Some(format!("Delete plan: {}", e));
            return;
        }
        self.load_plans();
        if self.plan_file_index > 0 && self.plan_file_index >= self.plan_files.len() {
            self.plan_file_index = self.plan_files.len().saturating_sub(1);
        }
        self.plan_content_scroll = 0;
    }

    fn delete_selected_session(&mut self) {
        if self.sessions.is_empty() {
            return;
        }
        let idx = self.session_list_index.min(self.sessions.len() - 1);
        let session_id = self.sessions[idx].session_id.clone();
        let project_dir = self
            .claude_home
            .join("projects")
            .join(&self.encoded_project);
        let path = project_dir.join(format!("{}.jsonl", session_id));
        if let Err(e) = std::fs::remove_file(&path) {
            self.last_error = Some(format!("Delete session: {}", e));
            return;
        }
        // Clear loaded transcript if it was the deleted session
        if self.loaded_session_id.as_deref() == Some(&session_id) {
            self.loaded_session_id = None;
            self.transcript_items.clear();
            self.transcript_scroll = 0;
            self.subagents.clear();
            self.subagent_transcript.clear();
            self.viewing_subagent = false;
        }
        self.load_sessions();
        if self.session_list_index > 0 && self.session_list_index >= self.sessions.len() {
            self.session_list_index = self.sessions.len().saturating_sub(1);
        }
    }

    fn delete_selected_team(&mut self) {
        if self.teams.is_empty() {
            return;
        }
        let idx = self.team_list_index.min(self.teams.len() - 1);
        let dir_name = self.teams[idx].dir_name.clone();
        let team_dir = self.claude_home.join("teams").join(&dir_name);
        if let Err(e) = std::fs::remove_dir_all(&team_dir) {
            self.last_error = Some(format!("Delete team: {}", e));
            return;
        }
        self.load_teams();
        if self.team_list_index > 0 && self.team_list_index >= self.teams.len() {
            self.team_list_index = self.teams.len().saturating_sub(1);
        }
        self.member_list_index = 0;
        self.task_list_index = 0;
        self.detail_scroll = 0;
        self.load_tasks_for_selected_team();
        self.compute_agent_statuses();
    }

    // --- GitHub PR helpers ---

    pub fn load_github_prs(&mut self) {
        if let (Some(ref repo), Some(ref user)) = (&self.gh_repo, &self.gh_user) {
            match github::list_open_prs(repo) {
                Ok(prs) => {
                    // Check for new activity
                    for pr in &prs {
                        if let Some(prev) = self.gh_prev_updated.get(&pr.number) {
                            if *prev != pr.updated_at {
                                self.gh_new_activity = true;
                            }
                        } else {
                            // New PR appeared
                            if !self.gh_prev_updated.is_empty() {
                                self.gh_new_activity = true;
                            }
                        }
                    }
                    // Update prev timestamps
                    self.gh_prev_updated.clear();
                    for pr in &prs {
                        self.gh_prev_updated
                            .insert(pr.number, pr.updated_at.clone());
                    }

                    self.gh_flat_list = github::categorize_prs(&prs, user);
                    self.gh_prs = prs;
                    if self.gh_pr_index >= self.gh_flat_list.len() {
                        self.gh_pr_index = 0;
                        self.gh_skip_to_pr_entry();
                    }
                    self.gh_last_poll = Instant::now();
                }
                Err(e) => {
                    self.last_error = Some(format!("GitHub: {}", e));
                }
            }
        }
    }

    fn gh_skip_to_next_pr(&mut self) {
        if self.gh_flat_list.is_empty() {
            return;
        }
        let start = self.gh_pr_index + 1;
        for i in start..self.gh_flat_list.len() {
            if matches!(self.gh_flat_list[i], FlatPrItem::Pr(_)) {
                self.gh_pr_index = i;
                return;
            }
        }
    }

    fn gh_skip_to_prev_pr(&mut self) {
        if self.gh_pr_index == 0 || self.gh_flat_list.is_empty() {
            return;
        }
        for i in (0..self.gh_pr_index).rev() {
            if matches!(self.gh_flat_list[i], FlatPrItem::Pr(_)) {
                self.gh_pr_index = i;
                return;
            }
        }
    }

    fn gh_skip_to_pr_entry(&mut self) {
        if self.gh_flat_list.is_empty() {
            return;
        }
        let idx = self.gh_pr_index.min(self.gh_flat_list.len() - 1);
        if matches!(self.gh_flat_list[idx], FlatPrItem::SectionHeader(_)) {
            for i in (idx + 1)..self.gh_flat_list.len() {
                if matches!(self.gh_flat_list[i], FlatPrItem::Pr(_)) {
                    self.gh_pr_index = i;
                    return;
                }
            }
        }
    }

    pub fn gh_selected_pr(&self) -> Option<&PullRequest> {
        if self.gh_flat_list.is_empty() {
            return None;
        }
        let idx = self.gh_pr_index.min(self.gh_flat_list.len() - 1);
        match &self.gh_flat_list[idx] {
            FlatPrItem::Pr(pr) => Some(pr),
            _ => None,
        }
    }

    pub fn gh_open_selected(&self) {
        if let Some(pr) = self.gh_selected_pr() {
            cli_detect::open_url(&pr.url);
        }
    }

    // --- GitHub Issues helpers ---

    pub fn load_github_issues(&mut self) {
        if !self.gh_issues_enabled {
            return;
        }
        if let (Some(ref repo), Some(ref user)) = (&self.gh_issues_repo, &self.gh_user) {
            let state = self.project_config.github_issues_state().to_string();
            match github::list_issues(repo, &state) {
                Ok(issues) => {
                    self.gh_issues_flat_list = github::categorize_issues(&issues, user);
                    self.gh_issues = issues;
                    if self.gh_issues_index >= self.gh_issues_flat_list.len() {
                        self.gh_issues_index = 0;
                        self.issues_skip_to_entry();
                    }
                    self.gh_issues_last_poll = Instant::now();
                }
                Err(e) => {
                    self.last_error = Some(format!("Issues: {}", e));
                }
            }
        }
    }

    fn issues_skip_to_next(&mut self) {
        if self.gh_issues_flat_list.is_empty() {
            return;
        }
        let start = self.gh_issues_index + 1;
        for i in start..self.gh_issues_flat_list.len() {
            if matches!(self.gh_issues_flat_list[i], FlatIssueItem::Issue(_)) {
                self.gh_issues_index = i;
                return;
            }
        }
    }

    fn issues_skip_to_prev(&mut self) {
        if self.gh_issues_index == 0 || self.gh_issues_flat_list.is_empty() {
            return;
        }
        for i in (0..self.gh_issues_index).rev() {
            if matches!(self.gh_issues_flat_list[i], FlatIssueItem::Issue(_)) {
                self.gh_issues_index = i;
                return;
            }
        }
    }

    fn issues_skip_to_entry(&mut self) {
        if self.gh_issues_flat_list.is_empty() {
            return;
        }
        let idx = self.gh_issues_index.min(self.gh_issues_flat_list.len() - 1);
        if matches!(
            self.gh_issues_flat_list[idx],
            FlatIssueItem::SectionHeader(_)
        ) {
            for i in (idx + 1)..self.gh_issues_flat_list.len() {
                if matches!(self.gh_issues_flat_list[i], FlatIssueItem::Issue(_)) {
                    self.gh_issues_index = i;
                    return;
                }
            }
        }
    }

    pub fn issues_selected(&self) -> Option<&GitHubIssue> {
        if self.gh_issues_flat_list.is_empty() {
            return None;
        }
        let idx = self.gh_issues_index.min(self.gh_issues_flat_list.len() - 1);
        match &self.gh_issues_flat_list[idx] {
            FlatIssueItem::Issue(issue) => Some(issue),
            _ => None,
        }
    }

    pub fn issues_open_in_browser(&self) {
        if let Some(issue) = self.issues_selected() {
            cli_detect::open_url(&issue.url);
        }
    }

    pub fn issues_start_create(&mut self) {
        let mut title_ed = tui_textarea::TextArea::default();
        title_ed.set_cursor_line_style(ratatui::style::Style::default());
        let mut body_ed = tui_textarea::TextArea::default();
        body_ed.set_cursor_line_style(ratatui::style::Style::default());
        self.gh_issues_title_editor = Some(title_ed);
        self.gh_issues_body_editor = Some(body_ed);
        self.gh_issues_edit_mode = Some(IssueEditMode::Create);
        self.gh_issues_edit_field = IssueEditField::Title;
        self.gh_issues_editing = true;
    }

    pub fn issues_start_edit(&mut self) {
        if let Some(issue) = self.issues_selected().cloned() {
            let mut title_ed = tui_textarea::TextArea::default();
            title_ed.set_cursor_line_style(ratatui::style::Style::default());
            title_ed.insert_str(&issue.title);
            title_ed.move_cursor(tui_textarea::CursorMove::Head);

            let mut body_ed = tui_textarea::TextArea::default();
            body_ed.set_cursor_line_style(ratatui::style::Style::default());
            if let Some(ref body) = issue.body {
                body_ed.insert_str(body);
                body_ed.move_cursor(tui_textarea::CursorMove::Top);
                body_ed.move_cursor(tui_textarea::CursorMove::Head);
            }

            self.gh_issues_title_editor = Some(title_ed);
            self.gh_issues_body_editor = Some(body_ed);
            self.gh_issues_edit_mode = Some(IssueEditMode::Edit(issue.number));
            self.gh_issues_edit_field = IssueEditField::Title;
            self.gh_issues_editing = true;
        }
    }

    pub fn issues_start_comment(&mut self) {
        if let Some(issue) = self.issues_selected().cloned() {
            let title_ed = tui_textarea::TextArea::default();
            let mut body_ed = tui_textarea::TextArea::default();
            body_ed.set_cursor_line_style(ratatui::style::Style::default());
            self.gh_issues_title_editor = Some(title_ed);
            self.gh_issues_body_editor = Some(body_ed);
            self.gh_issues_edit_mode = Some(IssueEditMode::Comment(issue.number));
            self.gh_issues_edit_field = IssueEditField::Body;
            self.gh_issues_editing = true;
        }
    }

    pub fn issues_save_edit(&mut self) {
        let Some(ref mode) = self.gh_issues_edit_mode.clone() else {
            return;
        };
        let repo = match &self.gh_issues_repo {
            Some(r) => r.clone(),
            None => return,
        };
        let title = self
            .gh_issues_title_editor
            .as_ref()
            .map(|e| e.lines().join(""))
            .unwrap_or_default();
        let body = self
            .gh_issues_body_editor
            .as_ref()
            .map(|e| e.lines().join("\n"))
            .unwrap_or_default();

        let result = match mode {
            IssueEditMode::Create => {
                if title.trim().is_empty() {
                    self.last_error = Some("Title cannot be empty".to_string());
                    return;
                }
                github::create_issue(&repo, &title, &body)
            }
            IssueEditMode::Edit(number) => github::edit_issue(&repo, *number, &title, &body),
            IssueEditMode::Comment(number) => {
                if body.trim().is_empty() {
                    self.last_error = Some("Comment cannot be empty".to_string());
                    return;
                }
                github::comment_issue(&repo, *number, &body)
            }
        };

        match result {
            Ok(()) => {
                self.issues_cancel_edit();
                self.load_github_issues();
            }
            Err(e) => {
                self.last_error = Some(format!("Issue save: {}", e));
            }
        }
    }

    pub fn issues_cancel_edit(&mut self) {
        self.gh_issues_editing = false;
        self.gh_issues_edit_mode = None;
        self.gh_issues_title_editor = None;
        self.gh_issues_body_editor = None;
    }

    pub fn issues_toggle_state(&mut self) {
        let Some(issue) = self.issues_selected().cloned() else {
            return;
        };
        let Some(ref repo) = self.gh_issues_repo.clone() else {
            return;
        };
        let result = if issue.state == "OPEN" {
            github::close_issue(repo, issue.number)
        } else {
            github::reopen_issue(repo, issue.number)
        };
        match result {
            Ok(()) => self.load_github_issues(),
            Err(e) => self.last_error = Some(format!("Issue state: {}", e)),
        }
    }

    // --- Jira helpers ---

    pub fn load_jira_issues(&mut self) {
        if !self.has_jira {
            return;
        }
        match jira::search_my_issues(
            self.project_config.jira_project(),
            self.project_config.jira_jql(),
        ) {
            Ok(issues) => {
                self.jira_flat_list = jira::categorize_issues(&issues);
                self.jira_issues = issues;
                if self.jira_index >= self.jira_flat_list.len() {
                    self.jira_index = 0;
                    self.jira_skip_to_issue_entry();
                }
                self.jira_last_poll = Instant::now();
            }
            Err(e) => {
                self.last_error = Some(format!("Jira: {}", e));
            }
        }
    }

    pub fn jira_search(&mut self) {
        let query = self.jira_search_input.trim().to_string();
        if query.is_empty() {
            return;
        }
        self.jira_search_mode = false;
        match jira::search_issues(&query) {
            Ok(issues) => {
                self.jira_flat_list = jira::categorize_issues(&issues);
                self.jira_issues = issues;
                self.jira_index = 0;
                self.jira_skip_to_issue_entry();
            }
            Err(e) => {
                self.last_error = Some(format!("Jira search: {}", e));
            }
        }
    }

    fn jira_load_detail(&mut self) {
        let issue = self.jira_selected_issue();
        if let Some(issue) = issue {
            let key = issue.key.clone();
            match jira::view_issue(&key) {
                Ok(detail) => {
                    self.jira_detail = Some(detail);
                    self.jira_detail_scroll = 0;
                }
                Err(e) => {
                    self.last_error = Some(format!("Jira detail: {}", e));
                }
            }
        }
    }

    pub fn jira_load_transitions(&mut self) {
        if let Some(issue) = self.jira_selected_issue().cloned() {
            let statuses = jira::get_status_options(&issue.status_name);
            self.jira_transitions = statuses
                .into_iter()
                .map(|name| JiraTransition { name })
                .collect();
            self.jira_show_transitions = true;
        }
    }

    pub fn jira_do_transition(&mut self, idx: usize) {
        if idx >= self.jira_transitions.len() {
            return;
        }
        let transition = self.jira_transitions[idx].clone();
        if let Some(issue) = self.jira_selected_issue().cloned() {
            match jira::transition_issue(&issue.key, &transition.name) {
                Ok(()) => {
                    self.jira_show_transitions = false;
                    self.load_jira_issues();
                }
                Err(e) => {
                    self.last_error = Some(format!("Transition: {}", e));
                }
            }
        }
    }

    fn jira_skip_to_next_issue(&mut self) {
        if self.jira_flat_list.is_empty() {
            return;
        }
        let start = self.jira_index + 1;
        for i in start..self.jira_flat_list.len() {
            if matches!(self.jira_flat_list[i], FlatJiraItem::Issue(_)) {
                self.jira_index = i;
                return;
            }
        }
    }

    fn jira_skip_to_prev_issue(&mut self) {
        if self.jira_index == 0 || self.jira_flat_list.is_empty() {
            return;
        }
        for i in (0..self.jira_index).rev() {
            if matches!(self.jira_flat_list[i], FlatJiraItem::Issue(_)) {
                self.jira_index = i;
                return;
            }
        }
    }

    fn jira_skip_to_issue_entry(&mut self) {
        if self.jira_flat_list.is_empty() {
            return;
        }
        let idx = self.jira_index.min(self.jira_flat_list.len() - 1);
        if matches!(self.jira_flat_list[idx], FlatJiraItem::StatusHeader(_, _)) {
            for i in (idx + 1)..self.jira_flat_list.len() {
                if matches!(self.jira_flat_list[i], FlatJiraItem::Issue(_)) {
                    self.jira_index = i;
                    return;
                }
            }
        }
    }

    pub fn jira_selected_issue(&self) -> Option<&JiraIssue> {
        if self.jira_flat_list.is_empty() {
            return None;
        }
        let idx = self.jira_index.min(self.jira_flat_list.len() - 1);
        match &self.jira_flat_list[idx] {
            FlatJiraItem::Issue(issue) => Some(issue),
            _ => None,
        }
    }

    pub fn jira_open_selected(&self) {
        if let Some(issue) = self.jira_selected_issue() {
            if !issue.url.is_empty() {
                cli_detect::open_url(&issue.url);
            }
        }
    }

    // --- Linear helpers ---

    pub fn load_linear_issues(&mut self) {
        if !self.has_linear {
            return;
        }
        let api_key = match self.project_config.linear_api_key() {
            Some(k) => k.to_string(),
            None => return,
        };
        let username = self.project_config.linear_username().map(|s| s.to_string());
        let team = self.project_config.linear_team().map(|s| s.to_string());

        self.linear_last_poll = Instant::now();
        match linear::fetch_my_issues(&api_key, username.as_deref(), team.as_deref()) {
            Ok(issues) => {
                self.linear_flat_list = linear::categorize_issues(&issues, username.as_deref());
                self.linear_issues = issues;
                if self.linear_index >= self.linear_flat_list.len() {
                    self.linear_index = 0;
                    self.linear_skip_to_issue_entry();
                }
            }
            Err(e) => {
                self.last_error = Some(format!("Linear: {}", e));
            }
        }
    }

    // --- Prompt modal helpers ---

    /// Open the prompt modal for the currently selected ticket (any issue management tab).
    pub fn open_prompt_modal_for_current(&mut self) {
        if !self.has_claude {
            self.last_error = Some("claude CLI not found on PATH".to_string());
            return;
        }

        let ticket = match self.active_tab {
            ActiveTab::GitHubPRs => self
                .gh_selected_pr()
                .map(prompt_builder::ticket_from_github_pr),
            ActiveTab::GitHubIssues => self
                .issues_selected()
                .map(prompt_builder::ticket_from_github_issue),
            ActiveTab::Linear => self
                .linear_selected_issue()
                .map(prompt_builder::ticket_from_linear),
            ActiveTab::Jira => self
                .jira_selected_issue()
                .map(prompt_builder::ticket_from_jira),
            _ => None,
        };

        if let Some(ticket) = ticket {
            let prompt = prompt_builder::build_default_prompt(&ticket);
            let mut editor = tui_textarea::TextArea::default();
            editor.insert_str(&prompt);
            editor.move_cursor(tui_textarea::CursorMove::Top);
            editor.move_cursor(tui_textarea::CursorMove::Head);

            self.prompt_editor = Some(editor);
            self.prompt_ticket_info = Some(ticket);
            self.show_prompt_modal = true;
        }
    }

    /// Confirm and launch the process from the prompt modal.
    pub fn confirm_prompt_modal(&mut self) {
        let prompt = if let Some(ref editor) = self.prompt_editor {
            editor.lines().join("\n")
        } else {
            return;
        };

        let ticket = match self.prompt_ticket_info.take() {
            Some(t) => t,
            None => return,
        };

        self.show_prompt_modal = false;
        self.prompt_editor = None;

        self.spawn_claude_process(&ticket, &prompt);
    }

    /// Cancel and close the prompt modal.
    pub fn cancel_prompt_modal(&mut self) {
        self.show_prompt_modal = false;
        self.prompt_editor = None;
        self.prompt_ticket_info = None;
    }

    // --- Process management ---

    /// Initialize the process output channel if not already created.
    fn ensure_process_channel(&mut self) {
        if self.process_tx.is_none() {
            let (tx, rx) = mpsc::channel();
            self.process_tx = Some(tx);
            self.process_rx = Some(rx);
        }
    }

    /// Spawn a new Claude Code process with the given prompt.
    fn spawn_claude_process(&mut self, ticket: &TicketInfo, prompt: &str) {
        self.ensure_process_channel();

        let id = self.next_process_id;
        self.next_process_id += 1;

        let tx = self.process_tx.as_ref().unwrap().clone();
        match process_runner::spawn_claude_headless(id, prompt, &self.project_cwd, tx) {
            Ok(child) => {
                let process = SpawnedProcess {
                    id,
                    label: ticket.key.clone(),
                    title: ticket.title.clone(),
                    source: ticket.source.clone(),
                    status: ProcessStatus::Running,
                    prompt: prompt.to_string(),
                    cwd: self.project_cwd.clone(),
                    output_lines: Vec::new(),
                    error_lines: Vec::new(),
                };
                self.processes.push(process);
                self.process_children.push((id, child));

                // Auto-switch to Processes tab
                self.active_tab = ActiveTab::Processes;
                self.process_index = self.processes.len() - 1;
                self.process_output_scroll = 0;
            }
            Err(e) => {
                self.last_error = Some(format!("Failed to spawn claude: {}", e));
            }
        }
    }

    fn linear_skip_to_next_issue(&mut self) {
        if self.linear_flat_list.is_empty() {
            return;
        }
        let start = self.linear_index + 1;
        for i in start..self.linear_flat_list.len() {
            if matches!(self.linear_flat_list[i], FlatLinearItem::Issue(_)) {
                self.linear_index = i;
                return;
            }
        }
    }

    fn linear_skip_to_prev_issue(&mut self) {
        if self.linear_index == 0 || self.linear_flat_list.is_empty() {
            return;
        }
        for i in (0..self.linear_index).rev() {
            if matches!(self.linear_flat_list[i], FlatLinearItem::Issue(_)) {
                self.linear_index = i;
                return;
            }
        }
    }

    fn linear_skip_to_issue_entry(&mut self) {
        if self.linear_flat_list.is_empty() {
            return;
        }
        let idx = self.linear_index.min(self.linear_flat_list.len() - 1);
        if matches!(
            self.linear_flat_list[idx],
            FlatLinearItem::AssignmentHeader(_)
        ) {
            for i in (idx + 1)..self.linear_flat_list.len() {
                if matches!(self.linear_flat_list[i], FlatLinearItem::Issue(_)) {
                    self.linear_index = i;
                    return;
                }
            }
        }
    }

    pub fn linear_selected_issue(&self) -> Option<&LinearIssue> {
        if self.linear_flat_list.is_empty() {
            return None;
        }
        let idx = self.linear_index.min(self.linear_flat_list.len() - 1);
        match &self.linear_flat_list[idx] {
            FlatLinearItem::Issue(issue) => Some(issue),
            _ => None,
        }
    }

    pub fn linear_open_selected(&self) {
        if let Some(issue) = self.linear_selected_issue() {
            if !issue.url.is_empty() {
                cli_detect::open_url(&issue.url);
            }
        }
    }

    /// Poll for process output messages (called from the event loop).
    pub fn poll_process_output(&mut self) {
        let rx = match self.process_rx {
            Some(ref rx) => rx,
            None => return,
        };

        while let Ok(msg) = rx.try_recv() {
            match msg {
                ProcessOutput::Stdout(id, line) => {
                    if let Some(proc) = self.processes.iter_mut().find(|p| p.id == id) {
                        proc.output_lines.push(line);
                    }
                }
                ProcessOutput::Stderr(id, line) => {
                    if let Some(proc) = self.processes.iter_mut().find(|p| p.id == id) {
                        proc.error_lines.push(line);
                    }
                }
                ProcessOutput::Exited(id, success) => {
                    if let Some(proc) = self.processes.iter_mut().find(|p| p.id == id) {
                        proc.status = if success {
                            ProcessStatus::Completed
                        } else {
                            ProcessStatus::Failed
                        };
                    }
                    self.process_children.retain(|(pid, _)| *pid != id);
                }
            }
        }

        // Check for exited children
        let mut exited = Vec::new();
        for (id, child) in &mut self.process_children {
            match child.try_wait() {
                Ok(Some(status)) => {
                    exited.push((*id, status.success()));
                }
                Ok(None) => {} // still running
                Err(_) => {
                    exited.push((*id, false));
                }
            }
        }
        for (id, success) in exited {
            if let Some(proc) = self.processes.iter_mut().find(|p| p.id == id) {
                if proc.status == ProcessStatus::Running {
                    proc.status = if success {
                        ProcessStatus::Completed
                    } else {
                        ProcessStatus::Failed
                    };
                }
            }
            self.process_children.retain(|(pid, _)| *pid != id);
        }
    }

    /// Get the currently selected process.
    pub fn selected_process(&self) -> Option<&SpawnedProcess> {
        if self.processes.is_empty() {
            return None;
        }
        let idx = self.process_index.min(self.processes.len() - 1);
        Some(&self.processes[idx])
    }

    /// Kill the currently selected process.
    pub fn kill_selected_process(&mut self) {
        if self.processes.is_empty() {
            return;
        }
        let idx = self.process_index.min(self.processes.len() - 1);
        let id = self.processes[idx].id;

        if self.processes[idx].status != ProcessStatus::Running {
            return;
        }

        if let Some(pos) = self.process_children.iter_mut().position(|(pid, _)| *pid == id) {
            let _ = self.process_children[pos].1.kill();
            self.process_children.remove(pos);
        }
        self.processes[idx].status = ProcessStatus::Failed;
    }
}
