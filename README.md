# The Associate

A real-time TUI dashboard for monitoring Claude Code sessions, teams, git status, and more — built natively for Windows with Rust, [ratatui](https://github.com/ratatui/ratatui), and [crossterm](https://github.com/crossterm-rs/crossterm).

## Prerequisites

- **Rust toolchain** — Install via [rustup](https://rustup.rs/). On Windows, use the GNU target: `stable-x86_64-pc-windows-gnu`.
- **MSYS2 with MinGW64** — Required for the GNU linker. Install [MSYS2](https://www.msys2.org/) and ensure `C:\msys64\mingw64\bin` is on your PATH.
- **Claude Code CLI** — The Associate reads data from `~/.claude/` that Claude Code produces during sessions.
- **Windows Terminal** (recommended) — Required for the `assoc launch` side-by-side mode. Available from the Microsoft Store.

### Optional integrations

- **GitHub CLI (`gh`)** — Enables the PRs and Issues tabs. Must be authenticated via `gh auth login`.
- **Atlassian CLI (`acli`)** — Enables the Jira tab. Must be configured with your Jira instance credentials.
- **Git** — Required for the Git tab's status and diff features.

## Installation

### Quick install (recommended)

Run this one-liner in PowerShell to download and install the latest release:

```powershell
irm https://raw.githubusercontent.com/tracstarr/the-associate/main/install.ps1 | iex
```

This downloads `assoc.exe` from the latest GitHub release and installs it to `%LOCALAPPDATA%\bin`, adding it to your user PATH automatically.

### Update

Re-run the install command to update to the latest release:

```powershell
irm https://raw.githubusercontent.com/tracstarr/the-associate/main/install.ps1 | iex
```

If you built from source, pull the latest changes and rebuild:

```bash
./install.sh update
```

### Uninstall

Remove The Associate and clean up the PATH entry:

```powershell
$env:ASSOC_ACTION='uninstall'; irm https://raw.githubusercontent.com/tracstarr/the-associate/main/install.ps1 | iex
```

If you built from source:

```bash
./install.sh uninstall
```

### Build from source

Clone the repository and build the release binary:

```bash
# Set up PATH for Rust + MinGW linker
export PATH="/c/Users/$USER/.cargo/bin:$PATH:/c/msys64/mingw64/bin"

# Build the optimized release binary
cargo build --release
```

The binary will be at `target/release/assoc.exe`. Copy it to a directory on your PATH for easy access. Alternatively, use the install script to build and install in one step:

```bash
./install.sh
```

> **Note:** The release profile uses `strip = true`, `lto = true`, and `opt-level = "z"` for a small, optimized binary.

## Usage

The Associate has two modes: a standalone TUI dashboard, and a side-by-side launch that opens Claude Code and the dashboard together in Windows Terminal.

### TUI Dashboard

Run `assoc` from your project directory to start the interactive dashboard:

```bash
# Monitor the current directory
assoc

# Monitor a specific project
assoc --cwd C:\dev\myproject
```

The dashboard opens in your terminal, showing real-time data from Claude Code's `~/.claude/` directory for the given project. All data updates automatically via a file watcher — no manual refresh needed.

### Side-by-Side Launch

The `launch` subcommand opens Windows Terminal with two panes: Claude Code on the left, The Associate on the right.

```bash
# Basic side-by-side launch
assoc launch

# Launch with a specific project directory
assoc launch --cwd C:\dev\myproject

# Resume a previous Claude Code session
assoc launch --resume abc123

# Adjust pane width ratio (0.0-1.0, default 0.5)
assoc launch --claude-ratio 0.6

# Set terminal dimensions
assoc launch --cols 220 --rows 55

# Pass extra arguments to Claude Code
assoc launch -- --dangerously-skip-permissions
```

#### Launch options

| Option | Default | Description |
|--------|---------|-------------|
| `--cwd <DIR>` | Current directory | Project directory to monitor |
| `--resume <ID>` | — | Resume a Claude Code session by ID |
| `--claude-ratio <FLOAT>` | `0.5` | Claude pane width as a fraction of the terminal |
| `--cols <N>` | `200` | Terminal width in columns |
| `--rows <N>` | `50` | Terminal height in rows |
| `-- <ARGS>` | — | Extra arguments passed through to Claude Code |

## Configuration

The Associate reads an optional `.assoc.toml` file from your project directory. This file lets you configure integrations and display settings without passing command-line flags.

```toml
# .assoc.toml - place in your project root

[github]
repo = "owner/repo-name"    # Override auto-detected GitHub repo

[github.issues]
enabled = true              # Set to false to hide the Issues tab
repo = "owner/repo-name"    # Override repo for issues specifically
state = "open"              # "open", "closed", or "all"

[jira]
project = "PROJ"             # Jira project key for filtering issues
jql = "assignee = currentUser() AND resolution = Unresolved"

[linear]
api_key = "lin_api_..."      # Linear personal API key (required)
username = "you@example.com" # Your Linear email for My Tasks grouping
team = "BIT"                 # Optional: filter to a specific team key

[display]
tick_rate = 250              # UI refresh interval in ms (default: 250)
tail_lines = 200             # Lines to load from end of transcript (default: 200)

[tabs]
sessions = true              # Set to false to disable the Sessions tab entirely
teams = true
todos = true
git = true
plans = true
github_prs = true
github_issues = true
jira = true
linear = true

# Custom prompts for the prompt picker (press 'p' on issue tabs)
[[prompts]]
title = "Fix Bug"
prompt = "Investigate and fix the bug described in this ticket."

[[prompts]]
title = "Code Review"
prompt = "Review the code changes related to this ticket and provide feedback."
```

### GitHub settings

| Key | Type | Description |
|-----|------|-------------|
| `github.repo` | String | GitHub repository in `owner/name` format. Overrides automatic detection from the git remote. |
| `github.issues.enabled` | Boolean | Set to `false` to hide the Issues tab even when `gh` is available. Default: `true`. |
| `github.issues.repo` | String | Override the repository used for the Issues tab specifically. Falls back to `github.repo`, then auto-detection. |
| `github.issues.state` | String | Filter issues by state: `"open"`, `"closed"`, or `"all"`. Default: `"open"`. |

### Jira settings

| Key | Type | Description |
|-----|------|-------------|
| `jira.project` | String | Jira project key (e.g. `PROJ`) used to filter displayed issues. |
| `jira.jql` | String | Custom JQL query for fetching issues. Overrides the default query. |

### Linear settings

| Key | Type | Description |
|-----|------|-------------|
| `linear.api_key` | String | Your Linear API key. Required to enable the Linear tab. Generate one at **Linear > Settings > API**. |
| `linear.username` | String | Your Linear account email address. Used to separate issues into **My Tasks** (assigned to you) and **Unassigned** sections. |
| `linear.team` | String | Linear team key (e.g. `BIT`) to filter issues to a specific team. Optional — omit to show issues across all teams. |

### Display settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `display.tick_rate` | Integer | `250` | How often the UI redraws, in milliseconds. |
| `display.tail_lines` | Integer | `200` | Number of lines loaded from the end of JSONL transcript files on initial read. Higher values load more history but use more memory. |

### Tabs settings

Set any tab to `false` to disable it entirely. Disabled tabs are hidden from the tab bar, their data is never loaded or polled, and their CLI tools are not detected at startup.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `tabs.sessions` | Boolean | `true` | Show the Sessions tab. |
| `tabs.teams` | Boolean | `true` | Show the Teams tab. |
| `tabs.todos` | Boolean | `true` | Show the Todos tab. |
| `tabs.git` | Boolean | `true` | Show the Git tab. |
| `tabs.plans` | Boolean | `true` | Show the Plans tab. |
| `tabs.github_prs` | Boolean | `true` | Show the PRs tab. When `false`, `gh` is not detected unless `tabs.github_issues` is also enabled. |
| `tabs.github_issues` | Boolean | `true` | Show the Issues tab. When `false`, `gh` is not detected unless `tabs.github_prs` is also enabled. |
| `tabs.jira` | Boolean | `true` | Show the Jira tab. When `false`, `acli` is not detected at startup. |
| `tabs.linear` | Boolean | `true` | Show the Linear tab. When `false`, the Linear API key is ignored and no polling occurs. |

### Custom Prompts

Define reusable prompt templates for the ticket-to-Claude launcher using the `[[prompts]]` array. Each entry has a `title` (shown in the picker) and a `prompt` (the text inserted into the editor).

```toml
[[prompts]]
title = "Fix Bug"
prompt = "Investigate and fix the bug described in this ticket."

[[prompts]]
title = "Code Review"
prompt = "Review the code changes related to this ticket and provide feedback."
```

When you press `p` on a PRs, Issues, Jira, or Linear tab:
- **Without custom prompts** — the prompt editor opens immediately with a default prompt generated from the ticket's title and description.
- **With custom prompts** — a picker overlay appears listing "Default (from ticket)" plus your custom prompts. Select one with `j`/`k` and `Enter`, or press `Esc` to cancel. The selected prompt is loaded into the editor for further editing before launch.

| Key | Type | Description |
|-----|------|-------------|
| `prompts[].title` | String | Display name shown in the prompt picker. |
| `prompts[].prompt` | String | The prompt text inserted into the editor when selected. |

## Keyboard Shortcuts

The Associate is fully keyboard-driven. Press `?` or `Ctrl+H` at any time to show the help overlay inside the TUI.

### Global

| Key | Action |
|-----|--------|
| `q` | Quit the application |
| `Ctrl+C` | Quit the application |
| `?` | Toggle the help overlay |
| `Ctrl+H` | Toggle the help overlay |
| `Esc` | Close help overlay (when open) |

### Navigation

| Key | Action |
|-----|--------|
| `Tab` | Next tab |
| `Shift+Tab` | Previous tab |
| `1` – `9` | Jump to tab by number |
| `j` / `Down` | Navigate down in list or scroll content down |
| `k` / `Up` | Navigate up in list or scroll content up |
| `h` / `Left` | Switch to left pane |
| `l` / `Right` | Switch to right pane |
| `Enter` | Select item or open content pane |
| `g` | Jump to top of list or content |
| `G` | Jump to bottom of list or content |

### Tab-Specific

| Key | Tab | Action |
|-----|-----|--------|
| `f` | Sessions | Toggle follow mode (auto-scroll to latest output) |
| `s` | Sessions | Cycle through subagent transcripts |
| `b` | Git | Toggle between git status view and file browser |
| `e` | Git (browser) | Edit the currently viewed file |
| `Ctrl+S` | Git (browser) | Save the file being edited |
| `Esc` | Git (browser) | Cancel editing |
| `Backspace` | Git (browser) | Collapse directory or navigate to parent |
| `p` | PRs / Issues / Jira / Linear | Open the prompt picker (if custom prompts are configured) or go straight to the prompt editor to compose and launch a Claude Code task from the selected ticket |
| `o` | PRs / Issues / Jira / Linear | Open the selected item in your web browser |
| `r` | PRs / Issues / Jira / Linear | Refresh data from the remote service |
| `n` | Issues | Create a new issue (opens editor popup) |
| `e` | Issues | Edit the selected issue's title and body |
| `c` | Issues | Add a comment to the selected issue |
| `x` | Issues | Close or reopen the selected issue |
| `x` | Processes | Kill the selected running process |
| `s` | Processes | Jump to the Sessions tab and load the transcript for the selected process |
| `d` / `Del` | Sessions / Teams / Todos / Plans | Delete the selected item (shows confirmation prompt) |
| `y` | Sessions / Teams / Todos / Plans | Confirm deletion when the prompt is active |
| `n` / `Esc` | Sessions / Teams / Todos / Plans | Cancel deletion prompt |
| `t` | Jira | Show available status transitions for selected issue |
| `/` | Jira | Enter search mode (type query, press Enter to search, Esc to cancel) |

## Tabs Reference

The Associate displays up to ten tabs. The first five are always visible; the PRs, Issues, Jira, Linear, and Processes tabs appear only when their respective tools are detected, configured, or actively used.

> **Pane pattern:** Every tab uses a left/right pane layout. The left pane shows a list; the right pane shows detail for the selected item. Use `h`/`l` to switch between panes.

### 1. Sessions

Displays all Claude Code sessions for the current project, sorted by most recent. The right pane shows the live transcript for the selected session.

- **Follow mode** (`f`) — When active, the transcript auto-scrolls to the latest output as Claude Code writes to the session file. Scrolling up manually disables follow mode; pressing `G` re-enables it.
- **Subagent cycling** (`s`) — If the session has spawned subagents (team members), press `s` to cycle through their individual transcripts. Press `s` again past the last subagent to return to the main transcript.
- **Incremental loading** — Only the last 200 lines (configurable via `display.tail_lines`) are loaded initially. New lines are read incrementally as they appear.
- **Delete** (`d` / `Del`) — Deletes the selected session's `.jsonl` transcript file from disk. A confirmation prompt appears; press `y` to confirm or `n` / `Esc` to cancel.

### 2. Teams

Monitors Claude Code multi-agent teams configured in `~/.claude/teams/`. Uses a four-pane drill-down: Teams > Members > Tasks > Detail.

- **Teams pane** — Lists all team configurations found for the current project.
- **Members pane** — Shows team members with their current status (starting, working, idle, shutdown). Lead agents are indicated.
- **Tasks pane** — Lists all tasks for the selected team, color-coded by status (pending, in progress, completed).
- **Detail pane** — Shows task details or inbox messages for the selected member.
- **Delete** (`d` / `Del`) — Removes the selected team's directory from `~/.claude/teams/`. A confirmation prompt appears; press `y` to confirm or `n` / `Esc` to cancel.

### 3. Todos

Aggregates all todo files from `~/.claude/todos/` into a unified view. Left pane lists todo files; right pane shows the individual items within the selected file.

- **Delete** (`d` / `Del`) — Deletes the selected `.json` todo file from `~/.claude/todos/`. A confirmation prompt appears; press `y` to confirm or `n` / `Esc` to cancel.

### 4. Git

Shows the git status for your project directory. Has two modes, toggled with `b`:

- **Status mode** (default) — Left pane shows staged, unstaged, and untracked files grouped by section. Right pane shows the diff for the selected file.
- **Browse mode** — A full file browser for navigating the project tree. Select files to preview their contents. Press `e` to edit, `Ctrl+S` to save, `Esc` to cancel.

### 5. Plans

Displays plan files from `~/.claude/`. Left pane lists available plan files; right pane renders the markdown content with syntax-aware formatting (headings, code blocks).

- **Delete** (`d` / `Del`) — Deletes the selected `.md` plan file from disk. A confirmation prompt appears; press `y` to confirm or `n` / `Esc` to cancel.

### 6. PRs

Shows open pull requests from the project's GitHub repository. Requires the `gh` CLI to be installed and authenticated.

- PRs are categorized into sections (e.g. authored by you, review requested, etc.).
- Review status is color-coded: approved (green), changes requested (red), pending review (yellow), draft (gray).
- A `*` badge appears on the tab name when new activity is detected.
- Data is polled every 60 seconds. Press `r` to refresh manually, `o` to open in your browser.
- Press `p` to open the prompt modal and launch a Claude Code task based on the selected PR.

> The repository is auto-detected from the git remote. Override it in `.assoc.toml` with `github.repo = "owner/name"`.

### 7. Issues

Displays GitHub issues for the current repository, categorized by assignment. Requires the `gh` CLI to be installed and authenticated. The tab appears automatically when `gh` is available and a GitHub repository is detected from the git remote.

- Issues are grouped into **Assigned to Me**, **My Issues** (authored), and **Other** sections.
- The right pane shows full issue details: state, author, assignees, labels, milestone, description, comments, and URL.
- Press `n` to create a new issue, `e` to edit the selected issue, `c` to add a comment, `x` to close or reopen.
- Press `o` to open the issue in your browser, `r` to refresh manually.
- Press `p` to open the prompt modal — a pre-filled editable prompt based on the issue title and description. Confirm with `Ctrl+Enter` to spawn a headless Claude Code process that works the issue autonomously. The dashboard switches to the Processes tab automatically.
- Data is polled every 60 seconds automatically.

> The repository is auto-detected from the git remote. You can override it or configure the state filter in `.assoc.toml` under `[github.issues]`.

### 8. Jira

Displays Jira issues for the current user. Requires the Atlassian CLI (`acli`) to be installed and configured.

- Issues are grouped by status (To Do, In Progress, Done) and color-coded by type (bug, story, task).
- Press `Enter` to load full issue details in the right pane.
- Press `t` to show available status transitions, then press a number key to execute a transition.
- Press `/` to search issues by text query. Press `Esc` to cancel search and return to the default view.
- Data is polled every 60 seconds. Press `r` to refresh manually, `o` to open in your browser.
- Press `p` to open the prompt modal and launch a Claude Code task from the selected Jira issue.

### 9. Linear

Displays Linear issues fetched from the Linear GraphQL API. Requires a `linear.api_key` in `.assoc.toml`. The tab appears automatically when an API key is configured.

- Issues are grouped into **My Tasks** (assigned to your configured email) and **Unassigned** sections, each sorted by workflow state (started first, then unstarted, then backlog).
- The right pane shows full issue details: identifier, title, state, priority, assignee, team, labels, description, and URL.
- Press `Enter` or `o` to open the selected issue in your browser.
- Press `r` to refresh data from the Linear API. Data is polled every 60 seconds automatically.
- Press `p` to open the prompt modal and launch a Claude Code task from the selected Linear issue.

> Configure `linear.username` with your Linear account email so that issues assigned to you are separated into the **My Tasks** section. Without it, only the **Unassigned** section is shown.

### 10. Processes

Tracks every headless Claude Code process spawned via the prompt modal (`p` on PRs, Issues, Jira, or Linear). The tab appears automatically when a process is launched and stays visible for the session.

- The left pane lists all spawned processes with a status icon: `*` running, `+` completed, `x` failed.
- The right pane shows a parsed, color-coded progress view: session link (magenta), tool calls (yellow), text snippets (white), and a final `[SUCCESS ($cost)]` or `[FAILED]` line.
- The output block title shows a short session ID suffix (`[sid:xxxxxxxx]`) once Claude Code emits the stream-json init event.
- Press `x` to kill the selected running process immediately.
- Press `s` to jump to the Sessions tab and load the full transcript for the selected process. This works once Claude Code has emitted its first stream-json event.

> Processes run with `--dangerously-skip-permissions` so they can operate fully autonomously. Review the generated prompt in the modal before confirming with `Ctrl+Enter`.

## Architecture

The Associate monitors `~/.claude/` for changes and re-renders the UI accordingly.

### Data flow

1. A debounced file watcher (200ms) monitors `~/.claude/` and the project's `.git/` directory.
2. When a change is detected, the watcher classifies it (session index, transcript, team config, task file, etc.) and sends a typed event through an internal channel.
3. The main event loop receives the event alongside keyboard input and a tick timer.
4. Only the affected data is reloaded — for example, a transcript change only reloads the transcript, not the teams or todos.
5. The UI re-renders from application state on every frame.

### Layers

- **`src/model/`** — Serde-derived data types that match Claude Code's JSON format (camelCase field names).
- **`src/data/`** — File loading and parsing. Each module reads from a specific subdirectory of `~/.claude/`.
- **`src/ui/`** — Stateless render functions. Each tab has its own view file. Theme constants live in `theme.rs`.
- **`src/watcher/`** — File watcher using the `notify` crate with debouncing.
- **`src/app.rs`** — Central `App` struct holding all state, navigation logic, and data reload methods.
- **`src/config.rs`** — Project configuration loading from `.assoc.toml`.

### Path encoding

Claude Code stores per-project data in `~/.claude/projects/` using an encoded form of the project's absolute path. The Associate uses the same encoding to locate the correct project directory:

```
# Encoding rule:
# 1. Replace ":\" with "--"
# 2. Replace remaining "\" with "-"

C:\dev\myproject  -->  C--dev-myproject
C:\Users\me\code  -->  C--Users-me-code
```

This encoding is handled internally by the `path_encoding` module. You do not need to perform this encoding yourself — just pass the actual project path via `--cwd` and The Associate resolves it automatically.
