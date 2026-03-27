# gh-review

Terminal UI for reviewing GitHub pull requests. View diffs (unified and side-by-side), comment on lines, suggest changes, resolve threads, expand context, and approve — all without leaving the terminal.

![gh-review screenshot](docs/screenshot.png)

## Install

```bash
cargo install gh-review
```

Requires the [GitHub CLI](https://cli.github.com/) (`gh`) to be installed and authenticated.

## Usage

```bash
gh-review <OWNER/REPO> <PR_NUMBER>
gh-review <URL>
gh-review <PR_NUMBER>
```

```bash
gh-review octocat/hello-world 42
```

Accepts GitHub and Graphite PR URLs. When given just a PR number, the repository is inferred from `git remote get-url origin`.

### With gh-dash

[gh-dash](https://github.com/dlvhdr/gh-dash) is a terminal dashboard for GitHub PRs, issues, and notifications. gh-review is designed to complement it — use gh-dash to browse and triage, press a key to jump into gh-review for deep code review.

#### Setup

1. Install gh-dash if you haven't already:

   ```bash
   gh extension install dlvhdr/gh-dash
   ```

2. Add a custom keybinding to `~/.config/gh-dash/config.yml`:

   ```yaml
   keybindings:
     prs:
       - key: R
         name: review (gh-review)
         command: >
           gh-review {{.RepoName}} {{.PrNumber}}
   ```

   `{{.RepoName}}` and `{{.PrNumber}}` are template variables that gh-dash fills in with the currently selected PR.

3. Run gh-dash:

   ```bash
   gh dash
   ```

4. Navigate to a PR and press `R`. gh-dash suspends and gh-review takes over the terminal. When you quit gh-review (`q`), you're back in gh-dash.

#### Workflow

```
gh-dash (browse PRs)
  │
  ├─ R  → gh-review (diff, comment, approve)
  ├─ D  → delta side-by-side diff (quick read-only view)
  ├─ d  → unified diff in pager
  └─ V  → approve directly
```

## Keybindings

### Navigation

| Key | Action |
|-----|--------|
| `j` / `k` / `↑` / `↓` | Scroll line |
| `gg` / `G` | Go to first / last line |
| `Ctrl+D` / `Ctrl+U` | Half page down / up |
| `Ctrl+F` / `Ctrl+B` | Full page down / up |
| `H` / `M` / `L` | Cursor to screen top / middle / bottom |
| `]` / `}` | Next hunk |
| `[` / `{` | Previous hunk |
| `)` / `(` | Next / previous change |
| `n` / `N` | Next / previous file (or search match when search active) |
| `zo` / `zc` | Expand / collapse file at cursor |
| `Enter` | Toggle comment expand or file fold |
| `zz` / `zt` / `zb` | Center / top / bottom cursor in viewport |
| `Tab` | Switch focus between file list and diff |

### Search

| Key | Action |
|-----|--------|
| `/` | Search forward (regex, smart-case) |
| `?` | Search backward (in diff view) |
| `n` / `N` | Next / previous match (respects search direction) |
| `Esc` | Cancel search and restore cursor |
| `Enter` | Confirm search |

In the file picker, `/` opens a fuzzy file filter instead.

### Diff

| Key | Action |
|-----|--------|
| `t` | Toggle unified / side-by-side view |
| `zo` / `zc` | Expand / collapse file fold |
| `e` | Suggest change on current line |
| `E` | Expand context around cursor (+10 lines) |

### Review

| Key | Action |
|-----|--------|
| `c` | Comment on current line (or edit pending / reply) |
| `v` | Visual select mode for multi-line comments |
| `x` | Discard pending comment at cursor |
| `Esc` | Cancel comment / cancel visual selection |
| `a` | Approve (quick confirm) |
| `r` | Resolve / unresolve comment thread |
| `s` | Submit review as comment-only (quick confirm) |
| `u` | Unapprove — dismiss your own approval |
| `y` | Accept suggestion (apply as commit) |

Comments are batched into a pending review and submitted together when you press `a` or `s`. These open a quick confirm popup (Enter / Esc). For request changes or review submissions with a body message, use the `:` command mode (see below).

### Command Mode

Press `:` to open the command prompt. Type a command name and press Enter to execute. Tab cycles through completions.

| Command | Action |
|---------|--------|
| `:approve` | Approve (quick confirm) |
| `:approve_with_comment` | Approve with review body |
| `:request_changes` | Request changes (quick confirm) |
| `:request_changes_with_comment` | Request changes with body |
| `:submit` | Submit comment-only (quick confirm) |
| `:comment` | Review comment with body |
| `:unapprove` | Dismiss own approval |
| `:suggest` | Suggest change on current line |
| `:expand` | Expand context |
| `:discard` | Discard pending comment |
| `:resolve` | Resolve / unresolve thread |
| `:accept_suggestion` | Accept suggestion |
| `:toggle_view` | Toggle unified / side-by-side |
| `:help` | Toggle help overlay |
| `:quit` (or `:q`) | Quit |
| `:open_browser` | Open PR in browser |

### Other

| Key | Action |
|-----|--------|
| `:` | Open command prompt |
| `o` | Open PR in browser |
| `!` | Show help overlay |
| `q` | Quit |

## Architecture

See [docs/architecture.md](docs/architecture.md) for the source tree layout and design overview.

## Roadmap

See [docs/ROADMAP.md](docs/ROADMAP.md) for planned features including stacked PR support, syntax highlighting, configuration, and more.

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on how to get started, submit pull requests, and report issues.

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md).

## License

[MIT](LICENSE)
