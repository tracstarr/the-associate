use std::io::Read;

use anyhow::Result;

use crate::model::linear::{FlatLinearItem, LinearIssue};

/// Fetch issues assigned to the authenticated user (viewer) from Linear's GraphQL API.
/// If `username` is provided, filter by assignee email instead of using the viewer query.
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

    parse_response(&output.stdout, username.is_some())
}

/// Build the GraphQL query string.
fn build_query(username: Option<&str>, team_key: Option<&str>) -> String {
    let mut filters = Vec::new();

    // Exclude completed and canceled issues
    filters.push("state: { type: { nin: [\"completed\", \"canceled\"] } }".to_string());

    if let Some(team) = team_key {
        filters.push(format!("team: {{ key: {{ eq: \"{}\" }} }}", team));
    }

    let filter_str = filters.join(", ");

    if let Some(email) = username {
        // Use top-level issues query with assignee email filter
        format!(
            r#"query {{ issues(filter: {{ assignee: {{ email: {{ eq: "{}" }} }}, {} }}, first: 50, orderBy: updatedAt) {{ nodes {{ identifier title description priority priorityLabel state {{ name type color }} assignee {{ name email }} labels {{ nodes {{ name color }} }} url team {{ name key }} createdAt updatedAt }} }} }}"#,
            email, filter_str
        )
    } else {
        // Use viewer.assignedIssues for the authenticated user
        format!(
            r#"query {{ viewer {{ assignedIssues(filter: {{ {} }}, first: 50, orderBy: updatedAt) {{ nodes {{ identifier title description priority priorityLabel state {{ name type color }} assignee {{ name email }} labels {{ nodes {{ name color }} }} url team {{ name key }} createdAt updatedAt }} }} }} }}"#,
            filter_str
        )
    }
}

/// Parse the GraphQL JSON response into a list of LinearIssues.
fn parse_response(data: &[u8], used_issues_query: bool) -> Result<Vec<LinearIssue>> {
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

    let nodes = if used_issues_query {
        // { data: { issues: { nodes: [...] } } }
        value.pointer("/data/issues/nodes")
    } else {
        // { data: { viewer: { assignedIssues: { nodes: [...] } } } }
        value.pointer("/data/viewer/assignedIssues/nodes")
    };

    let nodes = nodes
        .and_then(|n| n.as_array())
        .ok_or_else(|| anyhow::anyhow!("unexpected response structure from Linear API"))?;

    let issues: Vec<LinearIssue> = nodes
        .iter()
        .filter_map(|node| serde_json::from_value(node.clone()).ok())
        .collect();

    Ok(issues)
}

/// Group issues by workflow state into a flat list of headers and issues.
/// Groups are ordered: "started" states first, then "unstarted", then "backlog".
pub fn categorize_issues(issues: &[LinearIssue]) -> Vec<FlatLinearItem> {
    let mut state_order: Vec<(String, String)> = Vec::new();
    for issue in issues {
        if !state_order
            .iter()
            .any(|(name, _)| name == &issue.state.name)
        {
            state_order.push((issue.state.name.clone(), issue.state.state_type.clone()));
        }
    }

    // Sort: started first, then unstarted, then backlog, then anything else
    state_order.sort_by_key(|(_, state_type)| match state_type.as_str() {
        "started" => 0,
        "unstarted" => 1,
        "backlog" => 2,
        _ => 3,
    });

    let mut result = Vec::new();
    for (state_name, state_type) in &state_order {
        result.push(FlatLinearItem::StateHeader(
            state_name.clone(),
            state_type.clone(),
        ));
        for issue in issues {
            if issue.state.name == *state_name {
                result.push(FlatLinearItem::Issue(Box::new(issue.clone())));
            }
        }
    }

    result
}
