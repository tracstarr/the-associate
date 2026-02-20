# Linear Tab Implementation Plan

## Overview
Add a new "Linear" tab to the TUI dashboard that fetches issues from the Linear GraphQL API. The tab follows the existing Jira tab pattern (left pane = issue list grouped by status, right pane = issue detail). Visibility is conditional on having a Linear API key configured.

## Configuration (`.assoc.toml`)

```toml
[linear]
api_key = "lin_api_..."        # Required — Linear personal API key
username = "john@example.com"  # Optional — filter by assignee email (defaults to viewer/authenticated user)
team = "ENG"                   # Optional — filter by team key
```

A new `LinearConfig` struct in `config.rs` alongside the existing `GithubConfig`/`JiraConfig`.

## New Files

### 1. `src/model/linear.rs` — Data types
- `LinearIssue` — identifier, title, description, priority_label, state_name, state_type, assignee_name, labels, url, team_name
- `FlatLinearItem` — enum with `StatusHeader(String)` and `Issue(LinearIssue)` variants (same pattern as Jira)
- Helper method `priority_icon()` for display

### 2. `src/data/linear.rs` — API client
- Uses `std::process::Command` to run `curl` with the Linear GraphQL endpoint (`https://api.linear.app/graphql`)
- `fetch_my_issues(api_key, username, team)` — builds a GraphQL query using `viewer { assignedIssues }` (or filtered by email/team), parses JSON response
- `categorize_issues()` — groups issues by workflow state into `FlatLinearItem` list
- Timeout handling matching the existing Jira pattern (30s timeout with polling)

### 3. `src/ui/linear_view.rs` — UI rendering
- `draw_linear()` — 40/60 split, left = issue list, right = detail
- Same pattern as `jira_view.rs`: status headers with color coding, issue list with priority icons, detail pane with all fields

## Modified Files

### 4. `src/config.rs`
- Add `LinearConfig` struct with `api_key`, `username`, `team` fields
- Add `linear: Option<LinearConfig>` to `ProjectConfig`
- Add accessor methods: `linear_api_key()`, `linear_username()`, `linear_team()`

### 5. `src/model/mod.rs`
- Add `pub mod linear;`

### 6. `src/data/mod.rs`
- Add `pub mod linear;`

### 7. `src/app.rs`
- Add `ActiveTab::Linear` variant
- Add `LinearPane` enum (List, Detail)
- Add Linear state fields to `App`: `has_linear`, `linear_issues`, `linear_flat_list`, `linear_index`, `linear_pane`, `linear_detail_scroll`, `linear_last_poll`
- Add `load_linear_issues()` method
- Add Linear to `visible_tabs()` (conditional on `has_linear`)
- Add Linear to `load_all()`
- Add Linear branches to `navigate_down/up/left/right`, `select_item`, `jump_top/bottom`
- Add Linear skip helpers: `linear_skip_to_next_issue()`, `linear_skip_to_prev_issue()`, `linear_skip_to_issue_entry()`, `linear_selected_issue()`, `linear_open_selected()`

### 8. `src/ui/mod.rs`
- Add `pub mod linear_view;`

### 9. `src/ui/layout.rs`
- Import `linear_view`
- Add `ActiveTab::Linear` to `draw_content()` match
- Add Linear hint text
- Add Linear status bar indicators if needed

### 10. `src/ui/tabs.rs`
- Add `ActiveTab::Linear` label formatting

### 11. `src/ui/theme.rs`
- Add Linear-specific styles: `LINEAR_URGENT`, `LINEAR_HIGH`, `LINEAR_MEDIUM`, `LINEAR_LOW`, `LINEAR_STARTED`, `LINEAR_UNSTARTED`, `LINEAR_COMPLETED`

### 12. `src/main.rs`
- Add Linear polling in tick loop (every 60s, same as GitHub/Jira)
- Add `o` key handler for opening Linear issues in browser
- Add `r` key handler for refreshing Linear issues

### 13. `src/ui/help_overlay.rs`
- Add Linear-specific keybindings to help text

### 14. `Cargo.toml`
- No new dependencies needed — uses existing `serde_json` for JSON parsing and `std::process::Command` for curl

## Data Flow
1. On startup, check if `[linear]` config exists with `api_key` → set `has_linear = true`
2. If `has_linear`, call `fetch_my_issues()` which runs `curl` to POST GraphQL query
3. Parse response JSON into `Vec<LinearIssue>`
4. Group into `Vec<FlatLinearItem>` by workflow state
5. Poll every 60s in the tick loop
6. No file watcher needed — data comes from API, not filesystem

## GraphQL Query
```graphql
query {
  viewer {
    assignedIssues(
      filter: { state: { type: { nin: ["canceled", "completed"] } } }
      first: 50
      orderBy: updatedAt
    ) {
      nodes {
        identifier
        title
        description
        priority
        priorityLabel
        state { name type color }
        assignee { name email }
        labels { nodes { name color } }
        url
        team { name key }
        createdAt
        updatedAt
      }
    }
  }
}
```

When `team` is configured, add team filter. When `username` is configured, use `issues` query with assignee email filter instead of `viewer.assignedIssues`.
