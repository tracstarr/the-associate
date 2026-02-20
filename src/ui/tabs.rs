use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::theme;
use crate::app::{ActiveTab, App};

pub fn draw_tab_bar(f: &mut Frame, area: Rect, app: &App) {
    let visible = app.visible_tabs();

    let mut spans = Vec::new();
    for (i, tab) in visible.iter().enumerate() {
        let num = i + 1;
        let label = match tab {
            ActiveTab::Sessions => format!("{}:Sessions", num),
            ActiveTab::Teams => format!("{}:Teams", num),
            ActiveTab::Todos => format!("{}:Todos", num),
            ActiveTab::Git => format!("{}:Git", num),
            ActiveTab::Plans => format!("{}:Plans", num),
            ActiveTab::GitHubPRs => {
                if app.gh_new_activity {
                    format!("{}:PRs*", num)
                } else {
                    format!("{}:PRs", num)
                }
            }
            ActiveTab::Jira => format!("{}:Jira", num),
        };

        let style = if *tab == app.active_tab {
            theme::TAB_ACTIVE
        } else if *tab == ActiveTab::GitHubPRs && app.gh_new_activity {
            theme::PR_BADGE
        } else {
            theme::TAB_INACTIVE
        };
        spans.push(Span::styled(format!(" {} ", label), style));
        spans.push(Span::raw(" "));
    }

    // Version on the right
    let version = format!("The Associate v{}", env!("CARGO_PKG_VERSION"));
    let tabs_width: usize = spans.iter().map(|s| s.width()).sum();
    let total_used = tabs_width + version.len();
    let pad = (area.width as usize).saturating_sub(total_used);
    if pad > 0 {
        spans.push(Span::raw(" ".repeat(pad)));
    }
    spans.push(Span::styled(version, theme::STATUS_BAR));

    let line = Line::from(spans);
    f.render_widget(Paragraph::new(line), area);
}
