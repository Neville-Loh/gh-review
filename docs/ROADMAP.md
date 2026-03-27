# gh-review Roadmap

## Overview

| Milestone | Description | Status |
|-----------|-------------|--------|
| M1 — Read-only Diff Viewer | Parse diffs, unified + side-by-side rendering, file picker | done |
| M2 — Review Actions | Inline comments, pending review submit, expand context, syntax highlighting | done |
| M2.6 — Search | Regex search with smart-case, match highlighting, file picker filter | done |
| M3 — Full Review Comments | Resolve/unresolve threads, suggestion diffs, review body, unapprove | done |
| M7 — User Configuration | TOML config, remappable keybindings, custom actions | done |
| M4 — Claude Review | AI-powered code review via Claude API, inline comment display | **next** |
| M4.5 — PR Description Panel | View and edit PR description, panel navigation between diff/files/description | planned |
| M5 — Graphite Stacked PRs | Stack detection, navigate between PRs, diff against parent branch | planned |
| M6 — Polish | Word-level diff, status line, custom themes | later |
| M8 — gh-dash-rs Integration | Library crate extraction, native view inside gh-dash Rust rewrite | future |
| M9 — AI Chat Panel | Side-by-side chat panel for discussing code with Claude while reviewing | future |

## Milestones

### M1 — Read-only Diff Viewer (done)

- Parse GitHub patch format into structured hunks
- Unified and side-by-side rendering with syntax-colored +/- lines
- Dual-number gutters (old line / new line)
- File list sidebar with status indicators and +/- counts
- Keyboard navigation: scroll, page, jump to file, toggle view mode
- CLI aliases — invoke with a PR URL or just a PR number when inside the repo
- Debug mode — `--debug` flag to dump resolved config and diagnostics
- Cross-platform support (macOS, Linux, Windows)

### M2 — Review Actions (done)

- Inline comment textarea anchored to cursor line
- Pending review model — batch comments, submit as one review
- Approve, request changes, and comment-only submission with confirmation popup
- Existing review comments displayed inline in the diff with box-drawing thread rendering
- Expandable context — fetch full file content and splice +10 lines; row context expansion with line numbers
- Expand/collapse multi-line comments with Enter; bulk expand/collapse all
- Vim-style navigation (gg, G, H/M/L, ]/[, zz/zt/zb, Ctrl+F/B) with smooth scroll animation
- Syntax highlighting via tree-sitter (arborium, GitHub Dark theme) for unified and side-by-side views
- Command mode — vim-style `:` palette with tab-completion, fuzzy matching, and inline docs
- Collapsible files — `zo`/`zc` or Enter on file header to hide reviewed diffs
- Dynamic keybinding hints that update based on current context and active mode
- Enter to save comments (instead of inserting a newline)
- Open PR in browser (`o`)
- Clean process shutdown (works as gh-dash subprocess)

### M2.6 — Search (done)

Vim-style search across diff content and file names.

**Diff search (`/` and `?`)**
- `/` opens a search prompt at the bottom of the screen (forward search)
- `?` opens search in reverse direction (in diff view)
- Regex patterns with smart-case (case-insensitive unless pattern contains uppercase)
- Invalid regex silently escaped to a literal match
- All matches highlighted in the diff viewport; current match gets a distinct style
- `n` jumps to next match, `N` jumps to previous match; wraps at boundaries
- `Esc` cancels search and restores cursor to pre-search position
- `Enter` confirms search; match count displayed in search bar (`[3/12]`)

**File picker filter**
- When file picker is focused, `/` activates a filter prompt
- Filter against file paths; list updates as you type
- `j`/`k` navigate filtered results, `Enter` to select, `Esc` to cancel

**Resolved keybinding decisions**
- `n`/`N` are dual-purpose: search navigation when a search is active, file navigation otherwise
- `?` opens backward search in diff view, shows help overlay in file picker
- Help is also available via `F1` in all contexts

### M3 — Full Review Comments (done)

Complete the review comment workflow to cover all standard GitHub review operations.

**Resolve / unresolve threads**
- Cursor on a comment thread, press a key to resolve (hide) or unresolve (unhide)
- Uses the GitHub GraphQL API to minimize/resolve the thread
- Resolved threads shown as collapsed with a visual indicator

**Suggestion diffs**
- Press `e` on a diff line to open the line content in an editable textarea
- External editor support — uses `$EDITOR` when available, falls back to built-in text field
- Edit freely — on save, compute the diff between original and edited text and auto-generate the GitHub ```` ```suggestion ```` block
- Existing suggestion comments rendered as rich inline diffs (not plain markdown fences)
- Accept suggestion: apply as a commit directly from the TUI via GitHub API

**Review submission with body**
- When pressing `a` (approve), `r` (request changes), or `s` (comment), a textarea opens for the review body before submitting
- Body is optional — submit empty to skip, just like the GitHub web UI
- Pending comments are listed in the confirmation popup as a summary

**Unapprove**
- Dismiss your own prior approval via the GitHub API
- Keybinding to unapprove with an optional body explaining why

**Pending comment management**
- Discard pending comment — cursor on a pending comment, press `x` to remove from the pending review
- Edit pending comment — cursor on a pending comment, press `c` to re-open the textarea pre-filled with the existing body

**Multi-line comment selection** (pulled forward from M6)
- Press `v` to enter visual select mode (vim-style)
- Navigate to extend the selection; highlighted range shown in the diff
- Press `c` to comment on the selected line range
- `Esc` or `v` again to cancel visual selection
- GitHub API `start_line` and `start_side` fields used for multi-line comment ranges

### M7 — User Configuration (done)

User-facing config file (`~/.config/gh-review/config.toml`) for personalizing the tool without recompiling.

**Config file**
- TOML config at `~/.config/gh-review/config.toml` (XDG-compliant)
- CLI flags override config values
- Sensible defaults when no config file exists

**Remappable keybindings**
- Every action (scroll, comment, submit, search, etc.) can be rebound
- Config section `[keys]` with action-name = key-combo mapping
- Support modifier combinations (Ctrl, Alt, Shift)
- Validation on startup — warn on conflicts or unknown actions

```toml
[keys]
scroll_down = "j"
scroll_up = "k"
submit_approve = "a"
search_forward = "/"
next_file = "n"
```

**Custom actions**
- Define custom actions that run shell commands on review lifecycle events
- Actions receive context as environment variables (`GH_REVIEW_REPO`, `GH_REVIEW_PR`, `GH_REVIEW_ACTION`)
- Bind custom actions to any key via the keybinding system
- Async execution — actions run in background, don't block the UI

```toml
[[custom_action]]
name = "notify_approved"
command = "notify-send 'PR approved' '$GH_REVIEW_REPO#$GH_REVIEW_PR'"
key = "ctrl-shift-a"
```

### M4 — Claude Review (**next**)

AI-powered code review using Claude. Send the PR diff and context to Claude for automated review feedback displayed inline.

```mermaid
graph TD
    subgraph m4 [M4: Claude Review]
        CR1["Fetch PR diff + metadata"] --> CR2["Build review prompt with context"]
        CR2 --> CR3["Send to Claude API"]
        CR3 --> CR4["Parse structured review response"]
        CR4 --> CR5["Display AI comments inline in diff"]
        CR5 --> CR6["User can accept / dismiss / reply to AI comments"]
        CR7["--claude flag or keybinding"] --> CR1
    end
```

**Diff-based review**
- Send the unified diff, PR title, description, and file list to Claude
- Claude returns structured review comments (file, line, body, severity)
- AI comments displayed inline in the diff alongside human comments, visually distinct
- User can accept (convert to a real review comment), dismiss, or reply

**Integration**
- `--claude` CLI flag triggers AI review on PR load
- In-app keybinding to request Claude review on demand
- API key configured via environment variable (`ANTHROPIC_API_KEY`) or config file
- Rate limiting and cost awareness — show token usage in status bar

**Review quality**
- Context-aware: include file paths, hunk context, and PR description
- Configurable review focus (security, performance, correctness, style)
- Severity levels: error, warning, suggestion, nit

### M4.5 — PR Description Panel (planned)

A dedicated panel for viewing and editing the PR description, with keyboard-driven navigation between panels.

```mermaid
graph TD
    subgraph m45 [M4.5: PR Description Panel]
        PD1["Fetch PR title + description via API"] --> PD2["Render markdown description in scrollable panel"]
        PD2 --> PD3["Edit mode: open description in textarea / $EDITOR"]
        PD3 --> PD4["Save edits back via GitHub API"]
        PD5["Panel navigation keybindings"] --> PD6["Jump between file picker / diff / description panels"]
    end
```

**Description panel**
- New panel displaying the PR title, description body, labels, and metadata
- Markdown rendered with basic formatting (headings, lists, code blocks, links)
- Scrollable with standard vim navigation (`j`/`k`, `gg`/`G`, `Ctrl+D`/`Ctrl+U`)

**Edit description**
- Press `e` in the description panel to enter edit mode
- Opens the description in `$EDITOR` (or built-in textarea as fallback)
- On save, update the PR description via the GitHub API
- Edit the PR title via `:set-title` command or a dedicated keybinding

**Panel navigation**
- Keybindings to jump between panels: file picker, diff view, description panel
- Tab / Shift-Tab to cycle through panels, or direct jump keys (e.g. `1`/`2`/`3`)
- Active panel indicated visually with a highlighted border
- Each panel remembers its scroll position and cursor when switching away

### M5 — Graphite Stacked PRs (planned)

Graphite stacked PRs require reviewing each PR against its parent branch (not main), navigating between PRs in a stack, and understanding where a PR sits in the dependency chain.

```mermaid
graph TD
    subgraph m5 [M5: Stacked PRs]
        S1["Detect stack via gt CLI or GitHub API"] --> S2[Build stack graph: parent/child relationships]
        S2 --> S3["Stack navigator panel (1/5, 2/5, ...)"]
        S3 --> S4["Jump between PRs in stack"]
        S4 --> S5[Diff against parent branch, not main]
        S5 --> S6[Show cumulative stack diff option]
        S2 --> S7[Stack overview sidebar tab]
        S7 --> S8[Per-PR status: draft / review / approved / merged]
    end
```

**Stack detection**
- Run `gt stack` or parse PR base branches to detect the stack
- Each PR in a Graphite stack targets its parent PR's branch as the base, not `main`
- Build an ordered list: `main <- PR#1 <- PR#2 <- PR#3`

**Stack navigation**
- Show stack position in title bar: `[2/5] ROKT/srs #1234 — Add feature X`
- `]` / `[` keys to move to next/previous PR in the stack
- Loading the next PR fetches its diff and comments without quitting

**Stack-aware diffing**
- Default: diff each PR against its parent branch (incremental changes only)
- Toggle: show cumulative diff from `main` to current PR (full picture)
- Visual indicator when viewing incremental vs cumulative

**Stack overview**
- Sidebar tab showing the full stack as a vertical list
- Each PR shows: number, title, review status, CI status
- Highlight the currently viewed PR
- Jump to any PR in the stack by selecting it

**CLI changes**
```
gh-review ROKT/srs 1234              # single PR (existing)
gh-review ROKT/srs 1234 --stack      # auto-detect stack, start at this PR
gh-review ROKT/srs --stack 1234 1235 1236  # explicit stack order
```

### M6 — Polish (later)

```mermaid
graph TD
    subgraph m6 [M6: Polish]
        C7[Word-level diff highlighting]
        C9[Status line: review state + checks]
        C10[Custom color themes]
        C11["Built-in themes (dark / light / high-contrast)"]
        C12[User-defined color overrides]
    end
```

- Word-level diff within changed lines (highlight the exact characters that changed)
- Status line showing PR review state and CI check status
- Built-in themes: dark (default), light, high-contrast
- Select via config: `theme = "light"`
- Full color override via `[theme.colors]` section for diff add/remove, comments, UI chrome, search highlights
- Terminal capability detection (256-color, truecolor, basic)

### M8 — gh-dash-rs Integration (future)

```mermaid
graph TD
    subgraph m8 [M8: gh-dash-rs Integration]
        D1[Extract diff + components into library crate]
        D2[Replace gh subprocess with gh-dash-github API crate]
        D3["Add PrReviewView implementing Component trait"]
        D4[Wire R keybinding to open review view inline]
        D5[Stack-aware PR list grouping in gh-dash]
        D1 --> D3
        D2 --> D3
        D3 --> D4
        D4 --> D5
    end
```

- Extract `diff/` and `components/` into a reusable library crate
- Replace `gh` CLI subprocess calls with direct API calls via `gh-dash-github`
- Embed as a native view inside the gh-dash Rust rewrite
- Seamless transition: PR list -> review view -> back, no process suspension
- Stack-aware PR grouping in the dashboard list view

### M9 — AI Chat Panel (future)

Side-by-side chat panel for discussing code with Claude while reviewing a PR.

- Split the screen: diff on the left, chat on the right
- Ask Claude about specific lines, functions, or design decisions with full diff context
- Chat history persists for the duration of the review session
- Reference code by selecting lines in the diff — context auto-injected into the chat
- Claude responses can be converted into review comments with one key

## Feature Matrix

| Status | Feature |
|--------|---------|
| done | Unified diff |
| done | Side-by-side diff |
| done | File navigation |
| done | Inline commenting |
| done | Pending review submit |
| done | Expand context (with line numbers) |
| done | Existing comment display (box-drawing thread rendering) |
| done | Help overlay (`F1`) |
| done | Vim navigation (smooth scroll animation) |
| done | Expand/collapse comments (bulk expand/collapse all) |
| done | Review confirmation popup |
| done | Reply to comment threads |
| done | `/` forward search in diff |
| done | `?` backward search in diff |
| done | `n` / `N` jump between matches |
| done | Regex + smart-case matching |
| done | File picker filter |
| done | Syntax highlighting (tree-sitter) |
| done | Resolve / unresolve comment threads |
| done | Suggestion diffs (render as rich diffs, create, accept) |
| done | Approve / request changes with body |
| done | Unapprove with body |
| done | Discard pending comment |
| done | Edit pending comment |
| done | Multi-line comments (visual select `v`/`V`) |
| done | Command mode (`:` palette with tab-completion) |
| done | Collapsible files (`zo`/`zc`) |
| done | External editor support (`$EDITOR`) |
| done | CLI aliases (PR URL / PR number) |
| done | Debug mode (`--debug`) |
| done | Dynamic keybinding hints |
| done | Open PR in browser (`o`) |
| done | Cross-platform support |
| done | Remappable keybindings |
| done | Custom actions |
| **next** | Claude AI review |
| planned | PR description panel (view + edit) |
| planned | Panel navigation (Tab / direct jump) |
| planned | Stack detection via gt CLI |
| planned | Stack navigator panel |
| planned | Jump between stack PRs |
| planned | Diff against parent branch |
| planned | Cumulative vs incremental toggle |
| planned | Stack overview sidebar |
| later | Word-level diff |
| later | Custom themes |
| future | gh-dash-rs native view |
| future | Stack-aware PR grouping |
| future | AI chat panel (side-by-side with diff) |
