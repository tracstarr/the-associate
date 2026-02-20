use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;

use anyhow::Result;

/// Message sent from a process reader thread back to the main event loop.
#[derive(Debug)]
pub enum ProcessOutput {
    /// A line of stdout from the process.
    Stdout(usize, String),
    /// A line of stderr from the process.
    Stderr(usize, String),
    /// The process has exited with the given success status.
    Exited(usize, bool),
}

/// Spawn `claude -p "<prompt>"` in headless mode.
///
/// Uses `--output-format stream-json` for streaming output and
/// `--dangerously-skip-permissions` to allow fully autonomous execution.
///
/// Returns the child process handle. Output is sent via `tx` on background
/// threads so the TUI event loop can poll it non-blockingly.
pub fn spawn_claude_headless(
    process_id: usize,
    prompt: &str,
    cwd: &Path,
    tx: mpsc::Sender<ProcessOutput>,
) -> Result<Child> {
    let mut child = Command::new("claude")
        .args([
            "-p",
            prompt,
            "--dangerously-skip-permissions",
            "--output-format",
            "stream-json",
            "--verbose",
        ])
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .spawn()?;

    // Spawn thread to read stdout
    let stdout = child.stdout.take().expect("stdout was piped");
    let tx_out = tx.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(text) => {
                    if tx_out
                        .send(ProcessOutput::Stdout(process_id, text))
                        .is_err()
                    {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Spawn thread to read stderr
    let stderr = child.stderr.take().expect("stderr was piped");
    let tx_err = tx.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(text) => {
                    if tx_err
                        .send(ProcessOutput::Stderr(process_id, text))
                        .is_err()
                    {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Spawn thread to wait for process exit
    let child_id = child.id();
    thread::spawn(move || {
        // We need to wait for the child to exit.
        // Since we took stdout/stderr, the child handle is still valid for wait.
        // However, we don't have the child handle here. We'll handle exit detection
        // by detecting EOF on stdout/stderr readers above, then checking via try_wait
        // in the main loop.
        let _ = child_id; // just to prevent unused warning
    });

    Ok(child)
}
