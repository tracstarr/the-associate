use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::event::AppEvent;

/// Send text to the Claude Code pane asynchronously.
///
/// This uses a three-step approach:
/// 1. Copy text to the clipboard via PowerShell `Set-Clipboard`
/// 2. Focus the Claude Code pane via `wt.exe -w 0 move-focus <direction>`
/// 3. Paste (Ctrl+V) and press Enter via PowerShell `SendKeys`
/// 4. Refocus back to The Associate pane
///
/// The result is sent back through the event channel.
pub fn send_to_claude_pane(text: String, direction: &str, tx: mpsc::Sender<AppEvent>) {
    let dir = direction.to_string();
    thread::spawn(move || {
        let result = do_send(&text, &dir);
        let msg = match result {
            Ok(()) => None,
            Err(e) => Some(e.to_string()),
        };
        let _ = tx.send(AppEvent::PaneSendComplete(msg));
    });
}

fn opposite_direction(direction: &str) -> &str {
    match direction {
        "right" => "left",
        "left" => "right",
        "up" => "down",
        "down" => "up",
        _ => "left",
    }
}

fn do_send(text: &str, direction: &str) -> anyhow::Result<()> {
    // Step 1: Copy text to clipboard via PowerShell
    let escaped = text.replace('\'', "''");
    let ps_clip = format!("Set-Clipboard -Value '{}'", escaped);
    let status = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_clip])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;
    if !status.success() {
        anyhow::bail!("Failed to set clipboard");
    }

    // Step 2: Focus the Claude Code pane
    let status = Command::new("wt.exe")
        .args(["-w", "0", "move-focus", direction])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;
    if !status.success() {
        anyhow::bail!("Failed to focus Claude pane (move-focus {})", direction);
    }

    // Step 3: Wait for focus change to take effect
    thread::sleep(Duration::from_millis(300));

    // Step 4: Paste (Ctrl+V) then press Enter via SendKeys
    let ps_send = concat!(
        "Add-Type -AssemblyName System.Windows.Forms; ",
        "[System.Windows.Forms.SendKeys]::SendWait('^v'); ",
        "Start-Sleep -Milliseconds 200; ",
        "[System.Windows.Forms.SendKeys]::SendWait('{ENTER}')"
    );
    let send_result = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps_send])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    // Step 5: Always refocus back, even if SendKeys failed
    thread::sleep(Duration::from_millis(300));
    let back = opposite_direction(direction);
    let _ = Command::new("wt.exe")
        .args(["-w", "0", "move-focus", back])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    match send_result {
        Ok(s) if s.success() => Ok(()),
        Ok(_) => anyhow::bail!("SendKeys failed"),
        Err(e) => anyhow::bail!("SendKeys error: {}", e),
    }
}
