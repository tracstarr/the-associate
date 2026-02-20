use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use super::{filebrowser_view, theme};
use super::util::truncate_chars;
use crate::app::{App, GitMode, GitPane};
use crate::model::git::{DiffLineKind, FlatGitItem, GitFileSection};

pub fn draw_git(f: &mut Frame, area: Rect, app: &App) {
    if app.git_mode == GitMode::Browse {
        filebrowser_view::draw_filebrowser(f, area, app);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    draw_file_list(f, chunks[0], app);
    draw_diff_pane(f, chunks[1], app);
}

fn draw_file_list(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.git_pane == GitPane::Files;
    let border_style = if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let title = format!(" Files [{}] ", app.git_status.total_files());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.git_flat_list.is_empty() {
        // Detect whether this is a git repo by checking if .git exists
        let git_dir = app.project_cwd.join(".git");
        let msg = if git_dir.exists() {
            "Working tree clean"
        } else {
            "Not a git repository"
        };
        let p = Paragraph::new(msg)
            .style(theme::EMPTY_STATE)
            .block(block)
            .wrap(Wrap { trim: false });
        f.render_widget(p, area);
        return;
    }

    let items: Vec<ListItem> = app
        .git_flat_list
        .iter()
        .enumerate()
        .map(|(i, item)| match item {
            FlatGitItem::SectionHeader(label, section) => {
                let style = match section {
                    GitFileSection::Staged => theme::GIT_STAGED,
                    GitFileSection::Unstaged => theme::GIT_UNSTAGED,
                    GitFileSection::Untracked => theme::GIT_UNTRACKED,
                };
                ListItem::new(Line::from(Span::styled(label.clone(), style)))
            }
            FlatGitItem::File(entry) => {
                let status_style = match entry.section {
                    GitFileSection::Staged => theme::GIT_STAGED,
                    GitFileSection::Unstaged => theme::GIT_UNSTAGED,
                    GitFileSection::Untracked => theme::GIT_UNTRACKED,
                };
                let prefix = if i == app.git_file_index { ">" } else { " " };
                let line = Line::from(vec![
                    Span::raw(format!("{} ", prefix)),
                    Span::styled(format!("[{}] ", entry.status_char), status_style),
                    Span::raw(&entry.path),
                ]);
                ListItem::new(line)
            }
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.git_file_index));

    let list = List::new(items)
        .block(block)
        .highlight_style(theme::LIST_SELECTED);

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_diff_pane(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.git_pane == GitPane::Diff;
    let border_style = if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    // Title shows selected filename
    let title = if let Some(FlatGitItem::File(entry)) = app.git_flat_list.get(app.git_file_index) {
        format!(" {} ", entry.path)
    } else {
        " Diff ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.git_diff_lines.is_empty() {
        let p = Paragraph::new("Select a file to view diff")
            .style(theme::EMPTY_STATE)
            .block(block);
        f.render_widget(p, area);
        return;
    }

    let inner = block.inner(area);
    f.render_widget(block, area);

    let inner_height = inner.height as usize;
    let total = app.git_diff_lines.len();

    let scroll_offset = app.diff_scroll.min(total.saturating_sub(inner_height));
    let visible_end = (scroll_offset + inner_height).min(total);

    let lines: Vec<Line> = app.git_diff_lines[scroll_offset..visible_end]
        .iter()
        .map(|dl| {
            let style = match dl.kind {
                DiffLineKind::Add => theme::DIFF_ADD,
                DiffLineKind::Remove => theme::DIFF_REMOVE,
                DiffLineKind::Hunk => theme::DIFF_HUNK,
                DiffLineKind::Header => theme::DIFF_HEADER,
                DiffLineKind::Context => theme::LIST_NORMAL,
            };
            // Truncate to available width
            let available = inner.width as usize;
            let text = truncate_chars(&dl.text, available);
            Line::from(Span::styled(text, style))
        })
        .collect();

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);
}
