use crate::model::github::{GitHubIssue, PullRequest};
use crate::model::jira::JiraIssue;
use crate::model::linear::LinearIssue;
use crate::model::process::TicketInfo;
use crate::model::process::TicketSource;

/// Extract ticket info from a GitHub PR.
pub fn ticket_from_github_pr(pr: &PullRequest) -> TicketInfo {
    let mut extra = Vec::new();
    extra.push(("Branch".to_string(), pr.head_ref_name.clone()));
    extra.push(("Base".to_string(), pr.base_ref_name.clone()));
    extra.push(("Author".to_string(), pr.author.login.clone()));
    if let Some(ref review) = pr.review_decision {
        extra.push(("Review Status".to_string(), review.clone()));
    }

    TicketInfo {
        source: TicketSource::GitHubPR,
        key: format!("#{}", pr.number),
        title: pr.title.clone(),
        description: format!(
            "GitHub PR #{} - {}\nBranch: {} -> {}\nAdditions: {}, Deletions: {}",
            pr.number, pr.title, pr.head_ref_name, pr.base_ref_name, pr.additions, pr.deletions,
        ),
        labels: pr.labels.iter().map(|l| l.name.clone()).collect(),
        url: pr.url.clone(),
        extra_fields: extra,
    }
}

/// Extract ticket info from a GitHub Issue.
pub fn ticket_from_github_issue(issue: &GitHubIssue) -> TicketInfo {
    let mut extra = Vec::new();
    extra.push(("Author".to_string(), issue.author.login.clone()));
    extra.push(("State".to_string(), issue.state.clone()));
    if !issue.assignees.is_empty() {
        let assignees = issue.assignees.iter().map(|a| a.login.clone()).collect::<Vec<_>>().join(", ");
        extra.push(("Assignees".to_string(), assignees));
    }
    if let Some(ref ms) = issue.milestone {
        extra.push(("Milestone".to_string(), ms.title.clone()));
    }

    TicketInfo {
        source: TicketSource::GitHubIssue,
        key: format!("#{}", issue.number),
        title: issue.title.clone(),
        description: issue.body.clone().unwrap_or_default(),
        labels: issue.labels.iter().map(|l| l.name.clone()).collect(),
        url: issue.url.clone(),
        extra_fields: extra,
    }
}

/// Extract ticket info from a Linear issue.
pub fn ticket_from_linear(issue: &LinearIssue) -> TicketInfo {
    let extra = vec![
        ("Status".to_string(), issue.state.name.clone()),
        ("Priority".to_string(), issue.priority_label.clone()),
    ];

    TicketInfo {
        source: TicketSource::Linear,
        key: issue.identifier.clone(),
        title: issue.title.clone(),
        description: issue.description.clone().unwrap_or_default(),
        labels: issue.labels.nodes.iter().map(|l| l.name.clone()).collect(),
        url: issue.url.clone(),
        extra_fields: extra,
    }
}

/// Extract ticket info from a Jira issue.
pub fn ticket_from_jira(issue: &JiraIssue) -> TicketInfo {
    let extra = vec![
        ("Status".to_string(), issue.status_name.clone()),
        ("Type".to_string(), issue.issue_type.clone()),
        ("Priority".to_string(), issue.priority.clone()),
    ];

    TicketInfo {
        source: TicketSource::Jira,
        key: issue.key.clone(),
        title: issue.summary.clone(),
        description: issue.description.clone().unwrap_or_default(),
        labels: issue.labels.clone(),
        url: issue.url.clone(),
        extra_fields: extra,
    }
}

/// Generate the default prompt for a ticket.
///
/// The prompt instructs Claude Code to:
/// 1. Plan the implementation
/// 2. Implement the feature/fix
/// 3. Run tests and ensure they pass
/// 4. Create a PR with the changes
/// 5. Work as a team with parallel agents
pub fn build_default_prompt(ticket: &TicketInfo) -> String {
    let labels_str = if ticket.labels.is_empty() {
        "None".to_string()
    } else {
        ticket.labels.join(", ")
    };

    let extra_str = ticket
        .extra_fields
        .iter()
        .map(|(k, v)| format!("- {}: {}", k, v))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"You are implementing a feature/fix based on the following ticket.

## Ticket Information
- Source: {source}
- Key: {key}
- Title: {title}
- Labels: {labels}
- URL: {url}
{extra}

## Description
{description}

## Instructions

Please complete this ticket by following these steps:

1. **Planning Phase**: Analyze the ticket requirements thoroughly. Read relevant code in the codebase to understand the current architecture. Create a detailed implementation plan.

2. **Implementation Phase**: Implement the changes described in the ticket. Follow existing code patterns and conventions. Write clean, well-structured code.

3. **Testing Phase**: Run the existing test suite and ensure all tests pass. If the changes warrant new tests, write them. Fix any test failures.

4. **Quality Check**: Run linters and formatters. Fix any warnings or errors. Ensure the code meets project standards.

5. **PR Creation**: Create a new git branch for this work. Commit all changes with clear, descriptive commit messages. Push the branch and create a pull request with a summary of the changes.

Work as a team — use Claude's team/subagent capabilities to run tasks in parallel where possible. For example, you might have one agent handle implementation while another prepares tests, or split implementation across multiple modules.

Do not ask for user input — work autonomously to completion."#,
        source = match ticket.source {
            TicketSource::GitHubPR => "GitHub PR",
            TicketSource::GitHubIssue => "GitHub Issue",
            TicketSource::Linear => "Linear",
            TicketSource::Jira => "Jira",
        },
        key = ticket.key,
        title = ticket.title,
        labels = labels_str,
        url = ticket.url,
        extra = if extra_str.is_empty() {
            String::new()
        } else {
            format!("\n## Additional Details\n{}", extra_str)
        },
        description = if ticket.description.is_empty() {
            "No description provided.".to_string()
        } else {
            ticket.description.clone()
        },
    )
}
