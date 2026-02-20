# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build

```bash
export PATH="/c/Users/Keith/.cargo/bin:$PATH:/c/msys64/mingw64/bin" && cargo build --release
```

Requires the `stable-x86_64-pc-windows-gnu` toolchain with MinGW via MSYS2. MSVC tools are not available.

Release profile uses `strip = true`, `lto = true`, `opt-level = "z"` (size-optimized).

## Test

```bash
cargo test
```

Run a single test:
```bash
cargo test test_simple_path
```

Only `src/data/path_encoding.rs` has tests currently.

## Lint

```bash
cargo clippy
cargo fmt -- --check
```

No custom rustfmt or clippy configuration — defaults apply.

## Architecture

The Associate is a read-only TUI dashboard (ratatui + crossterm) that monitors Claude Code session data from `~/.claude/`. It does not write to any Claude Code files.

### Layers

- **`src/model/`** — Serde-derived data types. JSON fields use `#[serde(rename_all = "camelCase")]` to match Claude Code's format.
- **`src/data/`** — File loading and parsing. Each module reads from a specific `~/.claude/` subdirectory. Errors are surfaced in the status bar, not propagated as panics.
- **`src/ui/`** — Stateless render functions. Each tab has its own `*_view.rs`. Styles live in `theme.rs`.
- **`src/watcher/`** — Debounced file watcher (notify crate, 200ms). Classifies changes into `FileChange` variants so the app reloads only affected data.
- **`src/app.rs`** — Central `App` struct holding all state: tab selection, list indices, scroll positions, loaded data. Handles navigation and data reload logic.
- **`src/main.rs`** — CLI (clap) with two modes: `run_tui()` for the dashboard, `launch_wt()` for opening Windows Terminal with Claude Code + dashboard side-by-side.

### Data flow

1. File watcher detects change → sends `FileChange` through mpsc channel
2. `run_app()` event loop receives it alongside key events and tick timer
3. `App` reloads the specific data (sessions, transcript, teams, tasks, etc.)
4. UI re-renders from `App` state on every frame

### Tabs and panes

Each tab uses a left/right pane pattern (list on left, detail on right). Navigation: `h/l` switches panes, `j/k` scrolls within a pane.

| Tab | Left pane | Right pane |
|-----|-----------|------------|
| Sessions | Session list | Transcript viewer (with follow mode `f`, subagent cycling `s`) |
| Teams | Teams → Members → Tasks (nested drill-down) | Task/inbox detail |
| Todos | Todo file list | Todo items |
| Git | Staged/unstaged/untracked files | Diff viewer |
| Plans | Plan file list | Markdown content |

### Path encoding

Windows paths are encoded for `~/.claude/projects/` lookups: `C:\dev\foo` → `C--dev-foo` (replace `:\` with `--`, then `\` with `-`). See `src/data/path_encoding.rs`.

### Transcript parsing

JSONL files are read incrementally via `TranscriptReader` which tracks byte offset. Initial load reads the last 200 lines; subsequent reads parse only new data. Each line is a `TranscriptEnvelope` with `type` (user, assistant, system, progress) and nested content blocks (text, tool_use, tool_result).
