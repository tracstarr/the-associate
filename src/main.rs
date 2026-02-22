mod app;
mod config;
mod data;
mod event;
mod model;
mod pane_send;
mod ui;
mod watcher;

use std::io;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use anyhow::Result;
use clap::Parser;
use crossterm::event::{self as ct_event, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::app::App;
use crate::event::AppEvent;

#[derive(Parser)]
#[command(
    name = "assoc",
    version,
    about = "The Associate - Claude Code Session Dashboard",
    override_help = HELP_TEXT,
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Project directory to monitor (defaults to current directory)
    #[arg(long, global = true)]
    cwd: Option<PathBuf>,

    /// Indicate that exactly two WT panes are open (enables pane-send with 'i')
    #[arg(long, global = true)]
    two_pane: bool,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Launch Windows Terminal with Claude Code + Associate side by side
    Launch {
        /// Session ID to resume
        #[arg(long)]
        resume: Option<String>,

        /// Claude pane width ratio (0.01-0.99)
        #[arg(long, default_value_t = 0.5, value_parser = parse_claude_ratio)]
        claude_ratio: f64,

        /// Terminal columns
        #[arg(long, default_value_t = 200)]
        cols: u32,

        /// Terminal rows
        #[arg(long, default_value_t = 50)]
        rows: u32,

        /// Extra arguments passed to claude (e.g. --dangerously-skip-permissions)
        #[arg(last = true)]
        claude_args: Vec<String>,
    },
}

const HELP_TEXT: &str = "\
The Associate - Claude Code Session Dashboard

USAGE:
  assoc [OPTIONS]                   Start the TUI dashboard
  assoc launch [OPTIONS] [-- ...]   Open Windows Terminal with Claude + dashboard

MODES:
  (default)   Interactive TUI that monitors Claude Code sessions, teams,
              todos, git status, and plans for the given project directory.

  launch      Opens Windows Terminal with two panes side by side:
              left = Claude Code, right = Associate dashboard.
              Requires Windows Terminal (wt.exe) to be installed.

GLOBAL OPTIONS:
  --cwd <DIR>       Project directory to monitor [default: current dir]
  --two-pane        Enable two-pane mode (pane send with 'i')
  -h, --help        Print this help
  -V, --version     Print version

LAUNCH OPTIONS:
  --resume <ID>             Resume a Claude Code session by ID
  --claude-ratio <FLOAT>    Claude pane width ratio, 0.01-0.99 [default: 0.5]
  --cols <N>                Terminal columns [default: 200]
  --rows <N>                Terminal rows [default: 50]
  -- <ARGS>...              Extra arguments passed to claude
                            (e.g. -- --dangerously-skip-permissions)

TUI KEYBINDINGS:
  1-9                Jump to tab by number
  Tab / Shift+Tab    Cycle tabs
  j/k  Up/Down       Navigate list / scroll content
  h/l  Left/Right    Switch panes
  Enter              Select item / open content pane
  g / G              Jump to top / bottom
  f                  Toggle follow mode (Sessions tab)
  o                  Open session in new WT pane (Sessions tab)
  s                  Cycle subagent transcripts (Sessions tab)
  b                  Toggle file browser (Git tab)
  e                  Edit file (file browser, Content pane)
  Ctrl+S / Esc       Save / cancel edit (file browser)
  n                  New issue (Issues tab)
  e                  Edit issue (Issues tab) / file (browser)
  c                  Comment on issue (Issues tab)
  p                  Launch Claude Code prompt (PRs / Issues / Linear / Jira)
  n                  Spawn new terminal session (Terminals tab) / New issue (Issues tab)
  x                  Close/reopen issue (Issues tab) / Kill process (Processes/Terminals tab)
  d / Del            Delete file (Sessions / Teams / Todos / Plans)
  o                  Open in browser (PRs / Issues / Jira / Linear)
  r                  Refresh data (PRs / Issues / Jira / Linear)
  t                  Show transitions (Jira)
  /                  Search issues (Jira)
  i                  Send input to Claude pane
  ?                  Toggle help overlay
  q / Ctrl+C         Quit

EXAMPLES:
  assoc --cwd C:\\dev\\myproject
  assoc launch --cwd C:\\dev\\myproject -- --dangerously-skip-permissions
  assoc launch --resume abc123 --claude-ratio 0.6";

fn parse_claude_ratio(s: &str) -> Result<f64, String> {
    let v: f64 = s
        .parse()
        .map_err(|_| format!("'{}' is not a valid float", s))?;
    if v < 0.01 || v > 0.99 {
        return Err(format!(
            "claude-ratio must be between 0.01 and 0.99, got {}",
            v
        ));
    }
    Ok(v)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let project_cwd = resolve_cwd(cli.cwd)?;

    match cli.command {
        Some(Command::Launch {
            resume,
            claude_ratio,
            cols,
            rows,
            claude_args,
        }) => launch_wt(&project_cwd, resume, claude_ratio, cols, rows, &claude_args),
        None => run_tui(project_cwd, cli.two_pane),
    }
}

fn resolve_cwd(cwd: Option<PathBuf>) -> Result<PathBuf> {
    match cwd {
        Some(p) => {
            let canonical = std::fs::canonicalize(p)?;
            // On Windows, canonicalize returns \\?\C:\... extended-length paths.
            // Strip prefix so path encoding matches Claude Code's convention.
            let s = canonical.to_string_lossy();
            if s.starts_with(r"\\?\") {
                Ok(PathBuf::from(&s[4..]))
            } else {
                Ok(canonical)
            }
        }
        None => Ok(std::env::current_dir()?),
    }
}

fn run_tui(project_cwd: PathBuf, two_pane: bool) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let result = run_app(&mut terminal, project_cwd, two_pane);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(ref e) = result {
        eprintln!("Error: {}", e);
    }
    result
}

fn launch_wt(
    project_cwd: &PathBuf,
    resume: Option<String>,
    claude_ratio: f64,
    cols: u32,
    rows: u32,
    claude_args: &[String],
) -> Result<()> {
    // Find our own exe to spawn in the assoc pane
    let self_exe = std::env::current_exe()?;
    let dir = project_cwd.to_string_lossy();

    // Build claude arguments
    let mut claude_cmd_args: Vec<String> = Vec::new();
    if let Some(ref session_id) = resume {
        claude_cmd_args.push("--resume".to_string());
        claude_cmd_args.push(session_id.clone());
    }
    // Append any extra args passed after --
    claude_cmd_args.extend_from_slice(claude_args);

    let claude_full = if claude_cmd_args.is_empty() {
        "claude".to_string()
    } else {
        format!("claude {}", claude_cmd_args.join(" "))
    };

    // wt.exe new-tab: assoc (right/initial pane)
    // split-pane: claude (left pane, takes claude_ratio of width)
    // focus-pane: focus claude pane
    let status = std::process::Command::new("wt.exe")
        .arg("--size")
        .arg(format!("{},{}", cols, rows))
        .arg("new-tab")
        .arg("--title")
        .arg("The Associate")
        .arg("-d")
        .arg(&*dir)
        .arg("--")
        .arg(&self_exe)
        .arg("--cwd")
        .arg(&*dir)
        .arg("--two-pane")
        .arg(";")
        .arg("split-pane")
        .arg("-V")
        .arg("-s")
        .arg(format!("{}", claude_ratio))
        .arg("--title")
        .arg("Claude Code")
        .arg("-d")
        .arg(&*dir)
        .arg("--")
        .args(claude_full.split_whitespace())
        .arg(";")
        .arg("focus-pane")
        .arg("-t")
        .arg("1")
        .status();

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => anyhow::bail!("wt.exe exited with {}", s),
        Err(e) => anyhow::bail!(
            "Failed to run wt.exe: {}. Is Windows Terminal installed?",
            e
        ),
    }
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    project_cwd: PathBuf,
    two_pane: bool,
) -> Result<()> {
    let mut app = App::new(project_cwd);
    app.two_pane = two_pane;

    // Create event channel before initial load so async spawners can send results
    let (tx, rx) = mpsc::channel::<AppEvent>();
    app.event_tx = Some(tx.clone());

    // Initial data load (async loaders will send results through the channel)
    app.load_all();

    // Setup file watcher (skips directories for disabled tabs)
    let _debouncer = watcher::start_watcher(
        app.claude_home.clone(),
        app.encoded_project.clone(),
        app.project_cwd.clone(),
        tx,
        &app.project_config.tabs,
    )?;

    let tick_rate = Duration::from_millis(app.project_config.tick_rate());
    let poll_interval = Duration::from_secs(60);
    let mut last_tick = Instant::now();

    loop {
        // Draw only when dirty
        if app.dirty {
            terminal.draw(|f| ui::draw(f, &app))?;
            app.dirty = false;
        }

        // Handle events
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());

        // Check for crossterm events
        if ct_event::poll(timeout)? {
            if let Event::Key(key) = ct_event::read()? {
                if key.kind == KeyEventKind::Press {
                    handle_key(&mut app, key);
                    app.mark_dirty();
                }
            }
        }

        // Check for file watcher and pane send events
        while let Ok(evt) = rx.try_recv() {
            match evt {
                AppEvent::FileChanged(change) => app.handle_file_change(change),
                AppEvent::PaneSendComplete(err) => app.handle_send_complete(err),
                AppEvent::GitHubPrsLoaded(result) => app.handle_github_prs_loaded(result),
                AppEvent::GitHubIssuesLoaded(result) => {
                    app.handle_github_issues_loaded(result)
                }
                AppEvent::JiraIssuesLoaded(result) => {
                    app.handle_jira_issues_loaded(result)
                }
                AppEvent::LinearIssuesLoaded(result) => {
                    app.handle_linear_issues_loaded(result)
                }
                AppEvent::GitStatusLoaded(result) => app.handle_git_status_loaded(result),
                AppEvent::GitDiffLoaded(result) => app.handle_git_diff_loaded(result),
            }
            app.mark_dirty();
        }

        // Tick
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();

            // Poll GitHub PRs every 60s (skip if tab disabled)
            if app.is_tab_enabled(&app::ActiveTab::GitHubPRs)
                && app.has_gh
                && app.gh_repo.is_some()
                && app.gh_last_poll.elapsed() >= poll_interval
            {
                app.load_github_prs();
            }

            // Poll GitHub Issues every 60s (skip if tab disabled)
            if app.is_tab_enabled(&app::ActiveTab::GitHubIssues)
                && app.gh_issues_enabled
                && app.gh_issues_repo.is_some()
                && app.gh_issues_last_poll.elapsed() >= poll_interval
            {
                app.load_github_issues();
            }

            // Poll Jira every 60s (skip if tab disabled)
            if app.is_tab_enabled(&app::ActiveTab::Jira)
                && app.has_jira
                && app.jira_last_poll.elapsed() >= poll_interval
            {
                app.load_jira_issues();
            }

            // Poll Linear every 60s (skip if tab disabled)
            if app.is_tab_enabled(&app::ActiveTab::Linear)
                && app.has_linear
                && app.linear_last_poll.elapsed() >= poll_interval
            {
                app.load_linear_issues();
            }

            // Poll spawned process output
            app.poll_process_output();

            // Clear stale send status
            app.clear_stale_send_status();

            app.mark_dirty();
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn handle_key(app: &mut App, key: KeyEvent) {
    // Global keybindings (always active)
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
            return;
        }
        KeyCode::Char('?')
            if !app.fb_editing && !app.jira_search_mode && !app.gh_issues_editing =>
        {
            app.show_help = !app.show_help;
            return;
        }
        KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.show_help = !app.show_help;
            return;
        }
        KeyCode::Esc if app.show_help => {
            app.show_help = false;
            return;
        }
        _ => {}
    }

    // Don't process other keys when help is showing
    if app.show_help {
        return;
    }

    // Delete confirmation dialog
    if app.confirm_delete {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.execute_delete(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_delete(),
            _ => {}
        }
        return;
    }

    // Prompt picker — select from available prompts
    if app.show_prompt_picker {
        handle_prompt_picker_key(app, key);
        return;
    }

    // Prompt modal — pass keys to prompt editor
    if app.show_prompt_modal {
        handle_prompt_modal_key(app, key);
        return;
    }

    // Pane send input mode
    if app.send_mode {
        handle_send_key(app, key);
        return;
    }

    // File browser edit mode — pass keys to TextArea
    if app.fb_editing {
        handle_fb_edit_key(app, key);
        return;
    }

    // GitHub Issues edit mode — pass keys to TextArea editors
    if app.gh_issues_editing {
        handle_issues_edit_key(app, key);
        return;
    }

    // Jira transition popup — number keys select transition
    if app.jira_show_transitions {
        match key.code {
            KeyCode::Esc => app.jira_show_transitions = false,
            KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                let idx = (c as usize) - ('1' as usize);
                app.jira_do_transition(idx);
            }
            _ => {}
        }
        return;
    }

    // Jira search mode — text input
    if app.jira_search_mode {
        match key.code {
            KeyCode::Esc => {
                app.jira_search_mode = false;
                app.jira_search_input.clear();
                app.load_jira_issues(); // reset to default view
            }
            KeyCode::Enter => {
                app.jira_search();
            }
            KeyCode::Backspace => {
                app.jira_search_input.pop();
            }
            KeyCode::Char(c) => {
                app.jira_search_input.push(c);
            }
            _ => {}
        }
        return;
    }

    // Quit
    if key.code == KeyCode::Char('q') {
        app.should_quit = true;
        return;
    }

    match key.code {
        // Tab switching
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                app.prev_tab();
            } else {
                app.next_tab();
            }
        }

        // Dynamic number keys
        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
            let idx = (c as usize) - ('1' as usize);
            let tabs = app.visible_tabs();
            if idx < tabs.len() {
                app.switch_to_tab(tabs[idx].clone());
            }
        }

        // Navigation
        KeyCode::Char('j') | KeyCode::Down => app.navigate_down(),
        KeyCode::Char('k') | KeyCode::Up => app.navigate_up(),
        KeyCode::Char('h') | KeyCode::Left => app.navigate_left(),
        KeyCode::Char('l') | KeyCode::Right => app.navigate_right(),
        KeyCode::Enter => app.select_item(),

        // Jump
        KeyCode::Char('g') => app.jump_top(),
        KeyCode::Char('G') => app.jump_bottom(),

        // Follow mode (Sessions tab / Processes tab / Terminals tab)
        KeyCode::Char('f') => match app.active_tab {
            app::ActiveTab::Sessions => app.toggle_follow(),
            app::ActiveTab::Processes => app.toggle_process_follow(),
            app::ActiveTab::Terminals => app.toggle_terminal_follow(),
            _ => {}
        },

        // Subagent transcript cycling (Sessions tab) / Jump to session (Processes / Terminals tab)
        KeyCode::Char('s') => {
            if app.active_tab == app::ActiveTab::Sessions
                && app.sessions_pane == app::SessionsPane::Transcript
            {
                app.cycle_subagent();
            } else if app.active_tab == app::ActiveTab::Processes {
                app.jump_to_process_session();
            } else if app.active_tab == app::ActiveTab::Terminals {
                app.jump_to_terminal_session();
            }
        }

        // File browser toggle (Git tab)
        KeyCode::Char('b') => {
            if app.active_tab == app::ActiveTab::Git {
                app.toggle_git_mode();
            }
        }

        // Backspace for file browser navigation
        KeyCode::Backspace => {
            if app.active_tab == app::ActiveTab::Git && app.git_mode == app::GitMode::Browse {
                app.fb_backspace();
            }
        }

        // Edit file (file browser) or edit issue (Issues tab)
        KeyCode::Char('e') => match app.active_tab {
            app::ActiveTab::Git if app.git_mode == app::GitMode::Browse => {
                app.fb_start_edit();
            }
            app::ActiveTab::GitHubIssues => {
                app.issues_start_edit();
            }
            _ => {}
        },

        // New issue (Issues tab) / New terminal session (Terminals tab)
        KeyCode::Char('n') => {
            if app.active_tab == app::ActiveTab::GitHubIssues {
                app.issues_start_create();
            } else if app.active_tab == app::ActiveTab::Terminals {
                app.terminal_spawn_new();
            }
        }

        // Comment on issue (Issues tab)
        KeyCode::Char('c') => {
            if app.active_tab == app::ActiveTab::GitHubIssues {
                app.issues_start_comment();
            }
        }

        // Launch Claude Code prompt modal (all issue tabs)
        KeyCode::Char('p') => match app.active_tab {
            app::ActiveTab::GitHubPRs
            | app::ActiveTab::GitHubIssues
            | app::ActiveTab::Linear
            | app::ActiveTab::Jira => {
                app.open_prompt_modal_for_current();
            }
            _ => {}
        },

        // Close/reopen issue (Issues tab) / Kill process (Processes tab) / Kill terminal (Terminals tab)
        KeyCode::Char('x') => match app.active_tab {
            app::ActiveTab::GitHubIssues => app.issues_toggle_state(),
            app::ActiveTab::Processes => app.kill_selected_process(),
            app::ActiveTab::Terminals => app.kill_selected_terminal(),
            _ => {}
        },

        // Open in browser / open session in WT pane
        KeyCode::Char('o') => match app.active_tab {
            app::ActiveTab::GitHubPRs => app.gh_open_selected(),
            app::ActiveTab::GitHubIssues => app.issues_open_in_browser(),
            app::ActiveTab::Jira => app.jira_open_selected(),
            app::ActiveTab::Linear => app.linear_open_selected(),
            app::ActiveTab::Sessions => app.open_session_in_wt(),
            _ => {}
        },

        // Refresh
        KeyCode::Char('r') => match app.active_tab {
            app::ActiveTab::GitHubPRs => app.load_github_prs(),
            app::ActiveTab::GitHubIssues => app.load_github_issues(),
            app::ActiveTab::Jira => app.load_jira_issues(),
            app::ActiveTab::Linear => app.load_linear_issues(),
            _ => {}
        },

        // Jira transitions
        KeyCode::Char('t') => {
            if app.active_tab == app::ActiveTab::Jira {
                app.jira_load_transitions();
            }
        }

        // Jira search
        KeyCode::Char('/') => {
            if app.active_tab == app::ActiveTab::Jira {
                app.jira_search_mode = true;
                app.jira_search_input.clear();
            }
        }

        // Delete file
        KeyCode::Char('d') | KeyCode::Delete => match app.active_tab {
            app::ActiveTab::Todos
            | app::ActiveTab::Plans
            | app::ActiveTab::Sessions
            | app::ActiveTab::Teams => app.request_delete(),
            _ => {}
        },

        // Send to Claude pane
        KeyCode::Char('i') => {
            if !app.send_pending {
                app.start_send_mode();
            }
        }

        _ => {}
    }
}

fn handle_send_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.cancel_send_mode();
        }
        KeyCode::Enter => {
            app.execute_send();
        }
        KeyCode::Backspace => {
            app.send_input.pop();
        }
        KeyCode::Char(c) => {
            app.send_input.push(c);
        }
        _ => {}
    }
}

fn handle_issues_edit_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.issues_save_edit();
        }
        KeyCode::Esc => {
            app.issues_cancel_edit();
        }
        KeyCode::Tab => {
            // Toggle between title and body fields (only in Create/Edit mode, not Comment)
            if !matches!(app.gh_issues_edit_mode, Some(app::IssueEditMode::Comment(_))) {
                app.gh_issues_edit_field = match app.gh_issues_edit_field {
                    app::IssueEditField::Title => app::IssueEditField::Body,
                    app::IssueEditField::Body => app::IssueEditField::Title,
                };
            }
        }
        _ => {
            // Pass key to active TextArea
            match app.gh_issues_edit_field {
                app::IssueEditField::Title => {
                    if let Some(ref mut editor) = app.gh_issues_title_editor {
                        editor.input(key);
                    }
                }
                app::IssueEditField::Body => {
                    if let Some(ref mut editor) = app.gh_issues_body_editor {
                        editor.input(key);
                    }
                }
            }
        }
    }
}

fn handle_prompt_picker_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if app.prompt_picker_index + 1 < app.prompt_picker_len() {
                app.prompt_picker_index += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.prompt_picker_index = app.prompt_picker_index.saturating_sub(1);
        }
        KeyCode::Enter => {
            app.confirm_prompt_picker();
        }
        KeyCode::Esc => {
            app.cancel_prompt_picker();
        }
        _ => {}
    }
}

fn handle_prompt_modal_key(app: &mut App, key: KeyEvent) {
    match key.code {
        // Ctrl+Enter to confirm and launch
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.confirm_prompt_modal();
        }
        // Esc to cancel
        KeyCode::Esc => {
            app.cancel_prompt_modal();
        }
        // All other keys go to the TextArea editor
        _ => {
            if let Some(ref mut editor) = app.prompt_editor {
                editor.input(key);
            }
        }
    }
}

fn handle_fb_edit_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.fb_save_edit();
        }
        KeyCode::Esc => {
            app.fb_cancel_edit();
        }
        _ => {
            // Pass key to TextArea
            if let Some(ref mut editor) = app.fb_editor {
                editor.input(key);
            }
        }
    }
}
