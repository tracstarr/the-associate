use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use super::theme;
use crate::app::{App, FileBrowserPane};
use crate::model::filebrowser::{EntryKind, FileContent};
use crate::model::plan::MarkdownLineKind;

fn truncate_chars(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

pub fn draw_filebrowser(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    draw_tree_pane(f, chunks[0], app);
    draw_content_pane(f, chunks[1], app);
}

fn draw_tree_pane(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.fb_pane == FileBrowserPane::Tree;
    let border_style = if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let block = Block::default()
        .title(" Files ")
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.fb_entries.is_empty() {
        let p = Paragraph::new("No files")
            .style(theme::EMPTY_STATE)
            .block(block);
        f.render_widget(p, area);
        return;
    }

    let items: Vec<ListItem> = app
        .fb_entries
        .iter()
        .map(|entry| {
            let indent = "  ".repeat(entry.depth);
            let (prefix, style) = match entry.kind {
                EntryKind::Directory => {
                    let expanded = app.fb_expanded.contains(&entry.path);
                    let arrow = if expanded { "v " } else { "> " };
                    (arrow, theme::FB_DIR)
                }
                EntryKind::File => ("  ", theme::FB_FILE),
            };

            let line = Line::from(vec![
                Span::raw(indent),
                Span::styled(prefix, style),
                Span::styled(&entry.name, style),
            ]);
            ListItem::new(line)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.fb_index));

    let list = List::new(items)
        .block(block)
        .highlight_style(theme::LIST_SELECTED);

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_content_pane(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.fb_pane == FileBrowserPane::Content;

    let border_style = if app.fb_editing {
        theme::FB_EDIT_BORDER
    } else if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let title = if let Some(ref path) = app.fb_content_path {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "File".to_string());
        format!(" {} ", name)
    } else {
        " Content ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    // Edit mode: render the TextArea widget
    if app.fb_editing {
        if let Some(ref editor) = app.fb_editor {
            let inner = block.inner(area);
            f.render_widget(block, area);
            f.render_widget(editor, inner);
        } else {
            let p = Paragraph::new("Editor not initialized")
                .style(theme::EMPTY_STATE)
                .block(block);
            f.render_widget(p, area);
        }
        return;
    }

    match app.fb_content {
        Some(FileContent::Text(ref lines)) => {
            let inner = block.inner(area);
            f.render_widget(block, area);

            let inner_height = inner.height as usize;
            let total = lines.len();
            let scroll_offset = app
                .fb_content_scroll
                .min(total.saturating_sub(inner_height));
            let visible_end = (scroll_offset + inner_height).min(total);

            let available = inner.width as usize;
            // Reserve space for line numbers (digits + 1 space)
            let num_width = if total > 0 {
                format!("{}", total).len()
            } else {
                1
            };
            let text_width = available.saturating_sub(num_width + 1);

            let rendered: Vec<Line> = lines[scroll_offset..visible_end]
                .iter()
                .enumerate()
                .map(|(i, line_text)| {
                    let line_num = scroll_offset + i + 1;
                    let num_str = format!("{:>width$} ", line_num, width = num_width);
                    let text = truncate_chars(line_text, text_width);
                    Line::from(vec![
                        Span::styled(num_str, theme::FB_LINE_NUMBER),
                        Span::styled(text, theme::LIST_NORMAL),
                    ])
                })
                .collect();

            let paragraph = Paragraph::new(rendered);
            f.render_widget(paragraph, inner);
        }
        Some(FileContent::Markdown(ref md_lines)) => {
            let inner = block.inner(area);
            f.render_widget(block, area);

            let inner_height = inner.height as usize;
            let total = md_lines.len();
            let scroll_offset = app
                .fb_content_scroll
                .min(total.saturating_sub(inner_height));
            let visible_end = (scroll_offset + inner_height).min(total);
            let available = inner.width as usize;

            let rendered: Vec<Line> = md_lines[scroll_offset..visible_end]
                .iter()
                .map(|ml| {
                    let style = match ml.kind {
                        MarkdownLineKind::Heading => theme::MD_HEADING,
                        MarkdownLineKind::CodeFence => theme::MD_CODE_FENCE,
                        MarkdownLineKind::CodeBlock => theme::MD_CODE_BLOCK,
                        MarkdownLineKind::Normal => theme::MD_NORMAL,
                    };
                    let text = truncate_chars(&ml.text, available);
                    Line::from(Span::styled(text, style))
                })
                .collect();

            let paragraph = Paragraph::new(rendered);
            f.render_widget(paragraph, inner);
        }
        Some(FileContent::Binary) => {
            let p = Paragraph::new("Binary file - cannot display")
                .style(theme::EMPTY_STATE)
                .block(block);
            f.render_widget(p, area);
        }
        Some(FileContent::TooLarge) => {
            let p = Paragraph::new("File too large (>1MB)")
                .style(theme::EMPTY_STATE)
                .block(block);
            f.render_widget(p, area);
        }
        None => {
            let p = Paragraph::new("Select a file to view")
                .style(theme::EMPTY_STATE)
                .block(block);
            f.render_widget(p, area);
        }
    }
}
