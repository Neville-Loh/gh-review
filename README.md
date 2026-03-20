# gh-review

Terminal UI for reviewing GitHub pull requests. View diffs (unified and side-by-side), comment on specific lines, expand context, and approve — all without leaving the terminal.

![gh-review screenshot](docs/screenshot.png)

## Install

```bash
cargo install --path .
```

Requires the [GitHub CLI](https://cli.github.com/) (`gh`) to be installed and authenticated.

## Usage

```bash
gh-review <OWNER/REPO> <PR_NUMBER>
```

```bash
gh-review octocat/hello-world 42
```

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
| `n` / `N` | Next / previous file |
| `zz` / `zt` / `zb` | Center / top / bottom cursor in viewport |
| `Tab` | Switch focus between file list and diff |

### Diff

| Key | Action |
|-----|--------|
| `t` | Toggle unified / side-by-side view |
| `e` | Expand context around cursor (+10 lines) |

### Review

| Key | Action |
|-----|--------|
| `c` | Comment on current line |
| `Ctrl+S` | Save comment (in comment editor) |
| `Esc` | Cancel comment |
| `a` | Submit review — approve |
| `r` | Submit review — request changes |
| `s` | Submit review — comment only |

Comments are batched into a pending review and submitted together when you press `a`, `r`, or `s`. This matches GitHub's review model.

### Other

| Key | Action |
|-----|--------|
| `o` | Open PR in browser |
| `?` | Show help overlay |
| `q` | Quit |

## Architecture

```
src/
  main.rs              CLI entry point (clap)
  app.rs               Root state, event loop, key dispatch
  event.rs             Async event channel (crossterm + tokio)
  gh.rs                GitHub API via gh CLI subprocess
  types.rs             Domain types
  theme.rs             Colors and styles
  diff/
    parser.rs          Parse unified diff into structured hunks
    renderer.rs        Render unified + side-by-side views
    expand.rs          Fetch and splice expanded context
  components/
    diff_view.rs       Scrollable diff viewport
    file_picker.rs     File list sidebar
    comment_input.rs   Inline comment textarea popup
    review_bar.rs      Bottom status bar
    help.rs            Keybinding help overlay
```

All GitHub API calls go through the `gh` CLI, reusing your existing authentication. No tokens or OAuth configuration needed.
