use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use super::theme;
use crate::app::{App, SessionsPane};
use crate::model::transcript::TranscriptItemKind;

fn truncate_chars(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

pub fn draw_sessions(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    draw_session_list(f, chunks[0], app);
    draw_transcript(f, chunks[1], app);
}

fn draw_session_list(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.sessions_pane == SessionsPane::List;
    let border_style = if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let title = format!(" Sessions [{}] ", app.sessions.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.sessions.is_empty() {
        let msg = Paragraph::new(format!(
            "No sessions found.\nLooking in: ~/.claude/projects/{}/",
            app.encoded_project
        ))
        .style(theme::EMPTY_STATE)
        .block(block)
        .wrap(Wrap { trim: false });
        f.render_widget(msg, area);
        return;
    }

    let items: Vec<ListItem> = app
        .sessions
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let prefix = if i == app.session_list_index {
                ">"
            } else {
                " "
            };
            let branch = s.branch();
            let branch_span = if branch.is_empty() {
                Span::raw("")
            } else {
                Span::styled(format!("  {}", branch), theme::BRANCH_LABEL)
            };

            let title_raw = s.display_title();
            let title_text = truncate_chars(&title_raw, 30).to_string();

            // Subagent indicator: check if this is the loaded session and has subagents
            let subagent_span = if app.loaded_session_id.as_deref() == Some(&s.session_id)
                && !app.subagents.is_empty()
            {
                Span::styled(
                    format!(" [{} agents]", app.subagents.len()),
                    theme::SUBAGENT_BADGE,
                )
            } else {
                Span::raw("")
            };

            let line = Line::from(vec![
                Span::raw(format!("{} ", prefix)),
                Span::raw(title_text),
                branch_span,
                subagent_span,
            ]);
            ListItem::new(line)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.session_list_index));

    let list = List::new(items)
        .block(block)
        .highlight_style(theme::LIST_SELECTED);

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_transcript(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.sessions_pane == SessionsPane::Transcript;
    let border_style = if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    // Build title showing session name
    let session_title = if let Some(ref session) = app.sessions.get(app.session_list_index) {
        let raw = session.display_title();
        truncate_chars(&raw, 30).to_string()
    } else {
        String::new()
    };

    // If subagents exist, show a source tab bar
    let has_subagents = !app.subagents.is_empty();

    let title = if has_subagents {
        if app.viewing_subagent && app.subagent_index < app.subagents.len() {
            let agent_id = &app.subagents[app.subagent_index].agent_id;
            format!(" agent-{} ", agent_id)
        } else {
            format!(" {} ", session_title)
        }
    } else {
        format!(" Transcript: {} ", session_title)
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    // Choose which transcript to display
    let items = if app.viewing_subagent {
        &app.subagent_transcript
    } else {
        &app.transcript_items
    };

    if items.is_empty() && !has_subagents {
        let msg = if app.sessions.is_empty() {
            "Select a session to view transcript"
        } else {
            "Loading transcript..."
        };
        let p = Paragraph::new(msg).style(theme::EMPTY_STATE).block(block);
        f.render_widget(p, area);
        return;
    }

    // If we have subagents, reserve one line at the top for the source tab bar
    if has_subagents {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Fill(1)])
            .split(block.inner(area));

        // Draw the block border
        f.render_widget(block, area);

        // Draw the source tab bar
        let mut tabs = Vec::new();
        let main_style = if !app.viewing_subagent {
            theme::SUBAGENT_TAB_ACTIVE
        } else {
            theme::SUBAGENT_TAB_INACTIVE
        };
        tabs.push(Span::styled(" Main ", main_style));

        for (i, sa) in app.subagents.iter().enumerate() {
            tabs.push(Span::raw(" "));
            let style = if app.viewing_subagent && app.subagent_index == i {
                theme::SUBAGENT_TAB_ACTIVE
            } else {
                theme::SUBAGENT_TAB_INACTIVE
            };
            let short_id = truncate_chars(&sa.agent_id, 7);
            tabs.push(Span::styled(format!(" {} ", short_id), style));
        }
        tabs.push(Span::styled(
            "  (s to cycle)",
            ratatui::style::Style::new().fg(ratatui::style::Color::DarkGray),
        ));

        let tab_line = Paragraph::new(Line::from(tabs));
        f.render_widget(tab_line, chunks[0]);

        // Draw transcript content
        draw_transcript_content(f, chunks[1], items, app);
    } else {
        // No subagents — draw normally
        let inner = block.inner(area);
        f.render_widget(block, area);
        draw_transcript_content(f, inner, items, app);
    }
}

fn draw_transcript_content(
    f: &mut Frame,
    area: Rect,
    items: &[crate::model::transcript::TranscriptItem],
    app: &App,
) {
    if items.is_empty() {
        let p = Paragraph::new("(empty transcript)").style(theme::EMPTY_STATE);
        f.render_widget(p, area);
        return;
    }

    let inner_height = area.height as usize;
    let total = items.len();

    // Calculate visible range
    let scroll_offset = if app.follow_mode && !app.viewing_subagent {
        total.saturating_sub(inner_height)
    } else if app.viewing_subagent {
        // For subagent transcripts, start at top (no follow mode)
        0
    } else {
        app.transcript_scroll
            .min(total.saturating_sub(inner_height))
    };

    let visible_end = (scroll_offset + inner_height).min(total);

    let lines: Vec<Line> = items[scroll_offset..visible_end]
        .iter()
        .map(|item| {
            let time_str = item
                .timestamp
                .map(|ts| ts.format("%H:%M").to_string())
                .unwrap_or_else(|| "     ".to_string());

            let kind_style = match item.kind {
                TranscriptItemKind::User => theme::TX_USER,
                TranscriptItemKind::Assistant => theme::TX_ASSISTANT,
                TranscriptItemKind::ToolUse => theme::TX_TOOL,
                TranscriptItemKind::ToolResult => theme::TX_RESULT,
                TranscriptItemKind::System => theme::TX_SYSTEM,
                TranscriptItemKind::Progress => theme::TX_PROGRESS,
                TranscriptItemKind::Other => theme::TX_PROGRESS,
            };

            // Truncate text to fit
            let available_width = area.width.saturating_sub(14) as usize;
            let text = truncate_chars(&item.text, available_width);
            // Replace newlines with spaces for single-line display
            let text = text.replace('\n', " ").replace('\r', "");

            Line::from(vec![
                Span::raw(format!("{} ", time_str)),
                Span::styled(format!("{} ", item.kind.label()), kind_style),
                Span::raw(text),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, area);
}
