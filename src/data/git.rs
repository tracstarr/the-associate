use std::path::Path;
use std::process::Command;

use anyhow::Result;

use crate::model::git::{DiffLine, DiffLineKind, GitFileEntry, GitFileSection, GitStatus};

/// Load git status by running `git status --porcelain` in the given directory.
/// Returns an empty GitStatus if git is not available or cwd is not a repo.
pub fn load_git_status(cwd: &Path) -> Result<GitStatus> {
    let output = match Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(cwd)
        .output()
    {
        Ok(o) => o,
        Err(_) => return Ok(GitStatus::default()),
    };

    if !output.status.success() {
        return Ok(GitStatus::default());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut status = GitStatus::default();

    for line in stdout.lines() {
        if line.len() < 3 {
            continue;
        }

        let bytes = line.as_bytes();
        let index_char = bytes[0] as char;
        let worktree_char = bytes[1] as char;
        let path_str = &line[3..];
        let path = if index_char == 'R' || index_char == 'C' {
            path_str
                .split(" -> ")
                .last()
                .unwrap_or(path_str)
                .to_string()
        } else {
            path_str.to_string()
        };

        // Untracked
        if index_char == '?' && worktree_char == '?' {
            status.untracked.push(GitFileEntry {
                path,
                section: GitFileSection::Untracked,
                status_char: '?',
            });
            continue;
        }

        // Staged: non-space in index column
        if index_char != ' ' && index_char != '?' {
            status.staged.push(GitFileEntry {
                path: path.clone(),
                section: GitFileSection::Staged,
                status_char: index_char,
            });
        }

        // Unstaged: non-space in worktree column
        if worktree_char != ' ' && worktree_char != '?' {
            status.unstaged.push(GitFileEntry {
                path,
                section: GitFileSection::Unstaged,
                status_char: worktree_char,
            });
        }
    }

    Ok(status)
}

/// Load diff for a specific file entry.
pub fn load_diff(cwd: &Path, entry: &GitFileEntry) -> Result<Vec<DiffLine>> {
    match entry.section {
        GitFileSection::Staged => load_git_diff(cwd, &entry.path, true),
        GitFileSection::Unstaged => load_git_diff(cwd, &entry.path, false),
        GitFileSection::Untracked => load_untracked_content(cwd, &entry.path),
    }
}

fn load_git_diff(cwd: &Path, file_path: &str, staged: bool) -> Result<Vec<DiffLine>> {
    let mut args = vec!["diff"];
    if staged {
        args.push("--cached");
    }
    args.push("--");
    args.push(file_path);

    let output = Command::new("git").args(&args).current_dir(cwd).output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_diff_output(&stdout))
}

fn load_untracked_content(cwd: &Path, file_path: &str) -> Result<Vec<DiffLine>> {
    let full_path = cwd.join(file_path);

    // Size check: if file > 1MB, don't read it
    if let Ok(metadata) = std::fs::metadata(&full_path) {
        if metadata.len() > 1_048_576 {
            return Ok(vec![DiffLine {
                kind: DiffLineKind::Header,
                text: "(file too large to display)".to_string(),
            }]);
        }
    }

    // Read as bytes first for binary detection
    let bytes = match std::fs::read(&full_path) {
        Ok(b) => b,
        Err(_) => {
            return Ok(vec![DiffLine {
                kind: DiffLineKind::Header,
                text: "(cannot read file)".to_string(),
            }])
        }
    };

    // Binary detection: check for null bytes
    if bytes.contains(&0) {
        return Ok(vec![DiffLine {
            kind: DiffLineKind::Header,
            text: "(binary file)".to_string(),
        }]);
    }

    let content = String::from_utf8_lossy(&bytes);

    let mut lines = Vec::new();
    lines.push(DiffLine {
        kind: DiffLineKind::Header,
        text: format!("new file: {}", file_path),
    });

    for (i, line) in content.lines().enumerate() {
        if i >= 200 {
            lines.push(DiffLine {
                kind: DiffLineKind::Context,
                text: "... (truncated at 200 lines)".to_string(),
            });
            break;
        }
        lines.push(DiffLine {
            kind: DiffLineKind::Add,
            text: format!("+{}", line),
        });
    }

    Ok(lines)
}

fn parse_diff_output(output: &str) -> Vec<DiffLine> {
    output
        .lines()
        .map(|line| {
            let kind = if line.starts_with("diff ")
                || line.starts_with("index ")
                || line.starts_with("--- ")
                || line.starts_with("+++ ")
            {
                DiffLineKind::Header
            } else if line.starts_with("@@") {
                DiffLineKind::Hunk
            } else if line.starts_with('+') {
                DiffLineKind::Add
            } else if line.starts_with('-') {
                DiffLineKind::Remove
            } else {
                DiffLineKind::Context
            };

            DiffLine {
                kind,
                text: line.to_string(),
            }
        })
        .collect()
}
