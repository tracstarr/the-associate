use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::theme;

pub fn draw_help(f: &mut Frame, area: Rect) {
    // Center a box
    let width = 60u16.min(area.width.saturating_sub(4));
    let height = 38u16.min(area.height.saturating_sub(4));

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

    let bindings = [
        ("Tab / Shift+Tab", "Cycle tabs"),
        ("1-9", "Jump to tab by number"),
        ("j/k or Up/Down", "Navigate list / scroll"),
        ("h/l or Left/Right", "Switch panes"),
        ("Enter", "Select / open / open browser (Linear)"),
        ("g / G", "Jump to top / bottom"),
        ("f", "Toggle follow mode (Sessions)"),
        ("o", "Open session in new WT pane (Sessions)"),
        ("s", "Cycle subagent transcripts (Sessions)"),
        ("b", "Toggle file browser (Git tab)"),
        ("e", "Edit file (browser) / issue (Issues)"),
        ("Ctrl+S", "Save edit"),
        ("Backspace", "Collapse / go to parent (browser)"),
        ("n", "New issue (Issues tab)"),
        ("c", "Comment on issue (Issues tab)"),
        (
            "x",
            "Kill process (Processes tab) / Close/reopen issue (Issues)",
        ),
        ("o", "Open in browser (PRs / Issues / Jira / Linear)"),
        ("r", "Refresh (PRs / Issues / Jira / Linear)"),
        ("t", "Show transitions (Jira)"),
        ("/", "Search (Jira)"),
        (
            "p",
            "Launch Claude Code prompt (PRs / Issues / Linear / Jira)",
        ),
        ("s", "Jump to session (Processes tab)"),
        ("d / Del", "Delete file (Sessions/Teams/Todos/Plans)"),
        ("i", "Send input to Claude pane"),
        ("? / Ctrl-H", "Toggle this help"),
        ("q / Ctrl+C", "Quit"),
    ];

    let mut lines = vec![
        Line::from(Span::styled(" Keybindings", theme::HELP_TITLE)),
        Line::from(""),
    ];

    for (key, desc) in &bindings {
        lines.push(Line::from(vec![
            Span::styled(format!("  {:20}", key), theme::HELP_KEY),
            Span::styled(*desc, theme::HELP_DESC),
        ]));
    }

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(theme::BORDER_ACTIVE);

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, popup_area);
}
