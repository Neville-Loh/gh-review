# gh-review Roadmap

## Overview

```mermaid
graph LR
    M1["M1: Read-only Diff"] --> M2["M2: Review Actions"]
    M2 --> M25["M2.5: Comment Management"]
    M25 --> M26["M2.6: Search"]
    M26 --> M3["M3: Stacked PR Support"]
    M3 --> M4["M4: Polish"]
    M4 --> M45["M4.5: User Configuration"]
    M45 --> M5["M5: gh-dash-rs Integration"]
```

## Milestones

### M1 — Read-only Diff Viewer (done)

```mermaid
graph TD
    subgraph m1 [M1: Read-only Diff]
        A1[Unified diff parser] --> A2[Unified renderer]
        A1 --> A3[Side-by-side renderer]
        A4[gh CLI subprocess wrapper] --> A5[Fetch PR files + metadata]
        A2 --> A6[Scrollable diff viewport]
        A3 --> A6
        A5 --> A6
        A7[File picker sidebar] --> A6
        A8[File navigation n/N] --> A6
    end
```

- Parse GitHub patch format into structured hunks
- Unified and side-by-side rendering with syntax-colored +/- lines
- Dual-number gutters (old line / new line)
- File list sidebar with status indicators and +/- counts
- Keyboard navigation: scroll, page, jump to file, toggle view mode

### M2 — Review Actions (done)

```mermaid
graph TD
    subgraph m2 [M2: Review Actions]
        B1[Comment input popup] --> B2[Pending review accumulator]
        B2 --> B3["Submit review (approve / request changes / comment)"]
        B4[Fetch existing review threads] --> B5[Display inline in diff]
        B6[Expand context] --> B7[Fetch full file at base+head refs]
        B7 --> B8[Splice into hunk]
    end
```

- Inline comment textarea anchored to cursor line
- Pending review model — batch comments, submit as one review
- Approve, request changes, and comment-only submission with confirmation popup
- Existing review comments displayed inline in the diff
- Expandable context — fetch full file content and splice +10 lines
- Expand/collapse multi-line comments with Enter
- Vim-style navigation (gg, G, H/M/L, ]/[, zz/zt/zb, Ctrl+F/B)
- Clean process shutdown (works as gh-dash subprocess)

### M2.5 — Comment Management (next)

```mermaid
graph TD
    subgraph m25 [M2.5: Comment Management]
        CM1[Discard pending comment] --> CM2["Remove from pending list, rebuild display"]
        CM3[Edit pending comment] --> CM4["Re-open textarea with existing body"]
        CM5[Expand/collapse comments] --> CM6["Enter toggles, ▶/▼ indicator (done)"]
    end
```

- **Discard pending comment** — cursor on a pending comment, press `x` or `d` to remove it from the pending review
- **Edit pending comment** — cursor on a pending comment, press `c` or `e` to re-open the textarea pre-filled with the existing body
- Expand/collapse multi-line comments — done (Enter to toggle)

### M2.6 — Search

Vim-style search across diff content and file names, matching the `/` and `?` patterns familiar to vim and less users.

```mermaid
graph TD
    subgraph m26 [M2.6: Search]
        SR1["/ forward search prompt"] --> SR2[Regex + literal matching engine]
        SR3["? backward search prompt"] --> SR2
        SR2 --> SR4["Highlight all matches in diff viewport"]
        SR4 --> SR5["n / N jump to next / previous match"]
        SR6["File picker search: type to filter"] --> SR7["Fuzzy-match file names"]
    end
```

**Diff search (`/` and `?`)**
- `/` opens a search prompt at the bottom of the screen (forward search)
- `?` opens search in reverse direction (when help overlay is not active)
- Supports literal and regex patterns
- All matches highlighted in the diff viewport with a distinct style
- `n` jumps to next match, `N` jumps to previous match
- Search wraps around at end/beginning of diff
- `Esc` or `Enter` on empty input exits search mode
- Matches persist until a new search or explicit clear

**File picker search**
- When file picker is focused, typing `/` activates a filter prompt
- Fuzzy matching against file paths (e.g. `comp/diff` matches `src/components/diff_view.rs`)
- Filtered list updates as you type, press `Enter` to select, `Esc` to cancel

**Keybinding considerations**
- `n`/`N` currently navigate files — when a search is active, they switch to search navigation; when no search is active, they retain file navigation behavior
- `?` currently shows help — resolve by using `?` for search only in diff view and keeping `?` for help in other contexts, or by moving help to `F1`

### M3 — Stacked PR Support

Graphite stacked PRs require reviewing each PR against its parent branch (not main), navigating between PRs in a stack, and understanding where a PR sits in the dependency chain.

```mermaid
graph TD
    subgraph m3 [M3: Stacked PRs]
        S1["Detect stack via gt CLI or GitHub API"] --> S2[Build stack graph: parent/child relationships]
        S2 --> S3["Stack navigator panel (1/5, 2/5, ...)"]
        S3 --> S4["Jump between PRs in stack (] / [ keys)"]
        S4 --> S5[Diff against parent branch, not main]
        S5 --> S6[Show cumulative stack diff option]
        S2 --> S7[Stack overview sidebar tab]
        S7 --> S8[Per-PR status: draft / review / approved / merged]
    end
```

**Stack detection**
- Run `gt stack` or parse PR base branches to detect the stack
- Each PR in a Graphite stack targets its parent PR's branch as the base, not `main`
- Build an ordered list: `main ← PR#1 ← PR#2 ← PR#3`

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

### M4 — Polish

```mermaid
graph TD
    subgraph m4 [M4: Polish]
        C1[Syntax highlighting] --> C2["syntect integration"]
        C3[Multi-line comment selection] --> C4["start_line + line range"]
        C5[Reply to existing threads]
        C6[Resolve/unresolve threads]
        C7[Word-level diff highlighting]
        C9[Status line: review state + checks]
    end
```

- Syntax highlighting for diff content (Rust, Go, Python, TypeScript, etc.)
- Word-level diff within changed lines (highlight the exact characters that changed)
- Multi-line comment selection (visual select a range, then comment)
- Reply to and resolve existing review threads
- Status line showing PR review state and CI check status

### M4.5 — User Configuration

User-facing config file (`~/.config/gh-review/config.toml`) for personalizing the tool without recompiling.

```mermaid
graph TD
    subgraph m45 [M4.5: User Configuration]
        UC1["Config file loader (~/.config/gh-review/config.toml)"] --> UC2[Remappable keybindings]
        UC1 --> UC3[Custom color themes]
        UC1 --> UC4[Custom scripts via hooks]
        UC3 --> UC5["Built-in themes (dark / light / high-contrast)"]
        UC3 --> UC6["User-defined color overrides"]
        UC4 --> UC7["on_submit / on_approve / on_open hooks"]
    end
```

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

**Custom themes**
- Built-in themes: dark (default), light, high-contrast
- Select via config: `theme = "light"`
- Full color override via `[theme.colors]` section for diff add/remove, comments, UI chrome, search highlights
- Terminal capability detection (256-color, truecolor, basic)

```toml
theme = "dark"

[theme.colors]
add_bg = "#1a3a1a"
remove_bg = "#3a1a1a"
comment_fg = "#f0c674"
search_match = "#ffcc00"
```

**Custom scripts**
- Hook system: run user-defined shell commands on review lifecycle events
- Supported hooks: `on_open`, `on_submit`, `on_approve`, `on_request_changes`, `on_quit`
- Scripts receive context as environment variables (`GH_REVIEW_REPO`, `GH_REVIEW_PR`, `GH_REVIEW_ACTION`)
- Async execution — scripts run in background, don't block the UI

```toml
[hooks]
on_approve = "notify-send 'PR approved' '$GH_REVIEW_REPO#$GH_REVIEW_PR'"
on_submit = "~/.config/gh-review/scripts/post-review.sh"
```

### M5 — gh-dash-rs Integration (future)

```mermaid
graph TD
    subgraph m5 [M5: gh-dash-rs Integration]
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
- Seamless transition: PR list → review view → back, no process suspension
- Stack-aware PR grouping in the dashboard list view

## Feature Matrix

```mermaid
graph LR
    subgraph done [Done]
        F1[Unified diff]
        F2[Side-by-side diff]
        F3[File navigation]
        F4[Inline commenting]
        F5[Pending review submit]
        F6[Expand context]
        F7[Existing comment display]
        F8[Help overlay]
        Fv[Vim navigation]
        Fec[Expand/collapse comments]
        Frc[Review confirmation popup]
    end

    subgraph next25 [Next: Comment Management]
        Fd[Discard pending comment]
        Fe[Edit pending comment]
    end

    subgraph next26 [Next: Search]
        Fs1["/ forward search in diff"]
        Fs2["? backward search in diff"]
        Fs3["n / N jump between matches"]
        Fs4[Regex + literal matching]
        Fs5[File picker fuzzy filter]
    end

    subgraph next [Next: Stacked PRs]
        F9[Stack detection via gt CLI]
        F10[Stack navigator panel]
        F11["Jump between stack PRs (] / [)"]
        F12[Diff against parent branch]
        F13[Cumulative vs incremental toggle]
        F14[Stack overview sidebar]
    end

    subgraph later [Later: Polish]
        F15[Syntax highlighting]
        F16[Word-level diff]
        F17[Multi-line comments]
        F18[Reply to threads]
    end

    subgraph later45 [Later: User Configuration]
        F19[Remappable keybindings]
        F19b[Custom themes]
        F19c[Custom script hooks]
        F19d[TOML config file]
    end

    subgraph future [Future]
        F20[gh-dash-rs native view]
        F21[Suggest changes]
        F22[Stack-aware PR grouping]
    end
```
