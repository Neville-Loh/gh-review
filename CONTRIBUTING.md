# Contributing to gh-review

Thanks for your interest in contributing! This guide covers everything you need to get started.

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable, 2024 edition)
- [GitHub CLI](https://cli.github.com/) (`gh`) — installed and authenticated
- A GitHub account with access to at least one repo with open PRs (for manual testing)

### Build

```bash
git clone https://github.com/Neville-Loh/gh-review.git
cd gh-review
cargo build
```

### Run

```bash
cargo run -- <OWNER/REPO> <PR_NUMBER>
```

### Test

```bash
cargo test
```

## Making Changes

1. **Fork** the repository and create a branch from `main`.
2. **Keep commits focused** — one logical change per commit.
3. **Follow existing code style** — the project uses standard `rustfmt` formatting and `clippy` lints.
4. **Test your changes** — add tests where applicable, and verify manually against a real PR.
5. **Update documentation** — if you add or change keybindings, update `README.md` and `docs/ROADMAP.md`.

### Code Style

```bash
cargo fmt --check
cargo clippy -- -D warnings
```

Both checks run in CI and must pass before merge.

### Project Structure

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

## Pull Requests

- Open a PR against `main`.
- Describe **what** the change does and **why**.
- Link to a relevant issue if one exists.
- Keep PRs small and reviewable — under 400 lines changed is ideal.
- CI must be green before review.

## Reporting Bugs

Open an [issue](https://github.com/Neville-Loh/gh-review/issues) with:

- Steps to reproduce
- Expected vs actual behavior
- Terminal emulator and OS
- Output of `gh-review --version` and `gh --version`

## Feature Requests

Open an [issue](https://github.com/Neville-Loh/gh-review/issues) tagged with `enhancement`. Check the [roadmap](docs/ROADMAP.md) first to see if it's already planned.

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
