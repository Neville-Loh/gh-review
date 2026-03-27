# Architecture

```
src/
  main.rs                  CLI entry point (clap), terminal setup
  event.rs                 Async event channel (crossterm + tokio)
  gh.rs                    GitHub API via gh CLI subprocess (REST + GraphQL)
  types.rs                 Domain types (DiffFile, Hunk, ReviewComment, etc.)
  theme.rs                 Colors and styles
  highlight.rs             Syntax highlighting (arborium)
  app/
    mod.rs                 App struct, state, event dispatch, data loading
    command.rs             Command struct, registry macro, handler functions
    keymap.rs              Configurable key-to-command mapping (HashMap)
    handlers.rs            Modal key handlers, async commands
    ui.rs                  Layout and drawing
  diff/
    model.rs               DisplayRow types and display row builder
    renderer.rs            Unified + side-by-side row rendering
    parser.rs              Parse unified diff patches into structured hunks
    expand.rs              Fetch and splice expanded context lines
  search/
    mod.rs                 Regex search engine with match navigation
    tests.rs               Search unit tests
  components/
    diff_view/
      mod.rs               Scrollable diff viewport state and query helpers
      navigation.rs        Vim-style cursor movement (scroll, jump, page)
      draw.rs              Unified and side-by-side rendering
    file_picker.rs         File list sidebar with fuzzy filter
    comment_input.rs       Inline comment textarea popup
    command_bar.rs         Command mode prompt with tab completion
    search_bar.rs          Search prompt with match count display
    review_bar.rs          Bottom status bar
    review_confirm.rs      Review submission confirmation popup
    help.rs                Keybinding help overlay
```

Key handling follows the **Helix command pattern**: each action is a `Command` struct with name, doc, and handler fn pointer, registered via a `define_commands!` macro. A `Keymap` maps key combos to commands via `HashMap`. The `:` command mode looks up commands by name from the same registry. Modal states (search bar, comment input, review confirm, command bar, file filter, help) each have dedicated handlers.

All GitHub API calls go through the `gh` CLI, reusing your existing authentication. No tokens or OAuth configuration needed.
