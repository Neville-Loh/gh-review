# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

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
