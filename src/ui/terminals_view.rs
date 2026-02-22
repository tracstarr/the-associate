use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use super::theme;
use crate::app::{App, TerminalsPane};
use crate::model::process::ProcessStatus;

pub fn draw_terminals(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    draw_session_list(f, chunks[0], app);
    draw_session_output(f, chunks[1], app);
}

fn draw_session_list(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.terminals_pane == TerminalsPane::List;
    let border_style = if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let running_count = app
        .terminal_sessions
        .iter()
        .filter(|p| p.status == ProcessStatus::Running)
        .count();

    let title = if app.terminal_sessions.is_empty() {
        " Terminals ".to_string()
    } else {
        format!(
            " Terminals [{}/{}] ",
            running_count,
            app.terminal_sessions.len()
        )
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.terminal_sessions.is_empty() {
        let p = Paragraph::new(
            "No terminal sessions.\n\nPress 'n' to spawn a new\nClaude Code session.",
        )
        .style(theme::EMPTY_STATE)
        .block(block)
        .wrap(Wrap { trim: false });
        f.render_widget(p, area);
        return;
    }

    let items: Vec<ListItem> = app
        .terminal_sessions
        .iter()
        .map(|sess| {
            let status_icon = match sess.status {
                ProcessStatus::Running => Span::styled(" > ", theme::PROCESS_RUNNING),
                ProcessStatus::Completed => Span::styled(" + ", theme::PROCESS_COMPLETED),
                ProcessStatus::Failed => Span::styled(" x ", theme::PROCESS_FAILED),
            };

            let label = if sess.label.is_empty() {
                sess.label.as_str()
            } else {
                sess.label.as_str()
            };

            let title_text = if sess.title.is_empty() {
                truncate(&sess.prompt, 35)
            } else {
                truncate(&sess.title, 35)
            };

            let line = Line::from(vec![
                status_icon,
                Span::styled(label, theme::LIST_NORMAL.add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::styled(title_text, theme::LIST_NORMAL),
            ]);

            ListItem::new(line)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(
        app.terminal_index.min(app.terminal_sessions.len().saturating_sub(1)),
    ));

    let list = List::new(items)
        .block(block)
        .highlight_style(theme::LIST_SELECTED);

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_session_output(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.terminals_pane == TerminalsPane::Output;
    let border_style = if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let sess = app.selected_terminal();

    let title = if let Some(s) = sess {
        let status_str = match s.status {
            ProcessStatus::Running => "RUNNING",
            ProcessStatus::Completed => "DONE",
            ProcessStatus::Failed => "FAILED",
        };
        let follow_indicator = if app.terminal_follow { " [FOLLOW]" } else { "" };
        let display = if s.title.is_empty() {
            truncate(&s.prompt, 40)
        } else {
            truncate(&s.title, 40)
        };
        format!(" {} [{}]{} ", display, status_str, follow_indicator)
    } else {
        " Output ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let Some(sess) = sess else {
        let p = Paragraph::new("Select a session to view output.\n\nPress 'n' to start a new session.")
            .style(theme::EMPTY_STATE)
            .block(block)
            .wrap(Wrap { trim: false });
        f.render_widget(p, area);
        return;
    };

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    // Raw output lines (plain text from claude -p without stream-json)
    for line in &sess.output_lines {
        lines.push(Line::from(Span::styled(
            line.as_str(),
            theme::PROCESS_STDOUT,
        )));
    }

    if !sess.error_lines.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "--- stderr ---",
            theme::PROCESS_STDERR_HEADER,
        )));
        for line in &sess.error_lines {
            lines.push(Line::from(Span::styled(
                line.as_str(),
                theme::PROCESS_STDERR,
            )));
        }
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "Waiting for output...",
            theme::EMPTY_STATE,
        )));
    }

    // Apply scroll offset
    let inner_height = inner.height as usize;
    let total = lines.len();
    let scroll_offset = app
        .terminal_output_scroll
        .min(total.saturating_sub(inner_height));
    let visible_end = (scroll_offset + inner_height).min(total);

    let visible_lines: Vec<Line> = lines[scroll_offset..visible_end].to_vec();
    let paragraph = Paragraph::new(visible_lines).wrap(Wrap { trim: false });
    f.render_widget(paragraph, inner);
}

fn truncate(s: &str, max: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max {
        s.to_string()
    } else {
        let cut = max.saturating_sub(3);
        let end = s.char_indices().nth(cut).map(|(i, _)| i).unwrap_or(s.len());
        format!("{}...", &s[..end])
    }
}
