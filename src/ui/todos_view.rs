use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use super::theme;
use super::util::truncate_chars;
use crate::app::App;

pub fn draw_todos(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    draw_todo_file_list(f, chunks[0], app);
    draw_todo_items(f, chunks[1], app);
}

fn draw_todo_file_list(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.todos_pane_left;
    let border_style = if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let title = format!(" Todo Files (non-empty) [{}] ", app.todo_files.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.todo_files.is_empty() {
        let msg = Paragraph::new("No todo files found.")
            .style(theme::EMPTY_STATE)
            .block(block)
            .wrap(Wrap { trim: false });
        f.render_widget(msg, area);
        return;
    }

    let items: Vec<ListItem> = app
        .todo_files
        .iter()
        .enumerate()
        .map(|(i, tf)| {
            let prefix = if i == app.todo_file_index { ">" } else { " " };
            let text = format!(
                "{} {} ({} items)",
                prefix,
                tf.display_name(),
                tf.items.len()
            );
            ListItem::new(text)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.todo_file_index));

    let list = List::new(items)
        .block(block)
        .highlight_style(theme::LIST_SELECTED);

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_todo_items(f: &mut Frame, area: Rect, app: &App) {
    let is_active = !app.todos_pane_left;
    let border_style = if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let items = app.current_todo_items();
    let title = format!(" Items [{}] ", items.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if items.is_empty() {
        let msg = Paragraph::new("No items")
            .style(theme::EMPTY_STATE)
            .block(block);
        f.render_widget(msg, area);
        return;
    }

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let prefix = if i == app.todo_item_index { ">" } else { " " };
            let status_style = match item.status.as_deref() {
                Some("completed") => theme::TASK_COMPLETED,
                Some("in_progress") => theme::TASK_IN_PROGRESS,
                _ => theme::TASK_PENDING,
            };

            let text = item.display_text();
            // Truncate to fit
            let max_len = area.width.saturating_sub(10) as usize;
            let display = truncate_chars(&text, max_len);

            let line = Line::from(vec![
                Span::raw(format!("{} ", prefix)),
                Span::styled(format!("{} ", item.status_icon()), status_style),
                Span::raw(display),
            ]);

            ListItem::new(line)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.todo_item_index));

    let list = List::new(list_items)
        .block(block)
        .highlight_style(theme::LIST_SELECTED);

    f.render_stateful_widget(list, area, &mut state);
}
