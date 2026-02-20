use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;

use crate::data::plans;
use crate::model::filebrowser::{EntryKind, FileBrowserEntry, FileContent};

const MAX_DEPTH: usize = 20;

/// Build a flat list of directory entries from `root`, expanding only directories in `expanded`.
/// Directories first, then files, case-insensitive sort. Respect .gitignore.
pub fn build_tree(root: &Path, expanded: &HashSet<PathBuf>) -> Result<Vec<FileBrowserEntry>> {
    let ignored = load_git_ignored_set(root);
    let mut result = Vec::new();
    collect_children(root, root, expanded, 0, &ignored, &mut result)?;
    Ok(result)
}

fn collect_children(
    root: &Path,
    dir: &Path,
    expanded: &HashSet<PathBuf>,
    depth: usize,
    ignored: &HashSet<PathBuf>,
    result: &mut Vec<FileBrowserEntry>,
) -> Result<()> {
    if depth >= MAX_DEPTH {
        return Ok(());
    }
    let entries = list_dir_entries(dir, depth)?;
    for entry in entries {
        if is_ignored(root, &entry.path, ignored) {
            continue;
        }
        let is_dir = entry.kind == EntryKind::Directory;
        let path = entry.path.clone();
        result.push(entry);

        if is_dir && expanded.contains(&path) {
            collect_children(root, &path, expanded, depth + 1, ignored, result)?;
        }
    }
    Ok(())
}

fn list_dir_entries(dir: &Path, depth: usize) -> Result<Vec<FileBrowserEntry>> {
    let read_dir = std::fs::read_dir(dir)?;

    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for entry in read_dir {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();

        if metadata.is_dir() {
            dirs.push(FileBrowserEntry {
                name,
                path,
                kind: EntryKind::Directory,
                size: 0,
                depth,
            });
        } else {
            let size = metadata.len();
            files.push(FileBrowserEntry {
                name,
                path,
                kind: EntryKind::File,
                size,
                depth,
            });
        }
    }

    // Sort each group case-insensitively by name
    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    // Combine: dirs first, then files
    dirs.extend(files);
    Ok(dirs)
}

/// Load all git-ignored file paths in one batch call.
fn load_git_ignored_set(root: &Path) -> HashSet<PathBuf> {
    let output = Command::new("git")
        .args([
            "ls-files",
            "--others",
            "--ignored",
            "--exclude-standard",
            "--directory",
        ])
        .current_dir(root)
        .output();

    let mut set = HashSet::new();
    if let Ok(output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let line = line.trim_end_matches('/');
                if !line.is_empty() {
                    set.insert(root.join(line));
                }
            }
        }
    }
    set
}

/// Check if a path should be ignored (either .git dir or in the ignored set).
fn is_ignored(root: &Path, path: &Path, ignored: &HashSet<PathBuf>) -> bool {
    // Always ignore .git directory itself
    if let Some(name) = path.file_name() {
        if name == ".git" {
            return true;
        }
    }

    // Check against the pre-loaded ignored set
    if ignored.contains(path) {
        return true;
    }

    // Also check if any ancestor is ignored (for nested paths inside ignored dirs)
    if let Ok(rel) = path.strip_prefix(root) {
        let mut check = root.to_path_buf();
        for component in rel.components() {
            check.push(component);
            if ignored.contains(&check) {
                return true;
            }
        }
    }

    false
}

/// Read file content for display.
pub fn read_file_content(path: &Path) -> Result<FileContent> {
    let metadata = std::fs::metadata(path)?;

    // If file > 1MB, return TooLarge
    if metadata.len() > 1_048_576 {
        return Ok(FileContent::TooLarge);
    }

    // Try to read as raw bytes first
    let bytes = std::fs::read(path)?;

    // Try to interpret as UTF-8
    let text = match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => return Ok(FileContent::Binary),
    };

    // If .md extension, parse with markdown parser
    let is_markdown = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("md"))
        .unwrap_or(false);

    if is_markdown {
        let lines = plans::parse_markdown_lines(&text);
        Ok(FileContent::Markdown(lines))
    } else {
        let lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
        Ok(FileContent::Text(lines))
    }
}

/// Save edited file content.
pub fn save_file(path: &Path, content: &str) -> Result<()> {
    std::fs::write(path, content)?;
    Ok(())
}
