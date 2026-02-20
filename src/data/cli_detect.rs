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
