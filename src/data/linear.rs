use std::io::Read;

use anyhow::Result;

use crate::model::linear::{FlatLinearItem, LinearIssue};

/// Fetch issues from Linear's GraphQL API.
/// If `username` is provided, filter by assignee email.
/// If `team_key` is provided, add a team filter.
pub fn fetch_my_issues(
    api_key: &str,
    username: Option<&str>,
    team_key: Option<&str>,
) -> Result<Vec<LinearIssue>> {
    let query = build_query(username, team_key);

    let body = serde_json::json!({ "query": query });
    let body_str = serde_json::to_string(&body)?;

    let mut child = std::process::Command::new("curl")
        .args([
            "-s",
            "-X",
            "POST",
            "-H",
            "Content-Type: application/json",
            "-H",
            &format!("Authorization: {}", api_key),
            "-d",
            &body_str,
            "https://api.linear.app/graphql",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let output = {
        let timeout = std::time::Duration::from_secs(30);
        let start = std::time::Instant::now();
        loop {
            match child.try_wait()? {
                Some(status) => {
                    let mut stdout = Vec::new();
                    let mut stderr = Vec::new();
                    if let Some(mut s) = child.stdout.take() {
                        s.read_to_end(&mut stdout).ok();
                    }
                    if let Some(mut s) = child.stderr.take() {
                        s.read_to_end(&mut stderr).ok();
                    }
                    break std::process::Output {
                        status,
                        stdout,
                        stderr,
                    };
                }
                None => {
                    if start.elapsed() > timeout {
                        child.kill().ok();
                        anyhow::bail!("Linear API request timed out after 30s");
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("curl failed: {}", stderr.trim());
    }

    parse_response(&output.stdout)
}

/// Build the GraphQL query string.
fn build_query(username: Option<&str>, team_key: Option<&str>) -> String {
    let mut filters = Vec::new();

    // Exclude completed and cancelled issues
    filters.push("state: { type: { nin: [\"completed\", \"cancelled\"] } }".to_string());

    if let Some(team) = team_key {
        let safe_key = team.replace('\\', "\\\\").replace('"', "\\\"");
        filters.push(format!("team: {{ key: {{ eq: \"{}\" }} }}", safe_key));
    }

    let filter_str = filters.join(", ");

    // When username is set, include issues assigned to that user OR unassigned.
    // Without username, return all non-completed workspace issues.
    let assignee_filter = if let Some(email) = username {
        let safe_user = email.replace('\\', "\\\\").replace('"', "\\\"");
        format!(
            r#", or: [{{ assignee: {{ email: {{ eq: "{}" }} }} }}, {{ assignee: {{ null: true }} }}]"#,
            safe_user
        )
    } else {
        String::new()
    };

    format!(
        r#"query {{ issues(filter: {{ {}{} }}, first: 50, orderBy: updatedAt) {{ nodes {{ identifier title description priority priorityLabel state {{ name type color }} assignee {{ name email }} labels {{ nodes {{ name color }} }} url team {{ name key }} createdAt updatedAt }} }} }}"#,
        filter_str, assignee_filter
    )
}

/// Parse the GraphQL JSON response into a list of LinearIssues.
fn parse_response(data: &[u8]) -> Result<Vec<LinearIssue>> {
    let value: serde_json::Value = serde_json::from_slice(data)?;

    // Check for GraphQL errors
    if let Some(errors) = value.get("errors").and_then(|e| e.as_array()) {
        if let Some(first) = errors.first() {
            let msg = first
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            anyhow::bail!("Linear API error: {}", msg);
        }
    }

    let nodes = value.pointer("/data/issues/nodes");

    let nodes = nodes
        .and_then(|n| n.as_array())
        .ok_or_else(|| anyhow::anyhow!("unexpected response structure from Linear API"))?;

    let issues: Vec<LinearIssue> = nodes
        .iter()
        .map(|node| {
            serde_json::from_value(node.clone())
                .map_err(|e| anyhow::anyhow!("failed to parse Linear issue: {}", e))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(issues)
}

/// Group issues into "My Tasks" (assigned to username) and "Unassigned" sections.
/// Within each section issues are sorted by state: started → unstarted → backlog.
/// If username is None, all assigned issues appear in "Assigned" and unassigned in "Unassigned".
pub fn categorize_issues(issues: &[LinearIssue], username: Option<&str>) -> Vec<FlatLinearItem> {
    let state_priority = |state_type: &str| -> u8 {
        match state_type {
            "started" => 0,
            "unstarted" => 1,
            "backlog" => 2,
            _ => 3,
        }
    };

    let mut my_tasks: Vec<&LinearIssue> = Vec::new();
    let mut assigned: Vec<&LinearIssue> = Vec::new();
    let mut unassigned: Vec<&LinearIssue> = Vec::new();

    for issue in issues {
        match &issue.assignee {
            None => unassigned.push(issue),
            Some(assignee) => {
                let is_mine = username.map_or(false, |user| {
                    assignee
                        .email
                        .as_deref()
                        .map_or(false, |e| e.to_lowercase() == user.to_lowercase())
                });
                if is_mine {
                    my_tasks.push(issue);
                } else if username.is_none() {
                    // When no username is configured, show all assigned issues
                    assigned.push(issue);
                }
            }
        }
    }

    my_tasks.sort_by_key(|i| state_priority(&i.state.state_type));
    assigned.sort_by_key(|i| state_priority(&i.state.state_type));
    unassigned.sort_by_key(|i| state_priority(&i.state.state_type));

    let mut result = Vec::new();

    if !my_tasks.is_empty() {
        result.push(FlatLinearItem::AssignmentHeader("My Tasks".to_string()));
        for issue in my_tasks {
            result.push(FlatLinearItem::Issue(Box::new(issue.clone())));
        }
    }

    if !assigned.is_empty() {
        result.push(FlatLinearItem::AssignmentHeader("Assigned".to_string()));
        for issue in assigned {
            result.push(FlatLinearItem::Issue(Box::new(issue.clone())));
        }
    }

    if !unassigned.is_empty() {
        result.push(FlatLinearItem::AssignmentHeader("Unassigned".to_string()));
        for issue in unassigned {
            result.push(FlatLinearItem::Issue(Box::new(issue.clone())));
        }
    }

    result
}
