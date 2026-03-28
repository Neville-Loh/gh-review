//! Keymap: maps key presses to commands per panel (Diff, Picker, Description).
//!
//! Each key binding specifies which panels it applies to and which command to
//! run in each. `Global` is sugar for "same command in all panels". The binding
//! table in [`Keymap::default_binding_defs`] is the single source of truth for
//! default hotkeys.

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::command::{self, Command};
use super::custom_action::{CustomAction, ResolvedActions};
use super::Focus;
use crate::config::{KeyBinding, UserConfig, format_key_binding, parse_key_string};
use crate::types::RowContext;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct KeyCombo {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl From<KeyCode> for KeyCombo {
    fn from(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }
}

impl From<char> for KeyCombo {
    fn from(c: char) -> Self {
        Self {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::NONE,
        }
    }
}

impl KeyCombo {
    pub fn ctrl(c: char) -> Self {
        Self {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::CONTROL,
        }
    }
}

impl From<&KeyEvent> for KeyCombo {
    fn from(event: &KeyEvent) -> Self {
        let mut modifiers = event.modifiers;
        if let KeyCode::Char(_) = event.code {
            modifiers.remove(KeyModifiers::SHIFT);
        }
        Self {
            code: event.code,
            modifiers,
        }
    }
}

// ── Internal structures ───────────────────────────────────────────────

struct Binding {
    command: &'static Command,
    context: Option<RowContext>,
}

/// Which UI panel a binding targets.
#[derive(Clone, Copy, PartialEq)]
enum Panel {
    Diff,
    Picker,
    Description,
}

const ALL_PANELS: &[Panel] = &[Panel::Diff, Panel::Picker, Panel::Description];

/// Pairs a [`Panel`] with the [`Command`] to run when the key fires there.
#[derive(Clone, Copy)]
struct ScopeBinding {
    panel: Panel,
    command: &'static Command,
}

impl ScopeBinding {
    fn on(panel: Panel, command: &'static Command) -> Self {
        Self { panel, command }
    }
}

/// A key binding definition: one or more keys, one or more panel->command
/// mappings, and an optional diff-context filter.
struct BindingDef {
    keys: Vec<KeyBinding>,
    scopes: Vec<ScopeBinding>,
    /// Only used for diff panel; filters by cursor row type.
    context: Option<RowContext>,
}

// ── Binding definition constructors ─────────────────────────────────

impl BindingDef {
    fn global(cmd: &'static Command, keys: Vec<KeyBinding>) -> Self {
        Self {
            keys,
            scopes: ALL_PANELS
                .iter()
                .map(|&panel| ScopeBinding::on(panel, cmd))
                .collect(),
            context: None,
        }
    }

    fn diff(cmd: &'static Command, keys: Vec<KeyBinding>) -> Self {
        Self {
            keys,
            scopes: vec![ScopeBinding::on(Panel::Diff, cmd)],
            context: None,
        }
    }

    fn diff_ctx(cmd: &'static Command, keys: Vec<KeyBinding>, ctx: RowContext) -> Self {
        Self {
            keys,
            scopes: vec![ScopeBinding::on(Panel::Diff, cmd)],
            context: Some(ctx),
        }
    }

    fn picker(cmd: &'static Command, keys: Vec<KeyBinding>) -> Self {
        Self {
            keys,
            scopes: vec![ScopeBinding::on(Panel::Picker, cmd)],
            context: None,
        }
    }

    fn desc(cmd: &'static Command, keys: Vec<KeyBinding>) -> Self {
        Self {
            keys,
            scopes: vec![ScopeBinding::on(Panel::Description, cmd)],
            context: None,
        }
    }

    fn multi(keys: Vec<KeyBinding>, scopes: Vec<ScopeBinding>) -> Self {
        Self {
            keys,
            scopes,
            context: None,
        }
    }
}

// ── Lookup result ─────────────────────────────────────────────────────

pub enum LookupResult {
    Command(&'static Command),
    CustomAction(CustomAction),
    PendingPrefix(char),
    None,
}

// ── Keymap ────────────────────────────────────────────────────────────

type PendingMap = HashMap<char, HashMap<KeyCode, Vec<ScopeBinding>>>;

pub struct Keymap {
    diff_view: HashMap<KeyCombo, Vec<Binding>>,
    file_picker: HashMap<KeyCombo, &'static Command>,
    description: HashMap<KeyCombo, &'static Command>,
    pending: PendingMap,
    labels: HashMap<&'static str, Vec<String>>,
    custom_actions: HashMap<KeyCombo, CustomAction>,
    all_custom_actions: Vec<CustomAction>,
    aliases: HashMap<String, String>,
    disabled_commands: Vec<String>,
    pub warnings: Vec<String>,
}

impl Keymap {
    pub fn from_config(user_config: &UserConfig, resolved: ResolvedActions) -> Self {
        let mut defs = Self::default_binding_defs();
        let mut warnings = Vec::new();
        Self::apply_overrides(&mut defs, user_config, &mut warnings);
        Self::filter_disabled(&mut defs, &user_config.disabled_commands);
        warnings.extend(resolved.warnings.iter().cloned());
        let mut keymap = Self::build(
            defs,
            resolved,
            user_config.aliases.clone(),
            user_config.disabled_commands.clone(),
        );
        keymap.warnings = warnings;
        keymap
    }

    pub fn lookup(
        &self,
        key: &KeyEvent,
        focus: Focus,
        context: RowContext,
    ) -> LookupResult {
        let combo = KeyCombo::from(key);

        if matches!(focus, Focus::DiffView | Focus::Description)
            && let KeyCode::Char(c) = combo.code
            && combo.modifiers == KeyModifiers::NONE
            && self.pending.contains_key(&c)
        {
            return LookupResult::PendingPrefix(c);
        }

        match focus {
            Focus::DiffView => {
                if let Some(bindings) = self.diff_view.get(&combo)
                    && let Some(b) = bindings.iter().find(|b| match b.context {
                        Some(ctx) => context.matches(ctx),
                        None => true,
                    })
                {
                    return LookupResult::Command(b.command);
                }
                if let Some(action) = self.custom_actions.get(&combo) {
                    return LookupResult::CustomAction(action.clone());
                }
                LookupResult::None
            }
            Focus::FilePicker => self
                .file_picker
                .get(&combo)
                .copied()
                .map(LookupResult::Command)
                .unwrap_or(LookupResult::None),
            Focus::Description => self
                .description
                .get(&combo)
                .copied()
                .map(LookupResult::Command)
                .unwrap_or(LookupResult::None),
        }
    }

    pub fn lookup_pending(
        &self,
        prefix: char,
        code: KeyCode,
        focus: Focus,
    ) -> Option<&'static Command> {
        let panel = match focus {
            Focus::DiffView => Panel::Diff,
            Focus::FilePicker => Panel::Picker,
            Focus::Description => Panel::Description,
        };
        self.pending
            .get(&prefix)
            .and_then(|m| m.get(&code))
            .and_then(|bindings| {
                bindings
                    .iter()
                    .find(|s| s.panel == panel)
                    .map(|s| s.command)
            })
    }

    /// Get the primary display label for a command (e.g. "c", "gg", "Ctrl-d").
    pub fn key_label(&self, command_name: &str) -> String {
        self.labels
            .get(command_name)
            .and_then(|v| v.first())
            .cloned()
            .unwrap_or_default()
    }

    /// Get all display labels for a command (e.g. ["j", "↓"] for scroll_down).
    fn key_labels(&self, command_name: &str) -> String {
        self.labels
            .get(command_name)
            .map(|v| v.join(", "))
            .unwrap_or_default()
    }

    /// Build help overlay entries matching the original curated layout.
    /// Keys resolve dynamically from the active keymap.
    pub fn help_bindings(&self) -> Vec<(String, &'static str)> {
        let all = |name: &str| self.key_labels(name);
        let one = |name: &str| self.key_label(name);
        let pair = |a: &str, b: &str| format!("{}, {}", all(a), all(b));

        vec![
            // Navigation
            (pair("scroll_down", "scroll_up"), "Scroll line"),
            (format!("{} / {}", one("goto_first"), one("goto_last")), "Go to first / last line"),
            (format!("{} / {}", one("half_page_down"), one("half_page_up")), "Half page down / up"),
            (format!("{} / {}", one("full_page_down"), one("full_page_up")), "Full page down / up"),
            (format!("{} / {} / {}", one("screen_top"), one("screen_middle"), one("screen_bottom")), "Screen top / middle / bottom"),
            (format!("{} / {} / {}", one("center_cursor"), one("scroll_cursor_top"), one("scroll_cursor_bottom")), "Center / top / bottom cursor"),
            (String::new(), ""),
            // Jumps
            (all("next_hunk"), "Next hunk"),
            (all("prev_hunk"), "Previous hunk"),
            (format!("{} / {}", one("next_change"), one("prev_change")), "Next / previous change"),
            (format!("{} / {}", one("next_comment"), one("prev_comment")), "Next / previous comment"),
            (String::new(), ""),
            // Search
            (one("search_forward"), "Search forward in diff"),
            (one("search_backward"), "Search backward in diff"),
            (format!("{} / {}", one("next_match_or_file"), one("prev_match_or_file")), "Next / prev match (or file)"),
            (one("escape"), "Clear search / cancel / quit"),
            (String::new(), ""),
            // Code actions
            (one("visual"), "Visual select (multi-line)"),
            (one("expand"), "Expand context (+10 lines)"),
            (format!("{} / {}", one("fold_open"), one("fold_close")), "Open / close file fold"),
            (String::new(), ""),
            // Review actions
            (one("comment_on_line"), "Comment on line"),
            (one("suggest"), "Suggest change on current line"),
            (one("submit"), "Submit review"),
            (String::new(), ""),
            // General
            (one("switch_focus"), "Switch focus: files ↔ diff"),
            (one("toggle_view"), "Toggle unified / side-by-side"),
            (one("open_browser"), "Open in browser"),
            (one("quit"), "Quit"),
        ]
    }

    /// Build context-specific hint spans for the review bar.
    /// Returns Vec<(key_label, description)>.
    pub fn context_hint_pairs(&self, context: RowContext) -> Vec<(&'static str, String)> {
        match context {
            RowContext::File => vec![
                ("fold", self.key_label("fold_toggle")),
                ("approve", self.key_label("approve")),
                ("submit", self.key_label("submit")),
            ],
            RowContext::Code => vec![
                ("comment", self.key_label("comment_on_line")),
                ("suggest", self.key_label("suggest")),
                ("visual", self.key_label("visual")),
                ("approve", self.key_label("approve")),
                ("submit", self.key_label("submit")),
            ],
            RowContext::Comment => vec![
                ("reply", self.key_label("comment_on_line")),
                ("resolve", self.key_label("resolve")),
                ("discard", self.key_label("discard")),
                ("toggle", self.key_label("toggle_comment")),
            ],
            RowContext::Suggestion => vec![
                ("accept", self.key_label("accept_suggestion")),
                ("reply", self.key_label("comment_on_line")),
                ("resolve", self.key_label("resolve")),
            ],
        }
    }

    /// Find a custom action by name (for command bar resolution).
    pub fn find_custom_action(&self, name: &str) -> Option<&CustomAction> {
        self.all_custom_actions.iter().find(|a| a.name == name)
    }

    /// Resolve an alias to a built-in command name. Returns None if not an alias.
    pub fn resolve_alias(&self, name: &str) -> Option<&'static Command> {
        let target = self.aliases.get(name)?;
        command::Command::by_name(target)
    }

    /// Get all alias names for command bar completion.
    pub fn alias_entries(&self) -> impl Iterator<Item = (&String, &String)> {
        self.aliases.iter()
    }

    /// Check if a command is disabled.
    pub fn is_disabled(&self, name: &str) -> bool {
        self.disabled_commands.iter().any(|d| d == name)
    }

    /// Get all named custom actions for command bar completion.
    pub fn named_custom_actions(&self) -> impl Iterator<Item = &CustomAction> {
        self.all_custom_actions.iter().filter(|a| !a.name.is_empty())
    }

    /// Returns (key_label, description) pairs for all custom actions, for the help overlay.
    pub fn custom_action_help(&self) -> Vec<(String, String)> {
        self.custom_actions
            .iter()
            .map(|(combo, action)| {
                let label =
                    format_key_binding(&KeyBinding::Single(combo.clone()));
                (label, action.description.clone())
            })
            .collect()
    }

    // ── Private: declarative binding definitions ──────────────────────

    fn default_binding_defs() -> Vec<BindingDef> {
        use BindingDef as B;
        use KeyBinding::{Pending, Single};
        use Panel::{Description, Diff, Picker};
        use ScopeBinding as S;

        let key_down = vec![Single('j'.into()), Single(KeyCode::Down.into())];
        let key_up = vec![Single('k'.into()), Single(KeyCode::Up.into())];

        let mut defs = Vec::new();

        // ── Global keys (all panels) ──────────────────────────────────
        defs.extend([
            B::global(&command::quit, vec![Single('q'.into())]),
            B::multi(
                vec![Single(KeyCode::Esc.into())],
                vec![
                    S::on(Diff, &command::escape),
                    S::on(Picker, &command::escape),
                    S::on(Description, &command::desc_close),
                ],
            ),
            B::global(&command::switch_focus, vec![Single(KeyCode::Tab.into())]),
            B::global(&command::prev_panel, vec![Single('h'.into()), Single(KeyCode::Left.into())]),
            B::global(&command::next_panel, vec![Single('l'.into()), Single(KeyCode::Right.into())]),
            B::global(&command::help, vec![
                Single('!'.into()),
                Single(KeyCombo { code: KeyCode::F(1), modifiers: KeyModifiers::NONE }),
            ]),
            B::global(&command::open_browser, vec![Single('o'.into())]),
            B::picker(&command::help, vec![Single('?'.into())]),
        ]);

        // ── Navigation (per-panel scroll/page/jump) ───────────────────
        defs.extend([
            B::multi(key_down, vec![
                S::on(Diff, &command::scroll_down),
                S::on(Picker, &command::picker_down),
                S::on(Description, &command::desc_scroll_down),
            ]),
            B::multi(key_up, vec![
                S::on(Diff, &command::scroll_up),
                S::on(Picker, &command::picker_up),
                S::on(Description, &command::desc_scroll_up),
            ]),
            B::multi(vec![Single(KeyCombo::ctrl('d'))], vec![
                S::on(Diff, &command::half_page_down),
                S::on(Description, &command::desc_page_down),
            ]),
            B::multi(vec![Single(KeyCombo::ctrl('u'))], vec![
                S::on(Diff, &command::half_page_up),
                S::on(Description, &command::desc_page_up),
            ]),
            B::multi(vec![Single(KeyCombo::ctrl('f'))], vec![
                S::on(Diff, &command::full_page_down),
                S::on(Description, &command::desc_page_down),
            ]),
            B::multi(vec![Single(KeyCombo::ctrl('b'))], vec![
                S::on(Diff, &command::full_page_up),
                S::on(Description, &command::desc_page_up),
            ]),
            B::multi(vec![Single('G'.into())], vec![
                S::on(Diff, &command::goto_last),
                S::on(Description, &command::desc_goto_last),
            ]),
            B::diff(&command::screen_top, vec![Single('H'.into())]),
            B::diff(&command::screen_middle, vec![Single('M'.into())]),
            B::diff(&command::screen_bottom, vec![Single('L'.into())]),
        ]);

        // ── Search (diff only) ────────────────────────────────────────
        defs.extend([
            B::diff(&command::search_forward, vec![Single('/'.into())]),
            B::diff(&command::search_backward, vec![Single('?'.into())]),
            B::diff(&command::next_match_or_file, vec![Single('n'.into())]),
            B::diff(&command::prev_match_or_file, vec![Single('N'.into())]),
            B::picker(&command::file_filter, vec![Single('/'.into())]),
        ]);

        // ── Jumps (diff + description) ────────────────────────────────
        defs.extend([
            B::multi(vec![Single(']'.into()), Single('}'.into())], vec![
                S::on(Diff, &command::next_hunk),
                S::on(Description, &command::desc_next_section),
            ]),
            B::multi(vec![Single('['.into()), Single('{'.into())], vec![
                S::on(Diff, &command::prev_hunk),
                S::on(Description, &command::desc_prev_section),
            ]),
            B::diff(&command::next_change, vec![Single(')'.into())]),
            B::diff(&command::prev_change, vec![Single('('.into())]),
        ]);

        // ── View / UI ─────────────────────────────────────────────────
        defs.extend([
            B::diff(&command::toggle_view, vec![Single('t'.into())]),
            B::diff(&command::open_command_mode, vec![Single(':'.into())]),
        ]);

        // ── Review actions (diff only) ────────────────────────────────
        defs.extend([
            B::diff(&command::approve, vec![Single('a'.into())]),
            B::diff(&command::submit, vec![Single('s'.into())]),
            B::diff(&command::unapprove, vec![Single('u'.into())]),
        ]);

        // ── Diff context-sensitive keys ───────────────────────────────
        defs.extend([
            B::diff_ctx(&command::fold_toggle, vec![Single(KeyCode::Enter.into())], RowContext::File),
            B::diff_ctx(&command::comment_on_line, vec![Single('c'.into())], RowContext::Code),
            B::diff_ctx(&command::suggest, vec![Single('e'.into())], RowContext::Code),
            B::diff_ctx(&command::expand, vec![Single('E'.into())], RowContext::Code),
            B::diff_ctx(&command::visual, vec![Single('v'.into()), Single('V'.into())], RowContext::Code),
            B::diff_ctx(&command::toggle_comment, vec![Single(KeyCode::Enter.into())], RowContext::Comment),
            B::diff_ctx(&command::comment_on_line, vec![Single('c'.into())], RowContext::Comment),
            B::diff_ctx(&command::resolve, vec![Single('r'.into())], RowContext::Comment),
            B::diff_ctx(&command::discard, vec![Single('x'.into())], RowContext::Comment),
            B::diff_ctx(&command::accept_suggestion, vec![Single('y'.into())], RowContext::Suggestion),
        ]);

        // ── Description panel keys ────────────────────────────────────
        defs.push(B::desc(&command::edit_description, vec![Single('e'.into())]));

        // ── Pending sequences (two-key combos) ────────────────────────
        defs.extend([
            B::multi(vec![Pending { prefix: 'g', key: 'g' }], vec![
                S::on(Diff, &command::goto_first),
                S::on(Description, &command::desc_goto_first),
            ]),
            B::diff(&command::center_cursor, vec![Pending { prefix: 'z', key: 'z' }]),
            B::diff(&command::scroll_cursor_top, vec![Pending { prefix: 'z', key: 't' }]),
            B::diff(&command::scroll_cursor_bottom, vec![Pending { prefix: 'z', key: 'b' }]),
            B::diff(&command::fold_open, vec![Pending { prefix: 'z', key: 'o' }]),
            B::diff(&command::fold_close, vec![Pending { prefix: 'z', key: 'c' }]),
            B::diff(&command::next_comment, vec![Pending { prefix: 'g', key: 'c' }]),
            B::diff(&command::prev_comment, vec![Pending { prefix: 'g', key: 'C' }]),
        ]);

        defs
    }

    fn apply_overrides(
        defs: &mut Vec<BindingDef>,
        config: &UserConfig,
        warnings: &mut Vec<String>,
    ) {
        for (cmd_name, key_or_keys) in &config.keys {
            let key_strings = key_or_keys.to_vec();

            let is_no_op = key_strings.iter().any(|s| s == "no_op");

            let mut new_keys = Vec::new();
            if !is_no_op {
                for s in &key_strings {
                    if let Some(kb) = parse_key_string(s) {
                        new_keys.push(kb);
                    } else {
                        warnings.push(format!("Invalid key string: {s:?}"));
                    }
                }
                if new_keys.is_empty() {
                    continue;
                }
            }

            let mut found = false;
            for def in defs.iter_mut() {
                let matches = def.scopes.iter().any(|sb| sb.command.name == cmd_name);
                if matches {
                    def.keys = new_keys.clone();
                    found = true;
                }
            }
            if !found {
                let resolved_cmd = command::Command::by_name(cmd_name).or_else(|| {
                    config
                        .aliases
                        .get(cmd_name)
                        .and_then(|target| command::Command::by_name(target))
                });
                if let Some(cmd) = resolved_cmd {
                    defs.push(BindingDef::diff(cmd, new_keys));
                } else {
                    warnings.push(format!("Unknown command in config: {cmd_name}"));
                }
            }
        }
    }

    fn filter_disabled(defs: &mut Vec<BindingDef>, disabled: &[String]) {
        if disabled.is_empty() {
            return;
        }
        defs.retain(|def| {
            !def.scopes
                .iter()
                .any(|sb| disabled.iter().any(|d| d == sb.command.name))
        });
    }

    fn build(
        defs: Vec<BindingDef>,
        resolved: ResolvedActions,
        aliases: HashMap<String, String>,
        disabled_commands: Vec<String>,
    ) -> Self {
        let mut diff_view: HashMap<KeyCombo, Vec<Binding>> = HashMap::new();
        let mut file_picker: HashMap<KeyCombo, &'static Command> = HashMap::new();
        let mut description: HashMap<KeyCombo, &'static Command> = HashMap::new();
        let mut pending: PendingMap = HashMap::new();
        let mut labels: HashMap<&'static str, Vec<String>> = HashMap::new();

        for def in &defs {
            // Collect labels from the first scope binding's command name
            if let Some(first) = def.scopes.first() {
                let entry = labels.entry(first.command.name).or_default();
                for key in &def.keys {
                    let label = format_key_binding(key);
                    if !entry.contains(&label) {
                        entry.push(label);
                    }
                }
            }

            for sb in &def.scopes {
                for key in &def.keys {
                    match key {
                        KeyBinding::Single(combo) => match sb.panel {
                            Panel::Diff => {
                                diff_view
                                    .entry(combo.clone())
                                    .or_default()
                                    .push(Binding {
                                        command: sb.command,
                                        context: def.context,
                                    });
                            }
                            Panel::Picker => {
                                file_picker.insert(combo.clone(), sb.command);
                            }
                            Panel::Description => {
                                description.insert(combo.clone(), sb.command);
                            }
                        },
                        KeyBinding::Pending { prefix, key } => {
                            pending
                                .entry(*prefix)
                                .or_default()
                                .entry(KeyCode::Char(*key))
                                .or_default()
                                .push(*sb);
                        }
                    }
                }
            }
        }

        Self {
            diff_view,
            file_picker,
            description,
            pending,
            labels,
            custom_actions: resolved.keyed,
            all_custom_actions: resolved.all,
            aliases,
            disabled_commands,
            warnings: Vec::new(),
        }
    }
}

impl Default for Keymap {
    fn default() -> Self {
        Self::build(
            Self::default_binding_defs(),
            ResolvedActions {
                keyed: HashMap::new(),
                all: Vec::new(),
                warnings: Vec::new(),
            },
            HashMap::new(),
            Vec::new(),
        )
    }
}
