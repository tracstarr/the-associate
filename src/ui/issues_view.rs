use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use super::theme;
use crate::app::{App, IssueEditField, IssueEditMode, IssuesPane};
use crate::model::github::FlatIssueItem;

pub fn draw_issues(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    draw_issue_list(f, chunks[0], app);
    draw_issue_detail(f, chunks[1], app);

    if app.gh_issues_editing {
        draw_edit_popup(f, area, app);
    }
}

fn draw_issue_list(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.gh_issues_pane == IssuesPane::List;
    let border_style = if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let title = format!(" Issues [{}] ", app.gh_issues.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.gh_issues_flat_list.is_empty() {
        let p = Paragraph::new("No issues found")
            .style(theme::EMPTY_STATE)
            .block(block);
        f.render_widget(p, area);
        return;
    }

    let items: Vec<ListItem> = app
        .gh_issues_flat_list
        .iter()
        .map(|item| match item {
            FlatIssueItem::SectionHeader(label) => {
                let style = if label.contains("Current Issue") {
                    theme::CURRENT_ISSUE_HEADER
                } else {
                    theme::ISSUE_SECTION
                };
                ListItem::new(Line::from(Span::styled(label.clone(), style)))
            }
            FlatIssueItem::Issue(issue) => {
                let is_current = app.is_current_github_issue(issue.number);

                let icon = issue.state_icon();
                let icon_style = if is_current {
                    theme::CURRENT_ISSUE
                } else if issue.state == "OPEN" {
                    theme::ISSUE_OPEN
                } else {
                    theme::ISSUE_CLOSED
                };

                let text_style = if is_current {
                    theme::CURRENT_ISSUE
                } else {
                    theme::LIST_NORMAL
                };

                let mut spans = vec![
                    Span::styled(format!("{} ", icon), icon_style),
                    Span::styled(
                        format!("#{} ", issue.number),
                        text_style.add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(&issue.title, text_style),
                ];

                if !issue.labels.is_empty() {
                    let label_text: Vec<&str> =
                        issue.labels.iter().map(|l| l.name.as_str()).collect();
                    spans.push(Span::styled(
                        format!("  [{}]", label_text.join(",")),
                        if is_current {
                            theme::CURRENT_ISSUE
                        } else {
                            theme::ISSUE_LABEL
                        },
                    ));
                }

                ListItem::new(Line::from(spans))
            }
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.gh_issues_index));

    let list = List::new(items)
        .block(block)
        .highlight_style(theme::LIST_SELECTED);

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_issue_detail(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.gh_issues_pane == IssuesPane::Detail;
    let border_style = if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let selected = app.issues_selected();

    let title = if let Some(issue) = &selected {
        format!(" Issue #{} ", issue.number)
    } else {
        " Issue Detail ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let Some(issue) = selected else {
        let p = Paragraph::new("Select an issue to view details")
            .style(theme::EMPTY_STATE)
            .block(block);
        f.render_widget(p, area);
        return;
    };

    let mut lines: Vec<Line> = Vec::new();

    // Title
    lines.push(Line::from(Span::styled(
        &issue.title,
        theme::LIST_NORMAL.add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    // State
    let state_style = if issue.state == "OPEN" {
        theme::ISSUE_OPEN
    } else {
        theme::ISSUE_CLOSED
    };
    lines.push(Line::from(vec![
        Span::styled("State: ", theme::LIST_NORMAL.add_modifier(Modifier::BOLD)),
        Span::styled(&issue.state, state_style),
    ]));

    // Author
    lines.push(Line::from(vec![
        Span::styled("Author: ", theme::LIST_NORMAL.add_modifier(Modifier::BOLD)),
        Span::raw(&issue.author.login),
    ]));

    // Assignees
    if !issue.assignees.is_empty() {
        let names: Vec<&str> = issue.assignees.iter().map(|a| a.login.as_str()).collect();
        lines.push(Line::from(vec![
            Span::styled(
                "Assignees: ",
                theme::LIST_NORMAL.add_modifier(Modifier::BOLD),
            ),
            Span::raw(names.join(", ")),
        ]));
    }

    // Labels
    if !issue.labels.is_empty() {
        let label_names: Vec<&str> = issue.labels.iter().map(|l| l.name.as_str()).collect();
        lines.push(Line::from(vec![
            Span::styled("Labels: ", theme::LIST_NORMAL.add_modifier(Modifier::BOLD)),
            Span::styled(label_names.join(", "), theme::ISSUE_LABEL),
        ]));
    }

    // Milestone
    if let Some(ref milestone) = issue.milestone {
        lines.push(Line::from(vec![
            Span::styled(
                "Milestone: ",
                theme::LIST_NORMAL.add_modifier(Modifier::BOLD),
            ),
            Span::raw(&milestone.title),
        ]));
    }

    // Dates
    lines.push(Line::from(vec![
        Span::styled("Created: ", theme::LIST_NORMAL.add_modifier(Modifier::BOLD)),
        Span::raw(&issue.created_at),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Updated: ", theme::LIST_NORMAL.add_modifier(Modifier::BOLD)),
        Span::raw(&issue.updated_at),
    ]));

    lines.push(Line::from(""));

    // Body
    lines.push(Line::from(Span::styled(
        "Description:",
        theme::LIST_NORMAL.add_modifier(Modifier::BOLD),
    )));

    match issue.body.as_deref() {
        Some(body) if !body.is_empty() => {
            for line in body.lines() {
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

    // Comments
    if !issue.comments.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("Comments ({}):", issue.comments.len()),
            theme::LIST_NORMAL.add_modifier(Modifier::BOLD),
        )));

        for comment in &issue.comments {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {} ", comment.author.login),
                    theme::ISSUE_COMMENT_AUTHOR,
                ),
                Span::styled(&comment.created_at, theme::EMPTY_STATE),
            ]));
            for cline in comment.body.lines() {
                lines.push(Line::from(Span::raw(format!("  {}", cline))));
            }
        }
    }

    // URL
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("URL: ", theme::LIST_NORMAL.add_modifier(Modifier::BOLD)),
        Span::raw(&issue.url),
    ]));

    // Render with scroll
    let inner = block.inner(area);
    f.render_widget(block, area);

    let inner_height = inner.height as usize;
    let total = lines.len();
    let scroll_offset = app
        .gh_issues_detail_scroll
        .min(total.saturating_sub(inner_height));
    let visible_end = (scroll_offset + inner_height).min(total);

    let visible_lines: Vec<Line> = lines[scroll_offset..visible_end].to_vec();
    let paragraph = Paragraph::new(visible_lines).wrap(Wrap { trim: false });
    f.render_widget(paragraph, inner);
}

fn draw_edit_popup(f: &mut Frame, area: Rect, app: &App) {
    let width = 70u16.min(area.width.saturating_sub(6));
    let height = 24u16.min(area.height.saturating_sub(4));

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
    f.render_widget(Clear, popup_area);

    let dynamic_title = match &app.gh_issues_edit_mode {
        Some(IssueEditMode::Edit(n)) => format!(" Edit Issue #{} ", n),
        Some(IssueEditMode::Comment(n)) => format!(" Comment on #{} ", n),
        Some(IssueEditMode::Create) => " New Issue ".to_string(),
        None => " Editor ".to_string(),
    };

    let block = Block::default()
        .title(dynamic_title)
        .borders(Borders::ALL)
        .border_style(theme::BORDER_ACTIVE)
        .style(
            ratatui::style::Style::new()
                .fg(ratatui::style::Color::White)
                .bg(ratatui::style::Color::Black),
        );

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let is_comment = matches!(app.gh_issues_edit_mode, Some(IssueEditMode::Comment(_)));

    if is_comment {
        // Comment mode: only body field
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);

        // Body editor
        if let Some(ref editor) = app.gh_issues_body_editor {
            let body_block = Block::default()
                .title(" Comment (Ctrl+S save, Esc cancel) ")
                .borders(Borders::ALL)
                .border_style(theme::BORDER_ACTIVE);
            let body_inner = body_block.inner(chunks[0]);
            f.render_widget(body_block, chunks[0]);
            f.render_widget(editor, body_inner);
        }

        // Hint
        let hint = Line::from(Span::styled(" Ctrl+S: save  Esc: cancel", theme::HINT_DESC));
        f.render_widget(Paragraph::new(hint), chunks[1]);
    } else {
        // Create/Edit mode: title + body fields
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(1),    // Body
                Constraint::Length(1), // Hint
            ])
            .split(inner);

        // Title editor
        let title_active = app.gh_issues_edit_field == IssueEditField::Title;
        let title_border = if title_active {
            theme::BORDER_ACTIVE
        } else {
            theme::BORDER_INACTIVE
        };
        let title_block = Block::default()
            .title(" Title ")
            .borders(Borders::ALL)
            .border_style(title_border);

        if let Some(ref editor) = app.gh_issues_title_editor {
            let title_inner = title_block.inner(chunks[0]);
            f.render_widget(title_block, chunks[0]);
            f.render_widget(editor, title_inner);
        }

        // Body editor
        let body_active = app.gh_issues_edit_field == IssueEditField::Body;
        let body_border = if body_active {
            theme::BORDER_ACTIVE
        } else {
            theme::BORDER_INACTIVE
        };
        let body_block = Block::default()
            .title(" Body ")
            .borders(Borders::ALL)
            .border_style(body_border);

        if let Some(ref editor) = app.gh_issues_body_editor {
            let body_inner = body_block.inner(chunks[1]);
            f.render_widget(body_block, chunks[1]);
            f.render_widget(editor, body_inner);
        }

        // Hint
        let hint = Line::from(Span::styled(
            " Tab: switch field  Ctrl+S: save  Esc: cancel",
            theme::HINT_DESC,
        ));
        f.render_widget(Paragraph::new(hint), chunks[2]);
    }
}
