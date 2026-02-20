use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use super::theme;
use crate::app::{App, PlansPane};
use crate::model::plan::MarkdownLineKind;

fn truncate_chars(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

pub fn draw_plans(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    draw_plan_list(f, chunks[0], app);
    draw_plan_content(f, chunks[1], app);
}

fn draw_plan_list(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.plans_pane == PlansPane::List;
    let border_style = if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let title = format!(" Plans [{}] ", app.plan_files.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.plan_files.is_empty() {
        let home = app.claude_home.join("plans");
        let msg = format!("No plans found in\n{}", home.display());
        let p = Paragraph::new(msg)
            .style(theme::EMPTY_STATE)
            .block(block)
            .wrap(Wrap { trim: false });
        f.render_widget(p, area);
        return;
    }

    let items: Vec<ListItem> = app
        .plan_files
        .iter()
        .enumerate()
        .map(|(i, plan)| {
            let prefix = if i == app.plan_file_index { ">" } else { " " };
            let line = Line::from(vec![
                Span::raw(format!("{} ", prefix)),
                Span::raw(&plan.title),
            ]);
            ListItem::new(line)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.plan_file_index));

    let list = List::new(items)
        .block(block)
        .highlight_style(theme::LIST_SELECTED);

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_plan_content(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.plans_pane == PlansPane::Content;
    let border_style = if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let title = if !app.plan_files.is_empty() {
        let idx = app.plan_file_index.min(app.plan_files.len() - 1);
        format!(" {} ", app.plan_files[idx].display_name())
    } else {
        " Content ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let lines = app.current_plan_lines();
    if lines.is_empty() {
        let p = Paragraph::new("Select a plan to view")
            .style(theme::EMPTY_STATE)
            .block(block);
        f.render_widget(p, area);
        return;
    }

    let inner = block.inner(area);
    f.render_widget(block, area);

    let inner_height = inner.height as usize;
    let total = lines.len();

    let scroll_offset = app
        .plan_content_scroll
        .min(total.saturating_sub(inner_height));
    let visible_end = (scroll_offset + inner_height).min(total);

    let rendered: Vec<Line> = lines[scroll_offset..visible_end]
        .iter()
        .map(|ml| {
            let style = match ml.kind {
                MarkdownLineKind::Heading => theme::MD_HEADING,
                MarkdownLineKind::CodeFence => theme::MD_CODE_FENCE,
                MarkdownLineKind::CodeBlock => theme::MD_CODE_BLOCK,
                MarkdownLineKind::Normal => theme::MD_NORMAL,
            };
            let available = inner.width as usize;
            let text = truncate_chars(&ml.text, available);
            Line::from(Span::styled(text, style))
        })
        .collect();

    let paragraph = Paragraph::new(rendered);
    f.render_widget(paragraph, inner);
}
