pub mod filebrowser_view;
pub mod git_view;
pub mod github_view;
pub mod help_overlay;
pub mod issues_view;
pub mod jira_view;
pub mod layout;
pub mod linear_view;
pub mod plans_view;
pub mod processes_view;
pub mod prompt_modal;
pub mod sessions_view;
pub mod tabs;
pub mod teams_view;
pub mod theme;
pub mod todos_view;

use ratatui::Frame;

use crate::app::App;

/// Main draw dispatcher.
pub fn draw(f: &mut Frame, app: &App) {
    layout::draw_layout(f, app);
}
