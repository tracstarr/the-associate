use std::collections::HashMap;

use crate::model::inbox::InboxMessage;
use crate::model::task::{Task, TaskStatus};

#[derive(Debug, Clone, PartialEq)]
pub enum AgentStatus {
    Starting, // in config but no inbox messages from this agent
    Working,  // owns an in_progress task, or last message is not idle/shutdown
    Idle,     // last inbox message (sent BY this agent) is idle_notification
    ShutDown, // last inbox message (sent BY this agent) is shutdown_approved
}

impl AgentStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Starting => "[~]",
            Self::Working => "[>]",
            Self::Idle => "[z]",
            Self::ShutDown => "[x]",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Working => "working",
            Self::Idle => "idle",
            Self::ShutDown => "shut down",
        }
    }
}

/// Derive agent status from the team lead's inbox messages and the task list.
///
/// We look at the lead's inbox for the most recent message `from` this agent,
/// then check task ownership.
pub fn derive_agent_status(
    member_name: &str,
    lead_inbox: &[InboxMessage],
    tasks: &[Task],
) -> AgentStatus {
    // Find the most recent message FROM this agent in the lead's inbox.
    // Messages are sorted most-recent-first by the inbox loader.
    let latest_from_agent = lead_inbox.iter().find(|msg| msg.from == member_name);

    if let Some(msg) = latest_from_agent {
        if let Some(msg_type) = msg.message_type() {
            if msg_type == "shutdown_approved" {
                return AgentStatus::ShutDown;
            }
            if msg_type == "idle_notification" {
                return AgentStatus::Idle;
            }
        }
    }

    // Check if agent owns any in-progress task
    let owns_active_task = tasks
        .iter()
        .any(|t| t.owner.as_deref() == Some(member_name) && t.status == TaskStatus::InProgress);
    if owns_active_task {
        return AgentStatus::Working;
    }

    // No messages from this agent at all → Starting
    if latest_from_agent.is_none() {
        return AgentStatus::Starting;
    }

    // Has messages but not idle/shutdown, and no active task
    AgentStatus::Working
}

/// Derive statuses for all members given the lead's inbox and task list.
pub fn derive_all_statuses(
    member_names: &[&str],
    lead_inbox: &[InboxMessage],
    tasks: &[Task],
) -> HashMap<String, AgentStatus> {
    member_names
        .iter()
        .map(|&name| {
            let status = derive_agent_status(name, lead_inbox, tasks);
            (name.to_string(), status)
        })
        .collect()
}
