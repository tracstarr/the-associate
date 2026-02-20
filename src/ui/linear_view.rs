use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use super::theme;
use crate::app::{App, LinearPane};
use crate::model::linear::FlatLinearItem;

pub fn draw_linear(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    draw_issue_list(f, chunks[0], app);
    draw_detail_pane(f, chunks[1], app);
}

fn draw_issue_list(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.linear_pane == LinearPane::List;
    let border_style = if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let title = format!(" Linear [{}] ", app.linear_issues.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.linear_flat_list.is_empty() {
        let p = Paragraph::new("No issues found")
            .style(theme::EMPTY_STATE)
            .block(block)
            .wrap(Wrap { trim: false });
        f.render_widget(p, area);
    } else {
        let items: Vec<ListItem> = app
            .linear_flat_list
            .iter()
            .map(|item| match item {
                FlatLinearItem::AssignmentHeader(name) => {
                    let style = if name.contains("Current Issue") {
                        theme::CURRENT_ISSUE_HEADER
                    } else {
                        theme::LINEAR_SECTION
                    };
                    ListItem::new(Line::from(Span::styled(name.clone(), style)))
                }
                FlatLinearItem::Issue(issue) => {
                    let is_current = app.is_current_linear_issue(&issue.identifier);

                    let priority_style = if is_current {
                        theme::CURRENT_ISSUE
                    } else {
                        match issue.priority {
                            1 => theme::LINEAR_URGENT,
                            2 => theme::LINEAR_HIGH,
                            3 => theme::LINEAR_MEDIUM,
                            4 => theme::LINEAR_LOW,
                            _ => theme::LIST_NORMAL,
                        }
                    };

                    let text_style = if is_current {
                        theme::CURRENT_ISSUE
                    } else {
                        theme::LIST_NORMAL
                    };

                    let line = Line::from(vec![
                        Span::styled(format!("  {} ", issue.priority_icon()), priority_style),
                        Span::styled(&issue.identifier, text_style.add_modifier(Modifier::BOLD)),
                        Span::styled(" ", text_style),
                        Span::styled(&issue.title, text_style),
                    ]);
                    ListItem::new(line)
                }
            })
            .collect();

        let mut state = ListState::default();
        state.select(Some(app.linear_index));

        let list = List::new(items)
            .block(block)
            .highlight_style(theme::LIST_SELECTED);

        f.render_stateful_widget(list, area, &mut state);
    }
}

fn draw_detail_pane(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.linear_pane == LinearPane::Detail;
    let border_style = if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let selected = app.linear_selected_issue();

    let title = if let Some(issue) = selected {
        format!(" {} ", issue.identifier)
    } else {
        " Detail ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let issue = match selected {
        Some(i) => i,
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

    // Identifier
    lines.push(Line::from(vec![
        Span::styled("ID: ", theme::LIST_NORMAL.add_modifier(Modifier::BOLD)),
        Span::raw(&issue.identifier),
    ]));

    // Title
    lines.push(Line::from(vec![
        Span::styled("Title: ", theme::LIST_NORMAL.add_modifier(Modifier::BOLD)),
        Span::styled(
            &issue.title,
            theme::LIST_NORMAL.add_modifier(Modifier::BOLD),
        ),
    ]));

    // State with color
    let state_style = match issue.state.state_type.as_str() {
        "started" => theme::LINEAR_STARTED,
        "completed" => theme::LINEAR_COMPLETED,
        _ => theme::LINEAR_UNSTARTED,
    };
    lines.push(Line::from(vec![
        Span::styled("State: ", theme::LIST_NORMAL.add_modifier(Modifier::BOLD)),
        Span::styled(&issue.state.name, state_style),
    ]));

    // Priority
    let priority_style = match issue.priority {
        1 => theme::LINEAR_URGENT,
        2 => theme::LINEAR_HIGH,
        3 => theme::LINEAR_MEDIUM,
        4 => theme::LINEAR_LOW,
        _ => theme::LIST_NORMAL,
    };
    lines.push(Line::from(vec![
        Span::styled(
            "Priority: ",
            theme::LIST_NORMAL.add_modifier(Modifier::BOLD),
        ),
        Span::styled(&issue.priority_label, priority_style),
    ]));

    // Assignee
    if let Some(ref assignee) = issue.assignee {
        lines.push(Line::from(vec![
            Span::styled(
                "Assignee: ",
                theme::LIST_NORMAL.add_modifier(Modifier::BOLD),
            ),
            Span::raw(&assignee.name),
        ]));
    }

    // Team
    if let Some(ref team) = issue.team {
        lines.push(Line::from(vec![
            Span::styled("Team: ", theme::LIST_NORMAL.add_modifier(Modifier::BOLD)),
            Span::raw(format!("{} ({})", team.name, team.key)),
        ]));
    }

    // Labels
    if !issue.labels.nodes.is_empty() {
        let label_names: Vec<&str> = issue.labels.nodes.iter().map(|l| l.name.as_str()).collect();
        lines.push(Line::from(vec![
            Span::styled("Labels: ", theme::LIST_NORMAL.add_modifier(Modifier::BOLD)),
            Span::raw(label_names.join(", ")),
        ]));
    }

    // Blank line
    lines.push(Line::from(""));

    // Description
    lines.push(Line::from(Span::styled(
        "Description:",
        theme::LIST_NORMAL.add_modifier(Modifier::BOLD),
    )));

    match issue.description {
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

    // URL
    if !issue.url.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("URL: ", theme::LIST_NORMAL.add_modifier(Modifier::BOLD)),
            Span::raw(&issue.url),
        ]));
    }

    // Apply scroll offset
    let inner_height = inner.height as usize;
    let total = lines.len();
    let scroll_offset = app
        .linear_detail_scroll
        .min(total.saturating_sub(inner_height));
    let visible_end = (scroll_offset + inner_height).min(total);

    let visible_lines: Vec<Line> = lines[scroll_offset..visible_end].to_vec();
    let paragraph = Paragraph::new(visible_lines).wrap(Wrap { trim: false });
    f.render_widget(paragraph, inner);
}
