# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Test via Docker (preferred)

All builds and tests must be run inside the Docker container. The container cross-compiles from Linux for Windows (`x86_64-pc-windows-gnu`). Cargo/Rust is not installed on the host machine.

```bash
# Full release build — copies assoc.exe to target/x86_64-pc-windows-gnu/release/ locally
./build.sh

# Or manually: build image then export binary via BuildKit
docker build -t assoc-build --target builder .
DOCKER_BUILDKIT=1 docker build --target export \
    --output "type=local,dest=target/x86_64-pc-windows-gnu/release" .

# Run cargo check (fast compile check, no binary output)
docker run --rm -v "C:/dev/associate:/app" -w //app assoc-build cargo check --target x86_64-pc-windows-gnu

# Run tests
docker run --rm -v "C:/dev/associate:/app" -w //app assoc-build cargo test --target x86_64-pc-windows-gnu

# Run clippy lint
docker run --rm -v "C:/dev/associate:/app" -w //app assoc-build cargo clippy --target x86_64-pc-windows-gnu

# Run rustfmt check
docker run --rm -v "C:/dev/associate:/app" -w //app assoc-build cargo fmt -- --check
```

**Do not attempt to run cargo directly on the host.** Use Docker for all Rust commands.

The Dockerfile uses two stages:
- `builder` — compiles the project; used as the base for `cargo check/test/clippy`
- `export` — a scratch stage containing only `assoc.exe`; used with `--output` to copy the binary locally

Release profile uses `strip = true`, `lto = true`, `opt-level = "z"` (size-optimized).

## Build (legacy — host only, requires local toolchain)

```bash
export PATH="/c/Users/Keith/.cargo/bin:$PATH:/c/msys64/mingw64/bin" && cargo build --release
```

Requires the `stable-x86_64-pc-windows-gnu` toolchain with MinGW via MSYS2. MSVC tools are not available.

## After Implementing a Plan or Code Changes

Always run a full release build when finished implementing a plan or any code changes:

```bash
./build.sh
```

This ensures the binary is up to date and catches any issues not caught by `cargo check`.

## Documentation Requirements

**A change is not complete until the documentation is updated.** Every feature addition, behavior change, new configuration option, new keyboard shortcut, new tab, or UI change **must** include corresponding updates to the relevant documentation files before the work is considered done.

The project has three documentation surfaces that must be kept in sync:

| File | Purpose | When to update |
|------|---------|---------------|
| `README.md` | Primary user-facing documentation (install, usage, config, shortcuts, tabs, architecture) | Any user-visible change: new features, new CLI options, new keyboard shortcuts, new/changed configuration keys, new tabs, changed behavior |
| `docs/docs.html` | Full HTML documentation site (hosted) | Same as README — mirrors all user-facing content in the HTML documentation page |
| `docs/index.html` | Landing page / marketing site | New features, new tabs, new integrations, or anything that changes the product's capabilities or value proposition |

### Rules

1. **No code-only PRs for user-visible changes.** If a change affects what the user sees, how they interact with the app, or what they can configure, the corresponding docs must be updated in the same commit or PR.
2. **README.md is the source of truth.** Write the content in `README.md` first, then propagate to `docs/docs.html`. The landing page (`docs/index.html`) should reflect new capabilities but does not need to be a 1:1 copy.
3. **Update all three files when adding:**
   - A new tab or removing an existing one
   - A new keyboard shortcut or changing an existing one
   - A new `.assoc.toml` configuration key
   - A new CLI flag or subcommand
   - A new external integration (e.g., a new issue tracker)
4. **Update README.md and docs/docs.html when changing:**
   - Existing behavior or defaults
   - Architecture or data flow
   - Build or installation instructions
5. **Update docs/index.html when adding:**
   - A headline feature or integration that should be marketed
   - Changes to supported platforms or prerequisites

## Lint

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
