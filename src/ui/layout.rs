use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::{
    git_view, github_view, help_overlay, issues_view, jira_view, linear_view, plans_view,
    processes_view, prompt_modal, sessions_view, tabs, teams_view, theme, todos_view,
};
use crate::app::{ActiveTab, App, GitMode, SessionsPane};

pub fn draw_layout(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Tab bar
            Constraint::Min(3),    // Content
            Constraint::Length(1), // Status bar
        ])
        .split(f.area());

    // Tab bar
    tabs::draw_tab_bar(f, chunks[0], app);

    // Content area
    draw_content(f, chunks[1], app);

    // Status bar
    draw_status_bar(f, chunks[2], app);

    // Delete confirmation overlay
    if app.confirm_delete {
        draw_delete_confirm(f, f.area(), &app.delete_target_name);
    }

    // Help overlay (on top of everything)
    if app.show_help {
        help_overlay::draw_help(f, f.area());
    }

    // Prompt modal (on top of everything)
    if app.show_prompt_modal {
        prompt_modal::draw_prompt_modal(f, f.area(), app);
    }
}

fn draw_delete_confirm(f: &mut Frame, area: Rect, name: &str) {
    let width = 50u16.min(area.width.saturating_sub(4));
    let height = 5u16;

    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((area.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(area);

    let horiz = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((area.width.saturating_sub(width)) / 2),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(vert[1]);

    let popup_area = horiz[1];

    f.render_widget(Clear, popup_area);

    let display_name = if name.chars().count() > 36 {
        let truncated: String = name.chars().take(33).collect();
        format!("{}...", truncated)
    } else {
        name.to_string()
    };

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  Delete {}?", display_name),
            Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled(
                "  y",
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" yes  "),
            Span::styled(
                "n",
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" no"),
        ]),
    ];

    let block = Block::default()
        .title(" Confirm Delete ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Red));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, popup_area);
}

fn draw_content(f: &mut Frame, area: Rect, app: &App) {
    match app.active_tab {
        ActiveTab::Sessions => sessions_view::draw_sessions(f, area, app),
        ActiveTab::Teams => teams_view::draw_teams(f, area, app),
        ActiveTab::Todos => todos_view::draw_todos(f, area, app),
        ActiveTab::Git => git_view::draw_git(f, area, app),
        ActiveTab::Plans => plans_view::draw_plans(f, area, app),
        ActiveTab::GitHubPRs => github_view::draw_github(f, area, app),
        ActiveTab::GitHubIssues => issues_view::draw_issues(f, area, app),
        ActiveTab::Jira => jira_view::draw_jira(f, area, app),
        ActiveTab::Linear => linear_view::draw_linear(f, area, app),
        ActiveTab::Processes => processes_view::draw_processes(f, area, app),
    }
}

fn hint_text(app: &App) -> Vec<(&'static str, &'static str)> {
    let mut hints: Vec<(&str, &str)> = match app.active_tab {
        ActiveTab::Sessions => match app.sessions_pane {
            SessionsPane::List => vec![("j/k", "nav"), ("Enter", "select"), ("d", "delete")],
            SessionsPane::Transcript => vec![("f", "follow"), ("s", "subagent"), ("j/k", "scroll")],
        },
        ActiveTab::Teams => vec![
            ("j/k", "nav"),
            ("h/l", "panes"),
            ("Enter", "drill"),
            ("d", "delete"),
        ],
        ActiveTab::Todos => vec![("j/k", "nav"), ("h/l", "panes"), ("d", "delete")],
        ActiveTab::Git => {
            if app.git_mode == GitMode::Browse {
                vec![
                    ("e", "edit"),
                    ("Enter", "open"),
                    ("Bksp", "up"),
                    ("b", "status"),
                ]
            } else {
                vec![("j/k", "nav"), ("h/l", "panes"), ("b", "browse")]
            }
        }
        ActiveTab::Plans => vec![("j/k", "nav"), ("h/l", "panes"), ("d", "delete")],
        ActiveTab::GitHubPRs => vec![
            ("j/k", "nav"),
            ("o", "open"),
            ("r", "refresh"),
            ("p", "prompt"),
        ],
        ActiveTab::GitHubIssues => vec![
            ("j/k", "nav"),
            ("n", "new"),
            ("e", "edit"),
            ("c", "comment"),
            ("x", "close/open"),
            ("o", "browser"),
            ("r", "refresh"),
            ("p", "prompt"),
        ],
        ActiveTab::Jira => vec![
            ("j/k", "nav"),
            ("o", "open"),
            ("r", "refresh"),
            ("/", "search"),
            ("t", "transition"),
            ("p", "prompt"),
        ],
        ActiveTab::Linear => vec![
            ("j/k", "nav"),
            ("o", "open"),
            ("r", "refresh"),
            ("p", "prompt"),
        ],
        ActiveTab::Processes => vec![("j/k", "nav"), ("h/l", "panes"), ("x", "kill")],
    };
    hints.push(("^H", "help"));
    hints
}

fn draw_status_bar(f: &mut Frame, area: Rect, app: &App) {
    let mut left_spans: Vec<Span> = Vec::new();

    // Error display
    if let Some(ref err) = app.last_error {
        left_spans.push(Span::styled(
            format!(" ERR: {} ", err),
            ratatui::style::Style::new()
                .fg(ratatui::style::Color::Red)
                .bg(ratatui::style::Color::DarkGray),
        ));
    }

    // Follow mode indicator (only on sessions tab)
    if app.active_tab == ActiveTab::Sessions && app.follow_mode {
        left_spans.push(Span::styled(" FOLLOW ", theme::FOLLOW_ACTIVE));
    }

    // Browse mode indicator (Git tab)
    if app.active_tab == ActiveTab::Git && app.git_mode == GitMode::Browse {
        left_spans.push(Span::styled(
            " BROWSE ",
            ratatui::style::Style::new()
                .fg(ratatui::style::Color::Black)
                .bg(ratatui::style::Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ));
        if app.fb_editing {
            left_spans.push(Span::styled(
                " EDIT ",
                ratatui::style::Style::new()
                    .fg(ratatui::style::Color::Black)
                    .bg(ratatui::style::Color::Red)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ));
        }
    }

    // Issues edit mode indicator
    if app.active_tab == ActiveTab::GitHubIssues && app.gh_issues_editing {
        left_spans.push(Span::styled(
            " EDIT ",
            ratatui::style::Style::new()
                .fg(ratatui::style::Color::Black)
                .bg(ratatui::style::Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ));
    }

    // Jira search mode indicator
    if app.active_tab == ActiveTab::Jira && app.jira_search_mode {
        left_spans.push(Span::styled(
            " SEARCH ",
            ratatui::style::Style::new()
                .fg(ratatui::style::Color::Black)
                .bg(ratatui::style::Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ));
    }

    // Build right-aligned hint spans
    let hints = hint_text(app);
    let mut hint_spans: Vec<Span> = Vec::new();
    for (i, (key, desc)) in hints.iter().enumerate() {
        if i > 0 {
            hint_spans.push(Span::styled("  ", theme::STATUS_BAR));
        }
        hint_spans.push(Span::styled(*key, theme::HINT_KEY));
        hint_spans.push(Span::styled(":", theme::HINT_DESC));
        hint_spans.push(Span::styled(*desc, theme::HINT_DESC));
    }
    hint_spans.push(Span::styled(" ", theme::STATUS_BAR));

    let left_width: usize = left_spans.iter().map(|s| s.width()).sum();
    let hint_width: usize = hint_spans.iter().map(|s| s.width()).sum();
    let total = area.width as usize;
    let gap = total.saturating_sub(left_width + hint_width);

    let mut spans = left_spans;
    spans.push(Span::styled(" ".repeat(gap), theme::STATUS_BAR));
    spans.extend(hint_spans);

    let line = Line::from(spans);
    f.render_widget(Paragraph::new(line), area);
}
