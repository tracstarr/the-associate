use anyhow::Result;

use crate::model::github::{FlatIssueItem, FlatPrItem, GitHubIssue, PullRequest};

/// List open PRs for a repo using `gh pr list`.
pub fn list_open_prs(repo: &str) -> Result<Vec<PullRequest>> {
    let mut child = std::process::Command::new("gh")
        .args([
            "pr",
            "list",
            "--repo",
            repo,
            "--state",
            "open",
            "--limit",
            "100",
            "--json",
            "number,title,state,author,url,createdAt,updatedAt,headRefName,baseRefName,isDraft,additions,deletions,reviewDecision,assignees,labels",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;
    let output = wait_with_output(&mut child)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gh pr list failed: {}", stderr.trim());
    }

    let prs: Vec<PullRequest> = serde_json::from_slice(&output.stdout)?;
    Ok(prs)
}

/// Categorize PRs into sections: My PRs, Assigned to Me, Other Open.
/// Returns a flat list with section headers interleaved.
pub fn categorize_prs(prs: &[PullRequest], current_user: &str) -> Vec<FlatPrItem> {
    let mut my_prs: Vec<&PullRequest> = Vec::new();
    let mut assigned: Vec<&PullRequest> = Vec::new();
    let mut other: Vec<&PullRequest> = Vec::new();

    for pr in prs {
        if pr.author.login.eq_ignore_ascii_case(current_user) {
            my_prs.push(pr);
        } else if pr
            .assignees
            .iter()
            .any(|a| a.login.eq_ignore_ascii_case(current_user))
        {
            assigned.push(pr);
        } else {
            other.push(pr);
        }
    }

    // Sort each group by updated_at descending
    my_prs.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    assigned.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    other.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    let mut result = Vec::new();

    if !my_prs.is_empty() {
        result.push(FlatPrItem::SectionHeader(format!(
            "My PRs ({})",
            my_prs.len()
        )));
        for pr in my_prs {
            result.push(FlatPrItem::Pr(Box::new(pr.clone())));
        }
    }

    if !assigned.is_empty() {
        result.push(FlatPrItem::SectionHeader(format!(
            "Assigned to Me ({})",
            assigned.len()
        )));
        for pr in assigned {
            result.push(FlatPrItem::Pr(Box::new(pr.clone())));
        }
    }

    if !other.is_empty() {
        result.push(FlatPrItem::SectionHeader(format!(
            "Other Open ({})",
            other.len()
        )));
        for pr in other {
            result.push(FlatPrItem::Pr(Box::new(pr.clone())));
        }
    }

    result
}

// ---------------------------------------------------------------------------
// GitHub Issues
// ---------------------------------------------------------------------------

/// Run a gh command, returning stdout on success.
///
/// Reads stdout and stderr concurrently in separate threads to prevent the
/// pipe-buffer deadlock that occurs when the child writes more data than the
/// OS pipe buffer can hold before the parent drains it.
fn run_gh(args: &[&str]) -> Result<Vec<u8>> {
    let mut child = std::process::Command::new("gh")
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let output = wait_with_output(&mut child)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gh failed: {}", stderr.trim());
    }
    Ok(output.stdout)
}

/// Wait for a child process while concurrently draining its stdout and stderr
/// pipes. Using try_wait() and reading after exit can deadlock when output
/// exceeds the pipe buffer capacity — the child blocks on write, never exits.
fn wait_with_output(child: &mut std::process::Child) -> Result<std::process::Output> {
    use std::io::Read;

    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();

    let stdout_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(ref mut pipe) = stdout_pipe {
            pipe.read_to_end(&mut buf).ok();
        }
        buf
    });
    let stderr_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(ref mut pipe) = stderr_pipe {
            pipe.read_to_end(&mut buf).ok();
        }
        buf
    });

    let status = child.wait()?;
    let stdout = stdout_thread.join().unwrap_or_default();
    let stderr = stderr_thread.join().unwrap_or_default();

    Ok(std::process::Output { status, stdout, stderr })
}

/// Check whether the repo has issues enabled by querying the repo metadata.
pub fn repo_has_issues(repo: &str) -> bool {
    let args = vec![
        "repo",
        "view",
        repo,
        "--json",
        "hasIssuesEnabled",
        "--jq",
        ".hasIssuesEnabled",
    ];
    match run_gh(&args) {
        Ok(stdout) => {
            let val = String::from_utf8_lossy(&stdout).trim().to_string();
            val == "true"
        }
        Err(_) => false,
    }
}

/// List issues for a repo using `gh issue list`.
pub fn list_issues(repo: &str, state: &str) -> Result<Vec<GitHubIssue>> {
    let stdout = run_gh(&[
        "issue",
        "list",
        "--repo",
        repo,
        "--state",
        state,
        "--limit",
        "100",
        "--json",
        "number,title,state,url,createdAt,updatedAt,author,labels,assignees,body,comments,milestone",
    ])?;
    let issues: Vec<GitHubIssue> = serde_json::from_slice(&stdout)?;
    Ok(issues)
}

/// Categorize issues into sections: Assigned to Me, My Issues, Other.
pub fn categorize_issues(issues: &[GitHubIssue], current_user: &str) -> Vec<FlatIssueItem> {
    let mut my_issues: Vec<&GitHubIssue> = Vec::new();
    let mut assigned: Vec<&GitHubIssue> = Vec::new();
    let mut other: Vec<&GitHubIssue> = Vec::new();

    for issue in issues {
        if issue
            .assignees
            .iter()
            .any(|a| a.login.eq_ignore_ascii_case(current_user))
        {
            assigned.push(issue);
        } else if issue.author.login.eq_ignore_ascii_case(current_user) {
            my_issues.push(issue);
        } else {
            other.push(issue);
        }
    }

    assigned.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    my_issues.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    other.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    let mut result = Vec::new();

    if !assigned.is_empty() {
        result.push(FlatIssueItem::SectionHeader(format!(
            "Assigned to Me ({})",
            assigned.len()
        )));
        for issue in assigned {
            result.push(FlatIssueItem::Issue(Box::new(issue.clone())));
        }
    }

    if !my_issues.is_empty() {
        result.push(FlatIssueItem::SectionHeader(format!(
            "My Issues ({})",
            my_issues.len()
        )));
        for issue in my_issues {
            result.push(FlatIssueItem::Issue(Box::new(issue.clone())));
        }
    }

    if !other.is_empty() {
        result.push(FlatIssueItem::SectionHeader(format!(
            "Other ({})",
            other.len()
        )));
        for issue in other {
            result.push(FlatIssueItem::Issue(Box::new(issue.clone())));
        }
    }

    result
}

/// Create a new issue via `gh issue create`.
pub fn create_issue(repo: &str, title: &str, body: &str) -> Result<()> {
    let mut args = vec!["issue", "create", "--repo", repo, "--title", title];
    if !body.is_empty() {
        args.extend_from_slice(&["--body", body]);
    }
    run_gh(&args)?;
    Ok(())
}

/// Edit an existing issue's title and/or body via `gh issue edit`.
pub fn edit_issue(repo: &str, number: u64, title: &str, body: &str) -> Result<()> {
    let num_str = number.to_string();
    let mut args = vec!["issue", "edit", &num_str, "--repo", repo];
    args.extend_from_slice(&["--title", title]);
    args.extend_from_slice(&["--body", body]);
    run_gh(&args)?;
    Ok(())
}

/// Close an issue via `gh issue close`.
pub fn close_issue(repo: &str, number: u64) -> Result<()> {
    let num_str = number.to_string();
    run_gh(&["issue", "close", &num_str, "--repo", repo])?;
    Ok(())
}

/// Reopen an issue via `gh issue reopen`.
pub fn reopen_issue(repo: &str, number: u64) -> Result<()> {
    let num_str = number.to_string();
    run_gh(&["issue", "reopen", &num_str, "--repo", repo])?;
    Ok(())
}

/// Add a comment to an issue via `gh issue comment`.
pub fn comment_issue(repo: &str, number: u64, body: &str) -> Result<()> {
    let num_str = number.to_string();
    run_gh(&["issue", "comment", &num_str, "--repo", repo, "--body", body])?;
    Ok(())
}
