# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.2] - 2026-03-24

### Added

- **Syntax highlighting** — full tree-sitter-based highlighting for diff content via arborium with GitHub Dark theme; highlights both unified and side-by-side views, and re-highlights after context expansion (#9)
- **Command mode** — vim-style `:` command palette with tab-completion, sorted fuzzy matching, inline documentation, and a popup completion menu; all review actions are now executable as named commands (`:approve`, `:comment`, `:suggest`, `:quit`, etc.) (#16, #17)
- **Code suggestions** — propose inline code changes on any line with `e`; opens a pre-filled editor with the original line content for modification; suggestions are rendered as diff blocks in the review (#19)
- **Visual line selection** — `v` to enter visual mode, extend selection across lines, then `c` to comment or `e` to suggest across the entire range; multi-line comments include `start_line`/`start_side` fields for proper GitHub rendering (#15, #21)
- **Review comment management** — discard pending comments with `x`/`d`, edit pending comments by pressing `c` on them (re-opens editor pre-filled with existing body) (#15)
- **Resolve/unresolve threads** — toggle thread resolution status from the TUI via GraphQL API (#15)
- **Accept suggestions** — apply code suggestions directly to the PR branch from the terminal using the GitHub Contents API (#15)
- **Dismiss own approval** — `u` to unapprove, dismisses your review via the API (#15)
- **Reply to review comments** — post threaded replies to existing review comments inline (#15)
- **Review submission with body** — `:comment`, `:approve_with_comment`, and `:request_changes_with_comment` open a textarea for a review body; quick actions (`a`, `r`, `s`) now show a compact Enter/Esc confirmation instead (#20)
- **`V` for visual select** — capital V also enters visual line selection mode (#21)
- **Expand/collapse all comments** — `:expand_all_comments` and `:collapse_all_comments` commands to bulk toggle every thread at once

### Fixed

- Expanded context lines now receive syntax highlighting instead of appearing as plain text (#11)
- Search status indicator (`[3/15]`) updates correctly after navigating between matches (#13)
- Key modifiers (Ctrl, Shift) register correctly again after the keymap refactor (#18)
- GitHub API 422 errors now display the actual error message (e.g. "Pull request review thread is already resolved") instead of raw JSON or generic "gh api POST reviews failed" (#12)
- Side-by-side diff styling and missing `highlighted_content` field in tests (#12)

### Changed

- **Architecture overhaul** — split monolithic `app.rs` (712 lines) into `app/mod.rs`, `app/handlers.rs`, `app/keymap.rs`, `app/command.rs`, and `app/ui.rs`; split `diff_view.rs` into `diff_view/mod.rs`, `diff_view/draw.rs`, and `diff_view/navigation.rs`; extracted `diff/model.rs` from renderer; split `search.rs` into `search/mod.rs` and `search/tests.rs` (+2504/−2358 lines) (#10)
- **Command-driven keymap** — all keybindings now dispatch through a `Command` registry with named, documented, and discoverable actions instead of inline closures; enables future remappable keybindings (#16)
- `c` on a diff line now opens an inline comment; `:comment` opens a review-level comment with body (#20)
- **Comment thread rendering** — complete visual redesign of comment threads with box-drawing characters (┌─┐│└─┘), distinct color-coded backgrounds (blue for comments, green for resolved, amber for pending), word-wrapping within thread boxes, reply count badges, and responsive width that adapts to the terminal

## [0.1.1] - 2026-03-24

### Added

- Vim-style search in diff view (`/` forward, `?` backward)
- Regex and literal pattern matching with smart-case
- Incremental search with real-time match highlighting
- `n`/`N` to jump between search matches (wraps around)
- Character-level match highlighting in both unified and side-by-side modes
- File picker fuzzy filter (`/` when file picker focused)
- Subsequence fuzzy matching for file paths
- Search bar with match count indicator (`[3/15]`)
- `Esc` clears active search and restores cursor position
- `F1` keybinding for help overlay
- Reply to review comments
- Toggle comment visibility with `e`
- Cross-platform support (Windows PowerShell fix)

### Changed

- `n`/`N` now navigates search matches when search is active (file navigation when inactive)
- `?` opens backward search in diff view (help moved to `F1`)

## [0.1.0] - 2026-03-24

### Added

- Unified and side-by-side diff views
- Inline commenting on specific lines
- Pending review submission (approve, request changes, comment)
- File picker sidebar with navigation
- Context expansion around hunks
- Vim-style keybindings (gg, G, Ctrl+D/U, zz, etc.)
- gh-dash integration via custom keybinding
- Help overlay (`?`)
- Open PR in browser (`o`)
