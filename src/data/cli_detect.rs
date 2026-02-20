use std::path::Path;
use std::process::Command;

/// Check if a CLI tool is available on PATH.
pub fn is_available(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .is_ok()
}

/// Open a URL in the default browser (Windows).
pub fn open_url(url: &str) {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return;
    }
    let _ = Command::new("cmd").args(["/C", "start", "", url]).spawn();
}

/// Try to get `owner/repo` from `git remote get-url origin` in the given directory.
fn try_git_remote(dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    parse_gh_repo_url(&url)
}

/// Parse `git remote get-url origin` into `owner/repo`.
/// Walks up parent directories to find a `.git` dir if cwd itself isn't a repo.
pub fn detect_gh_repo(cwd: &Path) -> Option<String> {
    // Try cwd first
    if let Some(repo) = try_git_remote(cwd) {
        return Some(repo);
    }
    // Walk up parent dirs looking for a .git directory
    let mut dir = cwd.parent();
    while let Some(parent) = dir {
        if parent.join(".git").exists() {
            return try_git_remote(parent);
        }
        dir = parent.parent();
    }
    None
}

fn parse_gh_repo_url(url: &str) -> Option<String> {
    // SSH: git@github.com:owner/repo.git
    if let Some(rest) = url.strip_prefix("git@github.com:") {
        let repo = rest.strip_suffix(".git").unwrap_or(rest);
        return Some(repo.to_string());
    }
    // HTTPS: https://github.com/owner/repo.git
    if let Some(rest) = url.strip_prefix("https://github.com/") {
        let repo = rest.strip_suffix(".git").unwrap_or(rest);
        return Some(repo.to_string());
    }
    None
}

/// Get the current git branch name.
pub fn detect_git_branch(cwd: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() || branch == "HEAD" {
        None
    } else {
        Some(branch)
    }
}

/// Extract potential issue identifiers from a string (branch name or directory name).
///
/// Looks for patterns like:
/// - `PROJ-123` (Jira/Linear style: uppercase letters + hyphen + digits)
/// - `#123` or just a bare number at the end after a separator (GitHub issue style)
///
/// Returns all matches found, most specific first.
pub fn extract_issue_ids(input: &str) -> Vec<String> {
    let mut ids = Vec::new();

    // Match Jira/Linear-style identifiers: 2+ uppercase letters, hyphen, digits
    // e.g. PROJ-123, ENG-456, AB-1
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        // Find a run of uppercase ASCII letters (at least 1)
        let start = i;
        while i < len && bytes[i].is_ascii_uppercase() {
            i += 1;
        }
        let letter_count = i - start;
        if letter_count >= 1 && i < len && bytes[i] == b'-' {
            i += 1; // skip the hyphen
            let digit_start = i;
            while i < len && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i > digit_start {
                // Check boundaries: the character before the letters should not be
                // alphanumeric (or start should be 0), and the character after digits
                // should not be alphanumeric.
                let left_ok = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
                let right_ok = i >= len || !bytes[i].is_ascii_alphanumeric();
                if left_ok && right_ok {
                    let id = input[start..i].to_string();
                    if !ids.contains(&id) {
                        ids.push(id);
                    }
                }
            }
        } else if i == start {
            i += 1;
        }
    }

    // Match GitHub-style issue numbers: look for #NNN or a bare number after
    // common separators (/, -, _)
    let parts: Vec<&str> = input.split(['/', '-', '_']).collect();
    for part in &parts {
        // #123 style
        if let Some(num_str) = part.strip_prefix('#') {
            if num_str.chars().all(|c| c.is_ascii_digit()) && !num_str.is_empty() {
                let id = format!("#{}", num_str);
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
        }
    }
    // Also check if the last segment is a bare number (e.g. feature/fix/123)
    if let Some(last) = parts.last() {
        if last.chars().all(|c| c.is_ascii_digit()) && !last.is_empty() {
            let id = format!("#{}", last);
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }

    ids
}

/// Get the current GitHub user via `gh api user --jq .login`.
pub fn detect_gh_user() -> Option<String> {
    let output = Command::new("gh")
        .args(["api", "user", "--jq", ".login"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let user = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if user.is_empty() {
        None
    } else {
        Some(user)
    }
}
