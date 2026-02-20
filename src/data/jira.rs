use std::io::Read;

use anyhow::Result;

use crate::model::jira::{FlatJiraItem, JiraIssue};

/// Common statuses offered in the transition popup.
const COMMON_STATUSES: &[&str] = &["To Do", "In Progress", "In Review", "Done"];

/// Search for issues assigned to the current user that are not Done.
/// If `project_key` is provided, the query is scoped to that project.
/// If `custom_jql` is provided, it replaces the default JQL entirely.
pub fn search_my_issues(
    project_key: Option<&str>,
    custom_jql: Option<&str>,
) -> Result<Vec<JiraIssue>> {
    let jql = if let Some(jql) = custom_jql {
        jql.to_string()
    } else {
        let mut q = "assignee = currentUser() AND statusCategory not in (Done)".to_string();
        if let Some(key) = project_key {
            if !key.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
                || key.is_empty()
                || !key.starts_with(|c: char| c.is_ascii_uppercase())
            {
                anyhow::bail!(
                    "invalid Jira project key {:?}: must match [A-Z][A-Z0-9_]+",
                    key
                );
            }
            q.push_str(&format!(" AND project = \"{}\"", key));
        }
        q.push_str(" ORDER BY status ASC, updated DESC");
        q
    };

    let mut child = std::process::Command::new("acli")
        .args(["jira", "workitem", "search", "--jql", &jql, "--json"])
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
                        anyhow::bail!("command timed out after 30 seconds");
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("acli search failed: {}", stderr.trim());
    }

    parse_issues_json(&output.stdout)
}

/// Search for issues by key or label.
/// If `query` looks like a Jira key (starts with uppercase letters followed by '-'),
/// search by key. Otherwise search by label.
pub fn search_issues(query: &str) -> Result<Vec<JiraIssue>> {
    let safe_query = query.replace('\\', "\\\\").replace('"', "\\\"");
    let jql = if looks_like_jira_key(query) {
        format!("key = \"{}\"", safe_query)
    } else {
        format!("labels = \"{}\"", safe_query)
    };

    let mut child = std::process::Command::new("acli")
        .args(["jira", "workitem", "search", "--jql", &jql, "--json"])
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
                        anyhow::bail!("command timed out after 30 seconds");
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("acli search failed: {}", stderr.trim());
    }

    parse_issues_json(&output.stdout)
}

/// Return common status names for the transition popup.
/// acli has no get-transitions command, so we offer a hardcoded list
/// and filter out the issue's current status.
pub fn get_status_options(current_status: &str) -> Vec<String> {
    COMMON_STATUSES
        .iter()
        .filter(|&&s| !s.eq_ignore_ascii_case(current_status))
        .map(|s| s.to_string())
        .collect()
}

/// Transition an issue to a new status by name.
pub fn transition_issue(key: &str, status_name: &str) -> Result<()> {
    let mut child = std::process::Command::new("acli")
        .args([
            "jira",
            "workitem",
            "transition",
            "--key",
            key,
            "--status",
            status_name,
            "--yes",
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
                        anyhow::bail!("command timed out after 30 seconds");
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("acli transition failed: {}", stderr.trim());
    }

    Ok(())
}

/// Get full details for a single issue including description.
pub fn view_issue(key: &str) -> Result<JiraIssue> {
    let mut child = std::process::Command::new("acli")
        .args(["jira", "workitem", "view", key, "--json"])
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
                        anyhow::bail!("command timed out after 30 seconds");
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("acli issue get failed: {}", stderr.trim());
    }

    let value: serde_json::Value = serde_json::from_slice(&output.stdout)?;

    // The output might be a single object or an array with one element
    let obj = if value.is_array() {
        value
            .as_array()
            .and_then(|arr| arr.first())
            .ok_or_else(|| anyhow::anyhow!("empty response from acli"))?
    } else {
        &value
    };

    parse_issue_from_value(obj)
        .ok_or_else(|| anyhow::anyhow!("failed to parse issue from acli output"))
}

/// Group issues by status_name into a flat list of headers and issues.
/// Groups are ordered: "In Progress" statuses first, then "To Do", then anything else.
pub fn categorize_issues(issues: &[JiraIssue]) -> Vec<FlatJiraItem> {
    // Collect unique statuses in order of first appearance
    let mut status_order: Vec<(String, String)> = Vec::new();
    for issue in issues {
        if !status_order
            .iter()
            .any(|(name, _)| name == &issue.status_name)
        {
            status_order.push((issue.status_name.clone(), issue.status_category.clone()));
        }
    }

    // Sort groups: In Progress first, then To Do, then everything else
    status_order.sort_by_key(|(_, category)| match category.as_str() {
        "In Progress" => 0,
        "To Do" => 1,
        _ => 2,
    });

    let mut result = Vec::new();

    for (status_name, status_category) in &status_order {
        result.push(FlatJiraItem::StatusHeader(
            status_name.clone(),
            status_category.clone(),
        ));

        for issue in issues {
            if issue.status_name == *status_name {
                result.push(FlatJiraItem::Issue(issue.clone()));
            }
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Check if a string looks like a Jira issue key (e.g. "PROJ-123").
fn looks_like_jira_key(query: &str) -> bool {
    if !query.contains('-') {
        return false;
    }
    let parts: Vec<&str> = query.splitn(2, '-').collect();
    if parts.len() != 2 {
        return false;
    }
    // First part should be all uppercase letters
    parts[0].chars().all(|c| c.is_ascii_uppercase()) && !parts[0].is_empty()
}

/// Parse a JSON byte slice into a list of issues, trying serde first,
/// then falling back to manual Value extraction.
fn parse_issues_json(data: &[u8]) -> Result<Vec<JiraIssue>> {
    let value: serde_json::Value = serde_json::from_slice(data)?;

    let arr = if let Some(arr) = value.as_array() {
        arr.clone()
    } else if value.is_object() {
        // Some APIs wrap results in an object with an "issues" key
        if let Some(issues) = value.get("issues").and_then(|v| v.as_array()) {
            issues.clone()
        } else {
            vec![value]
        }
    } else {
        anyhow::bail!("unexpected JSON format from acli");
    };

    // Try direct deserialization first
    if let Ok(issues) =
        serde_json::from_value::<Vec<JiraIssue>>(serde_json::Value::Array(arr.clone()))
    {
        return Ok(issues);
    }

    // Fall back to manual extraction
    let issues: Vec<JiraIssue> = arr.iter().filter_map(parse_issue_from_value).collect();

    Ok(issues)
}

/// Parse a single JSON Value into a JiraIssue, handling both flat and nested formats.
fn parse_issue_from_value(v: &serde_json::Value) -> Option<JiraIssue> {
    let key = v.get("key")?.as_str()?.to_string();

    // Try flat format first, then nested fields format
    let summary = v
        .get("summary")
        .or_else(|| v.pointer("/fields/summary"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let status_name = v
        .get("statusName")
        .or_else(|| v.get("status_name"))
        .or_else(|| v.pointer("/status/name"))
        .or_else(|| v.pointer("/fields/status/name"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let status_category = v
        .get("statusCategory")
        .or_else(|| v.get("status_category"))
        .or_else(|| v.pointer("/status/statusCategory/name"))
        .or_else(|| v.pointer("/fields/status/statusCategory/name"))
        .and_then(|v| {
            // Could be a string or an object with a "name" field
            v.as_str().map(|s| s.to_string()).or_else(|| {
                v.get("name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string())
            })
        })
        .unwrap_or_default();

    let issue_type = v
        .get("issueType")
        .or_else(|| v.get("issue_type"))
        .or_else(|| v.pointer("/issuetype/name"))
        .or_else(|| v.pointer("/fields/issuetype/name"))
        .and_then(|v| {
            v.as_str().map(|s| s.to_string()).or_else(|| {
                v.get("name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string())
            })
        })
        .unwrap_or_default();

    let priority = v
        .get("priority")
        .or_else(|| v.pointer("/fields/priority"))
        .and_then(|v| {
            v.as_str().map(|s| s.to_string()).or_else(|| {
                v.get("name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string())
            })
        })
        .unwrap_or_default();

    let labels = v
        .get("labels")
        .or_else(|| v.pointer("/fields/labels"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let description = v
        .get("description")
        .or_else(|| v.pointer("/fields/description"))
        .and_then(|v| {
            // Plain text string
            if let Some(s) = v.as_str() {
                return Some(s.to_string());
            }
            // Atlassian Document Format (ADF) object - extract text content
            if v.is_object() {
                let text = extract_adf_text(v);
                if !text.is_empty() {
                    return Some(text);
                }
            }
            None
        });

    // Build browsable URL: extract the Jira base from the REST "self" link,
    // fall back to JIRA_URL env var, then construct /browse/{key}.
    let url = v
        .get("self")
        .and_then(|v| v.as_str())
        .and_then(|s| {
            // "self" is like "https://foo.atlassian.net/rest/api/2/issue/12345"
            s.find("/rest/").map(|i| &s[..i])
        })
        .or_else(|| {
            // Check for an explicit "url" field that might contain a base
            v.get("url").and_then(|v| v.as_str())
        })
        .map(|base| format!("{}/browse/{}", base.trim_end_matches('/'), key))
        .unwrap_or_else(|| {
            std::env::var("JIRA_URL")
                .map(|base| format!("{}/browse/{}", base.trim_end_matches('/'), key))
                .unwrap_or_default()
        });

    Some(JiraIssue {
        key,
        summary,
        status_name,
        status_category,
        issue_type,
        priority,
        labels,
        description,
        url,
    })
}

/// Recursively extract plain text from an Atlassian Document Format (ADF) value.
fn extract_adf_text(v: &serde_json::Value) -> String {
    let mut buf = String::new();
    extract_adf_text_inner(v, &mut buf);
    buf.trim().to_string()
}

fn extract_adf_text_inner(v: &serde_json::Value, buf: &mut String) {
    if let Some(text) = v.get("text").and_then(|t| t.as_str()) {
        buf.push_str(text);
    }
    if let Some(content) = v.get("content").and_then(|c| c.as_array()) {
        for child in content {
            extract_adf_text_inner(child, buf);
        }
    }
    // Add newline after block-level nodes like paragraph, heading, etc.
    if let Some(node_type) = v.get("type").and_then(|t| t.as_str()) {
        match node_type {
            "paragraph" | "heading" | "bulletList" | "orderedList" | "listItem" => {
                buf.push('\n');
            }
            _ => {}
        }
    }
}
