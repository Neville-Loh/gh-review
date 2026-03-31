# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- **PR status in title bar** — colored status icon (approved, changes requested, draft, merged, closed) derived from GitHub's `reviewDecision` field (#63)

### Changed

- **Description panel default off** — `d` toggles the description panel on/off; default is now hidden; navigate with arrow keys when open
- **`q` scoped to panel** — pressing `q` in the description panel closes the panel instead of quitting the app
- **Context-aware review bar hints** — status bar hints are now generated from the keymap with conditions (e.g. "discard" only shown on pending comments, "resolve"/"unresolve" adapts to thread state); replaces hardcoded hint strings (#64)
- **Help overlay reformatted** — hardcoded layout with stack navigation section when a stack is detected; cleaner presentation
- **`RowContext` carries state** — `Comment` and `Suggestion` variants now include `CommentState` (is_pending, is_resolved) for context-sensitive key dispatch and hint rendering
- **`PrStatus` consolidated** — merged review decision into `PrStatus` with `Approved` and `ChangesRequested` variants; added `icon()` and `color()` methods (#63)

## [0.2.1] - 2026-03-30

### Added

- **Graphite stack navigation** — auto-detects stacked PRs from Graphite bot comments; navigate between PRs in a stack with Cmd+K / Cmd+J (or Cmd+Up / Cmd+Down); PR data is cached so switching is instant (#57)
- **Stack indicator** — description panel shows the full stack with per-PR status (open, draft, merged, closed), titles, and highlights the currently viewed PR (#57)
- **Title bar additions/deletions** — PR title bar now shows colored `+N/-M` line counts (#59)
- **Paragraph jumps** — `{` / `}` to jump between blank lines in the diff, matching vim paragraph motion; `[` / `]` remain for hunk navigation (#61)
- **`:` command mode everywhere** — command bar now opens from any panel, not just the diff view (#57)
- **Branch info in description** — description panel header shows `base → head` branch names (#57)

### Changed

- **Comment rendering refactor** — extracted `emit_expanded_thread` helper to share rendering logic across inline threads, pending comments, and orphan threads; reduces duplication in `diff/model.rs` (#52, #58)
- **Fetch pipeline refactor** — replaced four parallel `tokio::spawn` blocks with a generic `spawn_fetch` helper; events now carry a `pr` field so stale responses from a previous PR are discarded (#57)
- **Graphite bot comments filtered** — Graphite's auto-generated stack management comments are now hidden from the diff view (#57)
- **Test data anonymized** — CLI test fixtures now use `acme/widgets` instead of real repo names (#58)

### Fixed

- **Paragraph jump in diff** — `{` / `}` now correctly land on file/hunk header boundaries instead of overshooting (#61)

## [0.2.0] - 2026-03-28

### Added

- **User configuration** — TOML config file at `~/.config/gh-review/config.toml` with all settings optional and sensible defaults; run `:config-path` to see the resolved path (#41)
- **Remappable keybindings** — every action can be rebound via `[keys]` section; supports single keys, modifier combos (`Ctrl-d`), pending sequences (`gg`, `zo`), and multiple bindings per action (`["j", "Down"]`); unbind with `"no_op"` (#41)
- **Custom actions** — user-defined shell commands via `[[actions]]` with template variables (`{PR_NUMBER}`, `{REPO}`, `{URL}`, etc.); bindable to hotkeys and invocable from the command bar (#42)
- **Command aliases** — define short names for built-in commands in `[aliases]` (e.g. `lgtm = "approve"`); usable in `[keys]` and the `:` command bar (#44)
- **Disable commands** — remove unwanted commands from both the keymap and command bar via `disabled_commands` (#44)
- **PR description panel** — dedicated panel showing the PR title and description with markdown rendering; toggle with `:description` (#53)
- **Panel navigation** — Tab / Shift-Tab to cycle between file picker, diff view, and description panel; active panel indicated with highlighted border (#46)
- **Status line** — transient message bar with info/success/error states and auto-expiry after 2 seconds (#50)
- **`:q!` command** — force-quit alias (#49)

### Changed

- **Comment block rendering** — migrated comment threads to a dedicated `CommentBlock` widget with rounded borders, color-coded backgrounds, word-wrapping, and proper width constraints; replaces the previous inline row-based rendering (#52)
- **Unresolved comments default open** — unresolved comment threads now expand automatically on load instead of starting collapsed (#45)
- **Architecture** — extracted `command_handlers.rs` (499 lines) from `command.rs` and `comment_block.rs` (297 lines) from `diff_view/draw.rs` for better separation of concerns (#52)

### Fixed

- **Visual select in side-by-side view** — visual line selection now works correctly in side-by-side diff mode (#47)
- **Top-level comments visible** — PR-level review comments (not attached to a specific line) are now displayed in the diff view (#54)

## [0.1.3] - 2026-03-27

### Added

- **Edit code with your own editor** — users can now modify code in suggestions using their preferred `$EDITOR`; falls back to a built-in text field on systems without one (e.g. Windows) (#37)
- **Collapsible files** — open and close files in the diff view with `zo`/`zc` or by pressing Enter on a file header to hide reviewed diffs and reduce clutter (#33)
- **Row context expansion** — expand context around diff hunks showing line numbers for better orientation (#38)
- **CLI aliases** — invoke with a PR URL or just a PR number when inside the repo (e.g. `gh-review 42` or `gh-review https://github.com/org/repo/pull/42`) (#34)
- **Debug mode** — `--debug` flag to dump resolved config and diagnostics for troubleshooting (#32)

### Changed

- **Suggestions render as rich diffs** — code suggestions now display with proper diff formatting instead of plain markdown code fences (#36)
- **Dynamic keybinding hints** — status bar hints now update based on the current context and active mode instead of being static
- **Enter to save comments** — pressing Enter now saves the current comment instead of inserting a newline; quality-of-life improvement for faster reviewing (#35)
- **Scroll animation** — smoother and more responsive scrolling in the diff view (#31)
- **Resolve/unresolve hotkey** — replaced the `r` (request changes) hotkey with resolve/unresolve thread toggle for faster review workflows (#30)
- Moved architecture documentation into a separate `docs/architecture.md` file (#27)

### Fixed

- **Side-by-side view rendering** — fixed empty area appearing when scrolling through paired add/remove lines, and comments now render on the correct panel (left for deletions, right for additions) (#29)
- **Backward search** — `?` search now correctly navigates in reverse direction (#28)

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
