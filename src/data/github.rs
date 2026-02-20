use std::io::Read;

use anyhow::Result;

use crate::model::github::{FlatPrItem, PullRequest};

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
