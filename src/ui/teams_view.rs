use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use super::theme;
use super::util::truncate_chars;
use crate::app::{App, TeamsPane};
use crate::model::agent_status::AgentStatus;
use crate::model::task::TaskStatus;

pub fn draw_teams(f: &mut Frame, area: Rect, app: &App) {
    // Layout: Teams (fixed) | Members/Tasks (fixed) | Detail (fills remaining)
    // Left two columns stay pinned; detail panel grows with the window.
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Max(30),
            Constraint::Max(35),
            Constraint::Fill(1),
        ])
        .split(area);

    draw_team_list(f, chunks[0], app);

    // Middle column: show tasks list when Tasks or Detail pane is focused, members otherwise
    match app.teams_pane {
        TeamsPane::Tasks | TeamsPane::Detail => {
            draw_task_list(f, chunks[1], app);
        }
        _ => {
            draw_member_list(f, chunks[1], app);
        }
    }

    // Right column: context-sensitive detail panel
    draw_detail_panel(f, chunks[2], app);
}

fn draw_team_list(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.teams_pane == TeamsPane::Teams;
    let border_style = if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let title = format!(" Teams [{}] ", app.teams.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.teams.is_empty() {
        let msg = Paragraph::new("No teams found.")
            .style(theme::EMPTY_STATE)
            .block(block)
            .wrap(Wrap { trim: false });
        f.render_widget(msg, area);
        return;
    }

    let items: Vec<ListItem> = app
        .teams
        .iter()
        .enumerate()
        .map(|(i, team)| {
            let prefix = if i == app.team_list_index { ">" } else { " " };
            let name = team.display_name();
            ListItem::new(format!("{} {}", prefix, name))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.team_list_index));

    let list = List::new(items)
        .block(block)
        .highlight_style(theme::LIST_SELECTED);

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_member_list(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.teams_pane == TeamsPane::Members;
    let border_style = if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let members = app.current_team_members();
    let title = format!(" Members [{}] ", members.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if members.is_empty() {
        let msg = Paragraph::new("No members")
            .style(theme::EMPTY_STATE)
            .block(block);
        f.render_widget(msg, area);
        return;
    }

    // Get team config for lead check
    let team_config = if !app.teams.is_empty() {
        let idx = app.team_list_index.min(app.teams.len() - 1);
        Some(&app.teams[idx].config)
    } else {
        None
    };

    let items: Vec<ListItem> = members
        .iter()
        .enumerate()
        .map(|(i, member)| {
            let prefix = if i == app.member_list_index { ">" } else { " " };

            // Status icon
            let status = app.agent_statuses.get(&member.name);
            let (status_icon, status_style) = match status {
                Some(AgentStatus::Starting) => ("[~]", theme::AGENT_STARTING),
                Some(AgentStatus::Working) => ("[>]", theme::AGENT_WORKING),
                Some(AgentStatus::Idle) => ("[z]", theme::AGENT_IDLE),
                Some(AgentStatus::ShutDown) => ("[x]", theme::AGENT_SHUTDOWN),
                None => ("   ", theme::LIST_NORMAL),
            };

            // Lead indicator
            let is_lead = team_config.map(|cfg| member.is_lead(cfg)).unwrap_or(false);

            let name_style = if is_lead {
                theme::AGENT_LEAD
            } else {
                theme::LIST_NORMAL
            };

            let agent_type = member.agent_type.as_deref().unwrap_or("");
            let type_suffix = if agent_type.is_empty() {
                String::new()
            } else {
                format!(" ({})", agent_type)
            };

            ListItem::new(Line::from(vec![
                Span::raw(format!("{} ", prefix)),
                Span::styled(format!("{} ", status_icon), status_style),
                Span::styled(member.name.clone(), name_style),
                Span::styled(type_suffix, theme::EMPTY_STATE),
            ]))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.member_list_index));

    let list = List::new(items)
        .block(block)
        .highlight_style(theme::LIST_SELECTED);

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_task_list(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.teams_pane == TeamsPane::Tasks;
    let border_style = if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let tasks = &app.tasks;
    let title = format!(" Tasks [{}] ", tasks.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if tasks.is_empty() {
        let msg = Paragraph::new("No tasks found.")
            .style(theme::EMPTY_STATE)
            .block(block);
        f.render_widget(msg, area);
        return;
    }

    let items: Vec<ListItem> = tasks
        .iter()
        .enumerate()
        .map(|(i, task)| {
            let prefix = if i == app.task_list_index { ">" } else { " " };
            let status_style = match task.status {
                TaskStatus::Pending => theme::TASK_PENDING,
                TaskStatus::InProgress => theme::TASK_IN_PROGRESS,
                TaskStatus::Completed => theme::TASK_COMPLETED,
                TaskStatus::Deleted => theme::TASK_COMPLETED,
            };

            ListItem::new(Line::from(vec![
                Span::raw(format!("{} ", prefix)),
                Span::styled(format!("{} ", task.status.icon()), status_style),
                Span::raw(format!("#{} {}", task.id, task.display_title())),
            ]))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.task_list_index));

    let list = List::new(items)
        .block(block)
        .highlight_style(theme::LIST_SELECTED);

    f.render_stateful_widget(list, area, &mut state);
}

/// Context-sensitive detail panel. Content depends on which pane is focused.
fn draw_detail_panel(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.teams_pane == TeamsPane::Detail;
    let border_style = if is_active {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    match app.teams_pane {
        TeamsPane::Teams => draw_team_detail(f, area, app, border_style),
        TeamsPane::Members => draw_member_detail(f, area, app, border_style),
        TeamsPane::Tasks | TeamsPane::Detail => draw_task_detail(f, area, app, border_style),
    }
}

/// Show team description and metadata.
fn draw_team_detail(f: &mut Frame, area: Rect, app: &App, border_style: ratatui::style::Style) {
    let block = Block::default()
        .title(" Team Info ")
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.teams.is_empty() {
        let msg = Paragraph::new("No team selected.")
            .style(theme::EMPTY_STATE)
            .block(block);
        f.render_widget(msg, area);
        return;
    }

    let idx = app.team_list_index.min(app.teams.len() - 1);
    let team = &app.teams[idx];

    let mut lines = Vec::new();
    let label_style = ratatui::style::Style::new().fg(ratatui::style::Color::Yellow);

    // Team name
    lines.push(Line::from(vec![
        Span::styled("Name: ", label_style),
        Span::raw(team.display_name()),
    ]));

    // Directory
    lines.push(Line::from(vec![
        Span::styled("Dir:  ", label_style),
        Span::raw(&team.dir_name),
    ]));

    // Created date
    if let Some(created_at) = team.config.created_at {
        let dt = chrono::DateTime::from_timestamp_millis(created_at as i64);
        if let Some(dt) = dt {
            lines.push(Line::from(vec![
                Span::styled("Created: ", label_style),
                Span::raw(dt.format("%Y-%m-%d %H:%M").to_string()),
            ]));
        }
    }

    // Lead agent
    if let Some(ref lead_id) = team.config.lead_agent_id {
        // Find the lead member name
        let lead_name = team
            .config
            .members
            .iter()
            .find(|m| m.agent_id.as_deref() == Some(lead_id))
            .map(|m| m.name.as_str())
            .unwrap_or(lead_id.as_str());
        lines.push(Line::from(vec![
            Span::styled("Lead: ", label_style),
            Span::styled(lead_name.to_string(), theme::AGENT_LEAD),
        ]));
    }

    // Lead session
    if let Some(ref session_id) = team.config.lead_session_id {
        let short_id = truncate_chars(session_id, 8);
        lines.push(Line::from(vec![
            Span::styled("Session: ", label_style),
            Span::raw(short_id),
        ]));
    }

    // Description
    if let Some(ref desc) = team.config.description {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Description:", label_style)));
        for line in desc.lines() {
            lines.push(Line::from(format!("  {}", line)));
        }
    }

    // Member count
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Members: ", label_style),
        Span::raw(format!("{}", team.config.members.len())),
    ]));

    // List member names with status
    for member in &team.config.members {
        let status = app.agent_statuses.get(&member.name);
        let (status_icon, status_style) = match status {
            Some(AgentStatus::Starting) => ("[~]", theme::AGENT_STARTING),
            Some(AgentStatus::Working) => ("[>]", theme::AGENT_WORKING),
            Some(AgentStatus::Idle) => ("[z]", theme::AGENT_IDLE),
            Some(AgentStatus::ShutDown) => ("[x]", theme::AGENT_SHUTDOWN),
            None => ("   ", theme::LIST_NORMAL),
        };

        let is_lead = member.is_lead(&team.config);
        let name_style = if is_lead {
            theme::AGENT_LEAD
        } else {
            theme::LIST_NORMAL
        };

        let agent_type = member.agent_type.as_deref().unwrap_or("");
        let type_suffix = if agent_type.is_empty() {
            String::new()
        } else {
            format!(" ({})", agent_type)
        };

        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{} ", status_icon), status_style),
            Span::styled(member.name.clone(), name_style),
            Span::styled(type_suffix, theme::EMPTY_STATE),
        ]));
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

/// Show member info header + inbox messages for the selected member.
fn draw_member_detail(f: &mut Frame, area: Rect, app: &App, border_style: ratatui::style::Style) {
    let members = app.current_team_members();
    let member = if !members.is_empty() {
        let idx = app.member_list_index.min(members.len() - 1);
        Some(&members[idx])
    } else {
        None
    };

    let member_name = member.map(|m| m.name.as_str()).unwrap_or("");

    let title = if member_name.is_empty() {
        " Member Detail ".to_string()
    } else {
        format!(" {} [{} msgs] ", member_name, app.inbox_messages.len())
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let Some(member) = member else {
        let msg = Paragraph::new("No member selected.")
            .style(theme::EMPTY_STATE)
            .block(block);
        f.render_widget(msg, area);
        return;
    };
    let label_style = ratatui::style::Style::new().fg(ratatui::style::Color::Yellow);
    let mut lines = Vec::new();

    // Member info header
    // Status badge
    let status = app.agent_statuses.get(&member.name);
    if let Some(status) = status {
        let (icon, style) = match status {
            AgentStatus::Starting => ("[~] Starting", theme::AGENT_STARTING),
            AgentStatus::Working => ("[>] Working", theme::AGENT_WORKING),
            AgentStatus::Idle => ("[z] Idle", theme::AGENT_IDLE),
            AgentStatus::ShutDown => ("[x] Shut down", theme::AGENT_SHUTDOWN),
        };
        lines.push(Line::from(vec![
            Span::styled("Status: ", label_style),
            Span::styled(icon, style),
        ]));
    }

    // Agent type + model
    if let Some(ref agent_type) = member.agent_type {
        let model_str = member.model.as_deref().unwrap_or("");
        let suffix = if model_str.is_empty() {
            String::new()
        } else {
            format!(" ({})", model_str)
        };
        lines.push(Line::from(vec![
            Span::styled("Type: ", label_style),
            Span::raw(format!("{}{}", agent_type, suffix)),
        ]));
    }

    // Backend type
    if let Some(ref backend) = member.backend_type {
        lines.push(Line::from(vec![
            Span::styled("Backend: ", label_style),
            Span::raw(backend.as_str()),
        ]));
    }

    // Joined date
    if let Some(joined_at) = member.joined_at {
        let dt = chrono::DateTime::from_timestamp_millis(joined_at as i64);
        if let Some(dt) = dt {
            lines.push(Line::from(vec![
                Span::styled("Joined: ", label_style),
                Span::raw(dt.format("%Y-%m-%d %H:%M").to_string()),
            ]));
        }
    }

    // Plan mode required
    if member.plan_mode_required == Some(true) {
        lines.push(Line::from(vec![
            Span::styled("Plan mode: ", label_style),
            Span::styled(
                "required",
                ratatui::style::Style::new().fg(ratatui::style::Color::Yellow),
            ),
        ]));
    }

    // Lead indicator
    if !app.teams.is_empty() {
        let team_idx = app.team_list_index.min(app.teams.len() - 1);
        if member.is_lead(&app.teams[team_idx].config) {
            lines.push(Line::from(Span::styled("Team Lead", theme::AGENT_LEAD)));
        }
    }

    // Separator before inbox
    if !lines.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "--- Inbox ---",
            ratatui::style::Style::new().fg(ratatui::style::Color::DarkGray),
        )));
    }

    if app.inbox_messages.is_empty() {
        lines.push(Line::from(Span::styled(
            "No inbox messages.",
            theme::EMPTY_STATE,
        )));
    } else {
        for (i, msg) in app.inbox_messages.iter().enumerate() {
            if i > 0 {
                lines.push(Line::from(Span::styled(
                    "────────────────────────────────",
                    ratatui::style::Style::new().fg(ratatui::style::Color::DarkGray),
                )));
            }

            // Header: from + timestamp
            let read_marker = if msg.read == Some(true) { " " } else { "*" };
            lines.push(Line::from(vec![
                Span::styled(
                    read_marker,
                    ratatui::style::Style::new().fg(ratatui::style::Color::Red),
                ),
                Span::styled(
                    format!(" {} ", msg.from),
                    ratatui::style::Style::new()
                        .fg(ratatui::style::Color::Cyan)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ),
                Span::styled(
                    msg.display_time(),
                    ratatui::style::Style::new().fg(ratatui::style::Color::DarkGray),
                ),
            ]));

            // Message body
            let text = msg.display_text();
            for line in text.lines().take(6) {
                lines.push(Line::from(format!("  {}", line)));
            }
            // Indicate truncation
            if text.lines().count() > 6 {
                lines.push(Line::from(Span::styled(
                    "  ...",
                    ratatui::style::Style::new().fg(ratatui::style::Color::DarkGray),
                )));
            }
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.detail_scroll.min(u16::MAX as usize) as u16, 0));
    f.render_widget(paragraph, area);
}

/// Show full task description, status, owner, and dependencies.
fn draw_task_detail(f: &mut Frame, area: Rect, app: &App, border_style: ratatui::style::Style) {
    let block = Block::default()
        .title(" Task Detail ")
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.tasks.is_empty() {
        let msg = Paragraph::new("No task selected.")
            .style(theme::EMPTY_STATE)
            .block(block);
        f.render_widget(msg, area);
        return;
    }

    let idx = app.task_list_index.min(app.tasks.len() - 1);
    let task = &app.tasks[idx];
    let label_style = ratatui::style::Style::new().fg(ratatui::style::Color::Yellow);

    let mut lines = Vec::new();

    // Title
    lines.push(Line::from(vec![
        Span::styled(
            format!("#{} ", task.id),
            ratatui::style::Style::new()
                .fg(ratatui::style::Color::White)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        Span::raw(task.display_title()),
    ]));

    lines.push(Line::from(""));

    // Status
    let status_style = match task.status {
        TaskStatus::Pending => theme::TASK_PENDING,
        TaskStatus::InProgress => theme::TASK_IN_PROGRESS,
        TaskStatus::Completed => theme::TASK_COMPLETED,
        TaskStatus::Deleted => theme::TASK_COMPLETED,
    };
    lines.push(Line::from(vec![
        Span::styled("Status: ", label_style),
        Span::styled(
            format!("{} {:?}", task.status.icon(), task.status),
            status_style,
        ),
    ]));

    // Owner
    if let Some(ref owner) = task.owner {
        lines.push(Line::from(vec![
            Span::styled("Owner:  ", label_style),
            Span::raw(owner.as_str()),
        ]));
    }

    // Active form
    if let Some(ref active_form) = task.active_form {
        lines.push(Line::from(vec![
            Span::styled("Active: ", label_style),
            Span::styled(
                active_form.as_str(),
                ratatui::style::Style::new().fg(ratatui::style::Color::Cyan),
            ),
        ]));
    }

    // Blocks
    if !task.blocks.is_empty() {
        let ids: Vec<String> = task.blocks.iter().map(|id| format!("#{}", id)).collect();
        lines.push(Line::from(vec![
            Span::styled("Blocks: ", label_style),
            Span::raw(ids.join(", ")),
        ]));
    }

    // Blocked by
    if !task.blocked_by.is_empty() {
        let ids: Vec<String> = task
            .blocked_by
            .iter()
            .map(|id| format!("#{}", id))
            .collect();
        lines.push(Line::from(vec![
            Span::styled("Blocked by: ", label_style),
            Span::raw(ids.join(", ")),
        ]));
    }

    // Description
    if let Some(ref desc) = task.description {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Description:", label_style)));
        for line in desc.lines() {
            lines.push(Line::from(format!("  {}", line)));
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.detail_scroll.min(u16::MAX as usize) as u16, 0));
    f.render_widget(paragraph, area);
}
