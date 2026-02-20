use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use super::theme;
use crate::app::{App, JiraPane};
use crate::model::jira::FlatJiraItem;

pub fn draw_jira(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    draw_issue_list(f, chunks[0], app);
    draw_detail_pane(f, chunks[1], app);

    if app.jira_show_transitions {
        draw_transition_popup(f, area, app);
    }
}

fn draw_issue_list(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.jira_pane == JiraPane::List;
    let border_style = if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    // If search mode is active, split vertically to show search input at bottom
    let (list_area, search_area) = if app.jira_search_mode {
        let parts = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(area);
        (parts[0], Some(parts[1]))
    } else {
        (area, None)
    };

    let title = format!(" Issues [{}] ", app.jira_issues.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.jira_flat_list.is_empty() {
        let p = Paragraph::new("No issues found")
            .style(theme::EMPTY_STATE)
            .block(block)
            .wrap(Wrap { trim: false });
        f.render_widget(p, list_area);
    } else {
        let items: Vec<ListItem> = app
            .jira_flat_list
            .iter()
            .map(|item| match item {
                FlatJiraItem::StatusHeader(name, category) => {
                    let style = if name.contains("Current Issue") {
                        theme::CURRENT_ISSUE_HEADER
                    } else {
                        match category.as_str() {
                            "In Progress" => theme::JIRA_IN_PROGRESS,
                            "Done" => theme::JIRA_DONE,
                            _ => theme::JIRA_TODO,
                        }
                    };
                    ListItem::new(Line::from(Span::styled(name.clone(), style)))
                }
                FlatJiraItem::Issue(issue) => {
                    let is_current = app.is_current_jira_issue(&issue.key);

                    let type_style = if is_current {
                        theme::CURRENT_ISSUE
                    } else {
                        match issue.issue_type.to_lowercase().as_str() {
                            "bug" => theme::JIRA_BUG,
                            "story" => theme::JIRA_STORY,
                            "task" => theme::JIRA_TASK,
                            _ => theme::LIST_NORMAL,
                        }
                    };

                    let text_style = if is_current {
                        theme::CURRENT_ISSUE
                    } else {
                        theme::LIST_NORMAL
                    };

                    let line = Line::from(vec![
                        Span::styled(format!("  [{}] ", issue.type_icon()), type_style),
                        Span::styled(&issue.key, text_style.add_modifier(Modifier::BOLD)),
                        Span::styled(" ", text_style),
                        Span::styled(&issue.summary, text_style),
                    ]);
                    ListItem::new(line)
                }
            })
            .collect();

        let mut state = ListState::default();
        state.select(Some(app.jira_index));

        let list = List::new(items)
            .block(block)
            .highlight_style(theme::LIST_SELECTED);

        f.render_stateful_widget(list, list_area, &mut state);
    }

    // Draw search input if active
    if let Some(search_area) = search_area {
        let search_block = Block::default()
            .title(" Search (Enter to search, Esc to cancel) ")
            .borders(Borders::ALL)
            .border_style(theme::JIRA_SEARCH_INPUT);

        let search_text = format!("> {}_", app.jira_search_input);
        let p = Paragraph::new(search_text)
            .style(theme::JIRA_SEARCH_INPUT)
            .block(search_block);
        f.render_widget(p, search_area);
    }
}

fn draw_detail_pane(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.jira_pane == JiraPane::Detail;
    let border_style = if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let title = if let Some(ref detail) = app.jira_detail {
        format!(" {} ", detail.key)
    } else {
        " Detail ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let detail = match app.jira_detail {
        Some(ref d) => d,
        None => {
            let p = Paragraph::new("Select an issue to view details")
                .style(theme::EMPTY_STATE)
                .block(block);
            f.render_widget(p, area);
            return;
        }
    };

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    // Key
    lines.push(Line::from(vec![
        Span::styled("Key: ", theme::LIST_NORMAL.add_modifier(Modifier::BOLD)),
        Span::raw(&detail.key),
    ]));

    // Summary
    lines.push(Line::from(vec![
        Span::styled("Summary: ", theme::LIST_NORMAL.add_modifier(Modifier::BOLD)),
        Span::styled(
            &detail.summary,
            theme::LIST_NORMAL.add_modifier(Modifier::BOLD),
        ),
    ]));

    // Status with color
    let status_style = match detail.status_category.as_str() {
        "In Progress" => theme::JIRA_IN_PROGRESS,
        "Done" => theme::JIRA_DONE,
        _ => theme::JIRA_TODO,
    };
    lines.push(Line::from(vec![
        Span::styled("Status: ", theme::LIST_NORMAL.add_modifier(Modifier::BOLD)),
        Span::styled(&detail.status_name, status_style),
    ]));

    // Type
    lines.push(Line::from(vec![
        Span::styled("Type: ", theme::LIST_NORMAL.add_modifier(Modifier::BOLD)),
        Span::raw(&detail.issue_type),
    ]));

    // Priority
    lines.push(Line::from(vec![
        Span::styled(
            "Priority: ",
            theme::LIST_NORMAL.add_modifier(Modifier::BOLD),
        ),
        Span::raw(&detail.priority),
    ]));

    // Labels
    if !detail.labels.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Labels: ", theme::LIST_NORMAL.add_modifier(Modifier::BOLD)),
            Span::raw(detail.labels.join(", ")),
        ]));
    }

    // Blank line
    lines.push(Line::from(""));

    // Description
    lines.push(Line::from(Span::styled(
        "Description:",
        theme::LIST_NORMAL.add_modifier(Modifier::BOLD),
    )));

    match detail.description {
        Some(ref desc) if !desc.is_empty() => {
            for line in desc.lines() {
                lines.push(Line::from(Span::raw(line.to_string())));
            }
        }
        _ => {
            lines.push(Line::from(Span::styled(
                "No description",
                theme::EMPTY_STATE,
            )));
        }
    }

    // Blank line + URL
    if !detail.url.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("URL: ", theme::LIST_NORMAL.add_modifier(Modifier::BOLD)),
            Span::raw(&detail.url),
        ]));
    }

    // Apply scroll offset
    let inner_height = inner.height as usize;
    let total = lines.len();
    let scroll_offset = app
        .jira_detail_scroll
        .min(total.saturating_sub(inner_height));
    let visible_end = (scroll_offset + inner_height).min(total);

    let visible_lines: Vec<Line> = lines[scroll_offset..visible_end].to_vec();
    let paragraph = Paragraph::new(visible_lines).wrap(Wrap { trim: false });
    f.render_widget(paragraph, inner);
}

fn draw_transition_popup(f: &mut Frame, area: Rect, app: &App) {
    let width = 40u16.min(area.width.saturating_sub(4));
    let height = (app.jira_transitions.len() as u16 + 4).min(area.height.saturating_sub(4));

    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((area.height - height) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(area);

    let horiz = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((area.width - width) / 2),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(vert[1]);

    let popup_area = horiz[1];

    // Clear background behind popup
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Transitions ")
        .borders(Borders::ALL)
        .border_style(theme::HELP_TITLE)
        .style(theme::JIRA_TRANSITION_POPUP);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    for (i, transition) in app.jira_transitions.iter().enumerate() {
        lines.push(Line::from(format!("  {}. {}", i + 1, transition.name)));
    }

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, popup_area);
}
