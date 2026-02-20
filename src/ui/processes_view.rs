use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use super::theme;
use crate::app::{App, ProcessesPane};
use crate::model::process::{ProcessStatus, TicketSource};

pub fn draw_processes(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    draw_process_list(f, chunks[0], app);
    draw_process_output(f, chunks[1], app);
}

fn draw_process_list(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.processes_pane == ProcessesPane::List;
    let border_style = if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let running_count = app
        .processes
        .iter()
        .filter(|p| p.status == ProcessStatus::Running)
        .count();
    let title = format!(" Processes [{}/{}] ", running_count, app.processes.len());

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.processes.is_empty() {
        let p = Paragraph::new(
            "No processes running\n\nPress 'c' on a GitHub PR or Jira issue to launch",
        )
        .style(theme::EMPTY_STATE)
        .block(block)
        .wrap(Wrap { trim: false });
        f.render_widget(p, area);
        return;
    }

    let items: Vec<ListItem> = app
        .processes
        .iter()
        .map(|proc| {
            let status_icon = match proc.status {
                ProcessStatus::Running => Span::styled(" * ", theme::PROCESS_RUNNING),
                ProcessStatus::Completed => Span::styled(" + ", theme::PROCESS_COMPLETED),
                ProcessStatus::Failed => Span::styled(" x ", theme::PROCESS_FAILED),
            };

            let source_icon = match proc.source {
                TicketSource::GitHubPR => "GH",
                TicketSource::GitHubIssue => "GH",
                TicketSource::Linear => "LN",
                TicketSource::Jira => "JR",
            };

            let line = Line::from(vec![
                status_icon,
                Span::styled(
                    format!("[{}] ", source_icon),
                    theme::LIST_NORMAL.add_modifier(Modifier::DIM),
                ),
                Span::styled(&proc.label, theme::LIST_NORMAL.add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::styled(truncate(&proc.title, 30), theme::LIST_NORMAL),
            ]);

            ListItem::new(line)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.process_index));

    let list = List::new(items)
        .block(block)
        .highlight_style(theme::LIST_SELECTED);

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_process_output(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.processes_pane == ProcessesPane::Output;
    let border_style = if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let proc = app.selected_process();

    let title = if let Some(p) = proc {
        let status_str = match p.status {
            ProcessStatus::Running => "RUNNING",
            ProcessStatus::Completed => "DONE",
            ProcessStatus::Failed => "FAILED",
        };
        let sid_suffix = p
            .session_id
            .as_deref()
            .map(|s| format!(" [sid:{}]", &s[..8.min(s.len())]))
            .unwrap_or_default();
        let follow_indicator = if app.process_follow { " [FOLLOW]" } else { "" };
        format!(
            " {} {} [{}]{}{} ",
            p.label, p.title, status_str, sid_suffix, follow_indicator
        )
    } else {
        " Output ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let Some(proc) = proc else {
        let p = Paragraph::new("Select a process to view output")
            .style(theme::EMPTY_STATE)
            .block(block);
        f.render_widget(p, area);
        return;
    };

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    if !proc.progress_lines.is_empty() {
        for line in &proc.progress_lines {
            let style = if line.starts_with("->") {
                theme::TX_TOOL
            } else if line.starts_with("[SUCCESS") {
                theme::PROCESS_COMPLETED
            } else if line.starts_with("[FAIL") {
                theme::PROCESS_FAILED
            } else if line.starts_with("Session:") {
                theme::TX_SYSTEM
            } else {
                theme::PROCESS_STDOUT
            };
            lines.push(Line::from(Span::styled(line.as_str(), style)));
        }
    } else {
        // Fall back to raw output lines dimly if no parsed progress yet
        for line in &proc.output_lines {
            lines.push(Line::from(Span::styled(
                line.as_str(),
                theme::PROCESS_STDOUT.add_modifier(Modifier::DIM),
            )));
        }
    }

    if !proc.error_lines.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "--- stderr ---",
            theme::PROCESS_STDERR_HEADER,
        )));
        for line in &proc.error_lines {
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
        .process_output_scroll
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
