# Config Module

This module handles loading and parsing the user's `config.toml` file.

## File Location

The config file is resolved in this order:

1. `$XDG_CONFIG_HOME/gh-review/config.toml` (if `XDG_CONFIG_HOME` is set)
2. `%APPDATA%/gh-review/config.toml` (Windows)
3. `~/.config/gh-review/config.toml` (macOS / Linux)

Run `:config-path` inside gh-review to see the resolved path.

## File Format

TOML. Only include what you want to override — everything else keeps its default.

```toml
[general]
smooth_scroll = false

[keys]
approve = "Ctrl-a"
submit = "Ctrl-s"
goto_first = "gg"
scroll_down = ["j", "Down"]
```

## Key Syntax

Key strings are parsed by [crokey](https://crates.io/crates/crokey),
with one rule: **two all-lowercase chars are always a pending sequence**.

Special keys and modifiers must start with an uppercase letter:

| Input           | Meaning                          |
|-----------------|----------------------------------|
| `"q"`           | Single key: q                    |
| `"gg"`          | Pending sequence: g then g       |
| `"zo"`          | Pending sequence: z then o       |
| `"Up"`          | Up arrow key                     |
| `"Down"`        | Down arrow key                   |
| `"Enter"`       | Enter key                        |
| `"Tab"`         | Tab key                          |
| `"Esc"`         | Escape key                       |
| `"Space"`       | Space key                        |
| `"F1"`          | F1 key                           |
| `"Ctrl-d"`      | Ctrl + D                         |
| `"Alt-x"`       | Alt / Option + X                 |
| `"Shift-Tab"`   | Shift + Tab                      |
| `"Cmd-s"`       | Cmd / Super + S                  |
| `"Ctrl-Shift-d"`| Ctrl + Shift + D                 |
| `["j", "Down"]` | Multiple keys for one action     |

## Available Actions

Every action below can be remapped in the `[keys]` section.
Only include the ones you want to change.

### Navigation

| Action                | Default        | Description                       |
|-----------------------|----------------|-----------------------------------|
| `scroll_down`         | `j`, `down`    | Scroll down one line              |
| `scroll_up`           | `k`, `up`      | Scroll up one line                |
| `half_page_down`      | `ctrl-d`       | Scroll down half page             |
| `half_page_up`        | `ctrl-u`       | Scroll up half page               |
| `full_page_down`      | `ctrl-f`       | Scroll down full page             |
| `full_page_up`        | `ctrl-b`       | Scroll up full page               |
| `goto_first`          | `gg`           | Go to first line                  |
| `goto_last`           | `G`            | Go to last line                   |
| `screen_top`          | `H`            | Cursor to screen top              |
| `screen_middle`       | `M`            | Cursor to screen middle           |
| `screen_bottom`       | `L`            | Cursor to screen bottom           |
| `center_cursor`       | `zz`           | Center cursor in viewport         |
| `scroll_cursor_top`   | `zt`           | Scroll cursor to top              |
| `scroll_cursor_bottom`| `zb`           | Scroll cursor to bottom           |
| `next_hunk`           | `]`, `}`       | Jump to next hunk                 |
| `prev_hunk`           | `[`, `{`       | Jump to previous hunk             |
| `next_change`         | `)`            | Jump to next change               |
| `prev_change`         | `(`            | Jump to previous change           |
| `next_match_or_file`  | `n`            | Next search match or file         |
| `prev_match_or_file`  | `N`            | Previous search match or file     |

### Search

| Action                | Default        | Description                       |
|-----------------------|----------------|-----------------------------------|
| `search_forward`      | `/`            | Search forward in diff            |
| `search_backward`     | `?`            | Search backward in diff           |

### UI

| Action                | Default        | Description                       |
|-----------------------|----------------|-----------------------------------|
| `help`                | `!`, `f1`      | Toggle help overlay               |
| `switch_focus`        | `tab`          | Switch focus: files / diff        |
| `toggle_view`         | `t`            | Toggle unified / side-by-side     |
| `toggle_comment`      | `enter`        | Toggle comment expand             |
| `fold_toggle`         | `enter`        | Toggle file fold                  |
| `fold_open`           | `zo`           | Expand file fold                  |
| `fold_close`          | `zc`           | Collapse file fold                |
| `open_command_mode`   | `:`            | Open command prompt                |
| `open_browser`        | `o`            | Open PR in browser                |
| `escape`              | `esc`          | Clear search / cancel / quit      |
| `quit`                | `q`            | Quit                              |

### Review

| Action                | Default        | Description                       |
|-----------------------|----------------|-----------------------------------|
| `comment_on_line`     | `c`            | Comment on current line           |
| `comment`             | —              | Review comment with body          |
| `suggest`             | `e`            | Suggest change on current line    |
| `expand`              | `E`            | Expand context (+10 lines)        |
| `visual`              | `v`, `V`       | Visual select mode                |
| `approve`             | `a`            | Approve (quick confirm)           |
| `approve_with_comment`| —              | Approve with review body          |
| `submit`              | `s`            | Submit comment-only review        |
| `unapprove`           | `u`            | Dismiss own approval              |
| `request_changes`     | —              | Request changes (quick)           |
| `request_changes_with_comment` | —     | Request changes with body         |
| `resolve`             | `r`            | Resolve / unresolve thread        |
| `discard`             | `x`            | Discard pending comment           |
| `accept_suggestion`   | `y`            | Accept suggestion                 |
| `expand_all_comments` | —              | Expand all comment threads        |
| `collapse_all_comments`| —             | Collapse all comment threads      |

### File Picker

| Action                | Default        | Description                       |
|-----------------------|----------------|-----------------------------------|
| `picker_down`         | `j`, `down`    | File picker: next                 |
| `picker_up`           | `k`, `up`      | File picker: previous             |
| `file_filter`         | `/`            | Filter file list                  |

### Other

| Action                | Default        | Description                       |
|-----------------------|----------------|-----------------------------------|
| `config_path`         | —              | Show config file path             |

Actions marked **—** have no default hotkey. They are accessible
via `:command` but you can assign a hotkey in your config:

```toml
[keys]
request_changes = "Ctrl-r"
expand_all_comments = "Alt-e"
```

## Module Structure

- **`mod.rs`** — `UserConfig` struct, TOML deserialization, `load_user_config()`
- **`keys.rs`** — `KeyBinding` enum, `parse_key_string()` (crokey), `format_key_binding()` / `format_key_combo()` (display)
- **`runtime.rs`** — `Config` struct (resolved runtime config), debug logging
