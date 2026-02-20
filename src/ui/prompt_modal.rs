use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::theme;
use crate::app::App;

/// Draw the prompt editor modal overlay.
pub fn draw_prompt_modal(f: &mut Frame, area: Rect, app: &App) {
    // Use most of the screen for the editor
    let width = (area.width - 4).min(120);
    let height = (area.height - 4).min(40);

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

    // Clear background
    f.render_widget(Clear, popup_area);

    // Split into title bar, editor area, and hint bar
    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // title
            Constraint::Min(3),    // editor
            Constraint::Length(2), // hints
        ])
        .split(popup_area);

    let title_area = inner_chunks[0];
    let editor_area = inner_chunks[1];
    let hint_area = inner_chunks[2];

    // Title
    let ticket_label = if let Some(ref ticket) = app.prompt_ticket_info {
        format!("{} - {}", ticket.key, ticket.title)
    } else {
        "Prompt Editor".to_string()
    };

    let title_block = Block::default()
        .title(format!(" Launch Claude: {} ", ticket_label))
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
        .border_style(theme::PROMPT_MODAL_BORDER);
    let title_text = Paragraph::new("").block(title_block);
    f.render_widget(title_text, title_area);

    // Editor
    if let Some(ref editor) = app.prompt_editor {
        let editor_block = Block::default()
            .borders(Borders::LEFT | Borders::RIGHT)
            .border_style(theme::PROMPT_MODAL_BORDER);

        let mut editor_clone = editor.clone();
        editor_clone.set_block(editor_block);
        editor_clone.set_cursor_line_style(theme::PROMPT_CURSOR_LINE);
        editor_clone.set_style(theme::PROMPT_EDITOR_TEXT);

        f.render_widget(&editor_clone, editor_area);
    }

    // Hints at bottom
    let hints = Line::from(vec![
        Span::styled(" Ctrl+Enter", theme::HELP_KEY),
        Span::styled(": Launch  ", theme::HELP_DESC),
        Span::styled("Esc", theme::HELP_KEY),
        Span::styled(": Cancel ", theme::HELP_DESC),
    ]);
    let hint_block = Block::default()
        .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
        .border_style(theme::PROMPT_MODAL_BORDER);
    let hint_paragraph = Paragraph::new(hints).block(hint_block);
    f.render_widget(hint_paragraph, hint_area);
}
