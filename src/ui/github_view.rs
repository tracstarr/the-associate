use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use super::theme;
use crate::app::{App, GitHubPane};
use crate::model::github::FlatPrItem;

pub fn draw_github(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    draw_pr_list(f, chunks[0], app);
    draw_pr_detail(f, chunks[1], app);
}

fn draw_pr_list(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.gh_pane == GitHubPane::List;
    let border_style = if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let title = format!(" Pull Requests [{}] ", app.gh_prs.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.gh_flat_list.is_empty() {
        let p = Paragraph::new("No open PRs")
            .style(theme::EMPTY_STATE)
            .block(block);
        f.render_widget(p, area);
        return;
    }

    let items: Vec<ListItem> = app
        .gh_flat_list
        .iter()
        .map(|item| match item {
            FlatPrItem::SectionHeader(label) => {
                ListItem::new(Line::from(Span::styled(label.clone(), theme::PR_SECTION)))
            }
            FlatPrItem::Pr(pr) => {
                let icon = pr.review_icon();
                let icon_style = match icon {
                    "[+]" => theme::PR_APPROVED,
                    "[!]" => theme::PR_CHANGES_REQUESTED,
                    "[?]" => theme::PR_PENDING_REVIEW,
                    _ => theme::LIST_NORMAL,
                };

                let size = pr.size_label();

                let line = if pr.is_draft {
                    Line::from(vec![
                        Span::styled(format!("{} ", icon), theme::PR_DRAFT),
                        Span::styled(format!("#{} {}  ", pr.number, pr.title), theme::PR_DRAFT),
                        Span::styled(size, theme::PR_DRAFT),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(format!("{} ", icon), icon_style),
                        Span::styled(format!("#{} {}  ", pr.number, pr.title), theme::LIST_NORMAL),
                        Span::styled(size, theme::PR_SIZE),
                    ])
                };

                ListItem::new(line)
            }
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.gh_pr_index));

    let list = List::new(items)
        .block(block)
        .highlight_style(theme::LIST_SELECTED);

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_pr_detail(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.gh_pane == GitHubPane::Detail;
    let border_style = if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    // Find the selected PR (skip section headers)
    let selected_pr = if !app.gh_flat_list.is_empty() {
        let idx = app.gh_pr_index.min(app.gh_flat_list.len() - 1);
        match &app.gh_flat_list[idx] {
            FlatPrItem::Pr(pr) => Some(pr),
            _ => None,
        }
    } else {
        None
    };

    let title = if let Some(pr) = &selected_pr {
        format!(" PR #{} ", pr.number)
    } else {
        " PR Detail ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let Some(pr) = selected_pr else {
        let p = Paragraph::new("Select a PR to view details")
            .style(theme::EMPTY_STATE)
            .block(block);
        f.render_widget(p, area);
        return;
    };

    let mut lines: Vec<Line> = Vec::new();

    // Title
    lines.push(Line::from(Span::styled(
        format!("Title: {}", pr.title),
        theme::LIST_NORMAL.add_modifier(Modifier::BOLD),
    )));

    // Author
    let author_text = if let Some(ref name) = pr.author.name {
        format!("Author: {} ({})", pr.author.login, name)
    } else {
        format!("Author: {}", pr.author.login)
    };
    lines.push(Line::from(Span::styled(author_text, theme::LIST_NORMAL)));

    // Branch
    lines.push(Line::from(Span::styled(
        format!("Branch: {} -> {}", pr.head_ref_name, pr.base_ref_name),
        theme::BRANCH_LABEL,
    )));

    // Status
    let status_text = if pr.is_draft {
        format!("Status: {} DRAFT", pr.state)
    } else {
        format!("Status: {}", pr.state)
    };
    let status_line = if pr.is_draft {
        Line::from(vec![
            Span::styled(format!("Status: {} ", pr.state), theme::LIST_NORMAL),
            Span::styled("DRAFT", theme::PR_DRAFT),
        ])
    } else {
        Line::from(Span::styled(status_text, theme::LIST_NORMAL))
    };
    lines.push(status_line);

    // Review
    let review_text = pr.review_decision.as_deref().unwrap_or("None").to_string();
    let review_style = match pr.review_decision.as_deref() {
        Some("APPROVED") => theme::PR_APPROVED,
        Some("CHANGES_REQUESTED") => theme::PR_CHANGES_REQUESTED,
        Some("REVIEW_REQUIRED") => theme::PR_PENDING_REVIEW,
        _ => theme::LIST_NORMAL,
    };
    lines.push(Line::from(vec![
        Span::styled("Review: ", theme::LIST_NORMAL),
        Span::styled(review_text, review_style),
    ]));

    // Size
    lines.push(Line::from(vec![
        Span::styled(format!("Size: {} (", pr.size_label()), theme::LIST_NORMAL),
        Span::styled(format!("+{}", pr.additions), theme::DIFF_ADD),
        Span::styled(" ", theme::LIST_NORMAL),
        Span::styled(format!("-{}", pr.deletions), theme::DIFF_REMOVE),
        Span::styled(")", theme::LIST_NORMAL),
    ]));

    // Created
    lines.push(Line::from(Span::styled(
        format!("Created: {}", pr.created_at),
        theme::LIST_NORMAL,
    )));

    // Updated
    lines.push(Line::from(Span::styled(
        format!("Updated: {}", pr.updated_at),
        theme::LIST_NORMAL,
    )));

    // Labels
    if !pr.labels.is_empty() {
        let label_names: Vec<&str> = pr.labels.iter().map(|l| l.name.as_str()).collect();
        lines.push(Line::from(Span::styled(
            format!("Labels: {}", label_names.join(", ")),
            theme::LIST_NORMAL,
        )));
    }

    // Assignees
    if !pr.assignees.is_empty() {
        let assignee_names: Vec<&str> = pr.assignees.iter().map(|a| a.login.as_str()).collect();
        lines.push(Line::from(Span::styled(
            format!("Assignees: {}", assignee_names.join(", ")),
            theme::LIST_NORMAL,
        )));
    }

    // Blank line
    lines.push(Line::from(""));

    // URL
    lines.push(Line::from(Span::styled(
        format!("URL: {}", pr.url),
        theme::LIST_NORMAL,
    )));

    // Apply scroll offset
    let inner = block.inner(area);
    f.render_widget(block, area);

    let inner_height = inner.height as usize;
    let total = lines.len();
    let scroll_offset = app.gh_detail_scroll.min(total.saturating_sub(inner_height));
    let visible_end = (scroll_offset + inner_height).min(total);

    let visible_lines: Vec<Line> = lines[scroll_offset..visible_end].to_vec();
    let paragraph = Paragraph::new(visible_lines);
    f.render_widget(paragraph, inner);
}
