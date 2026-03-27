use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::Focus;
use crate::config::{KeyBinding, UserConfig, format_key_binding, parse_key_string};
use crate::types::RowContext;

use super::command::{self, Command};
use super::custom_action::{CustomAction, ResolvedActions};

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

#[derive(Clone, Copy)]
enum Scope {
    Global,
    DiffOnly,
    PickerOnly,
}

struct BindingDef {
    command: &'static Command,
    keys: Vec<KeyBinding>,
    scope: Scope,
    context: Option<RowContext>,
}

// ── Lookup result ─────────────────────────────────────────────────────

pub enum LookupResult {
    Command(&'static Command),
    CustomAction(CustomAction),
    PendingPrefix(char),
    None,
}

// ── Keymap ────────────────────────────────────────────────────────────

pub struct Keymap {
    diff_view: HashMap<KeyCombo, Vec<Binding>>,
    file_picker: HashMap<KeyCombo, &'static Command>,
    pending: HashMap<char, HashMap<KeyCode, &'static Command>>,
    labels: HashMap<&'static str, Vec<String>>,
    custom_actions: HashMap<KeyCombo, CustomAction>,
    all_custom_actions: Vec<CustomAction>,
    aliases: HashMap<String, String>,
    disabled_commands: Vec<String>,
}

impl Keymap {
    pub fn from_config(user_config: &UserConfig, resolved: ResolvedActions) -> Self {
        let mut defs = Self::default_binding_defs();
        Self::apply_overrides(&mut defs, user_config);
        Self::filter_disabled(&mut defs, &user_config.disabled_commands);
        Self::build(
            defs,
            resolved,
            user_config.aliases.clone(),
            user_config.disabled_commands.clone(),
        )
    }

    pub fn lookup(
        &self,
        key: &KeyEvent,
        focus: Focus,
        context: RowContext,
    ) -> LookupResult {
        let combo = KeyCombo::from(key);

        if matches!(focus, Focus::DiffView)
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
        }
    }

    pub fn lookup_pending(&self, prefix: char, code: KeyCode) -> Option<&'static Command> {
        self.pending
            .get(&prefix)
            .and_then(|m| m.get(&code))
            .copied()
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
        use KeyBinding::{Pending, Single};
        use Scope::{DiffOnly, Global, PickerOnly};

        vec![
            // ── Global ────────────────────────────────────────────────
            BindingDef {
                command: &command::quit,
                keys: vec![Single('q'.into())],
                scope: Global,
                context: None,
            },
            BindingDef {
                command: &command::escape,
                keys: vec![Single(KeyCode::Esc.into())],
                scope: Global,
                context: None,
            },
            BindingDef {
                command: &command::switch_focus,
                keys: vec![Single(KeyCode::Tab.into())],
                scope: Global,
                context: None,
            },
            BindingDef {
                command: &command::help,
                keys: vec![
                    Single('!'.into()),
                    Single(KeyCombo {
                        code: KeyCode::F(1),
                        modifiers: KeyModifiers::NONE,
                    }),
                ],
                scope: Global,
                context: None,
            },
            // file picker also maps ? to help
            BindingDef {
                command: &command::help,
                keys: vec![Single('?'.into())],
                scope: PickerOnly,
                context: None,
            },
            // ── Navigation ────────────────────────────────────────────
            BindingDef {
                command: &command::scroll_down,
                keys: vec![Single('j'.into()), Single(KeyCode::Down.into())],
                scope: DiffOnly,
                context: None,
            },
            BindingDef {
                command: &command::scroll_up,
                keys: vec![Single('k'.into()), Single(KeyCode::Up.into())],
                scope: DiffOnly,
                context: None,
            },
            BindingDef {
                command: &command::picker_down,
                keys: vec![Single('j'.into()), Single(KeyCode::Down.into())],
                scope: PickerOnly,
                context: None,
            },
            BindingDef {
                command: &command::picker_up,
                keys: vec![Single('k'.into()), Single(KeyCode::Up.into())],
                scope: PickerOnly,
                context: None,
            },
            BindingDef {
                command: &command::half_page_down,
                keys: vec![Single(KeyCombo::ctrl('d'))],
                scope: DiffOnly,
                context: None,
            },
            BindingDef {
                command: &command::half_page_up,
                keys: vec![Single(KeyCombo::ctrl('u'))],
                scope: DiffOnly,
                context: None,
            },
            BindingDef {
                command: &command::full_page_down,
                keys: vec![Single(KeyCombo::ctrl('f'))],
                scope: DiffOnly,
                context: None,
            },
            BindingDef {
                command: &command::full_page_up,
                keys: vec![Single(KeyCombo::ctrl('b'))],
                scope: DiffOnly,
                context: None,
            },
            BindingDef {
                command: &command::goto_last,
                keys: vec![Single('G'.into())],
                scope: DiffOnly,
                context: None,
            },
            BindingDef {
                command: &command::screen_top,
                keys: vec![Single('H'.into())],
                scope: DiffOnly,
                context: None,
            },
            BindingDef {
                command: &command::screen_middle,
                keys: vec![Single('M'.into())],
                scope: DiffOnly,
                context: None,
            },
            BindingDef {
                command: &command::screen_bottom,
                keys: vec![Single('L'.into())],
                scope: DiffOnly,
                context: None,
            },
            // ── Search ────────────────────────────────────────────────
            BindingDef {
                command: &command::search_forward,
                keys: vec![Single('/'.into())],
                scope: DiffOnly,
                context: None,
            },
            BindingDef {
                command: &command::search_backward,
                keys: vec![Single('?'.into())],
                scope: DiffOnly,
                context: None,
            },
            BindingDef {
                command: &command::next_match_or_file,
                keys: vec![Single('n'.into())],
                scope: DiffOnly,
                context: None,
            },
            BindingDef {
                command: &command::prev_match_or_file,
                keys: vec![Single('N'.into())],
                scope: DiffOnly,
                context: None,
            },
            BindingDef {
                command: &command::file_filter,
                keys: vec![Single('/'.into())],
                scope: PickerOnly,
                context: None,
            },
            // ── Hunks ─────────────────────────────────────────────────
            BindingDef {
                command: &command::next_hunk,
                keys: vec![Single(']'.into()), Single('}'.into())],
                scope: DiffOnly,
                context: None,
            },
            BindingDef {
                command: &command::prev_hunk,
                keys: vec![Single('['.into()), Single('{'.into())],
                scope: DiffOnly,
                context: None,
            },
            BindingDef {
                command: &command::next_change,
                keys: vec![Single(')'.into())],
                scope: DiffOnly,
                context: None,
            },
            BindingDef {
                command: &command::prev_change,
                keys: vec![Single('('.into())],
                scope: DiffOnly,
                context: None,
            },
            // ── View / UI ─────────────────────────────────────────────
            BindingDef {
                command: &command::toggle_view,
                keys: vec![Single('t'.into())],
                scope: DiffOnly,
                context: None,
            },
            BindingDef {
                command: &command::open_browser,
                keys: vec![Single('o'.into())],
                scope: DiffOnly,
                context: None,
            },
            BindingDef {
                command: &command::open_command_mode,
                keys: vec![Single(':'.into())],
                scope: DiffOnly,
                context: None,
            },
            // ── Review actions ────────────────────────────────────────
            BindingDef {
                command: &command::approve,
                keys: vec![Single('a'.into())],
                scope: DiffOnly,
                context: None,
            },
            BindingDef {
                command: &command::submit,
                keys: vec![Single('s'.into())],
                scope: DiffOnly,
                context: None,
            },
            BindingDef {
                command: &command::unapprove,
                keys: vec![Single('u'.into())],
                scope: DiffOnly,
                context: None,
            },
            // ── Context: File ─────────────────────────────────────────
            BindingDef {
                command: &command::fold_toggle,
                keys: vec![Single(KeyCode::Enter.into())],
                scope: DiffOnly,
                context: Some(RowContext::File),
            },
            // ── Context: Code ─────────────────────────────────────────
            BindingDef {
                command: &command::comment_on_line,
                keys: vec![Single('c'.into())],
                scope: DiffOnly,
                context: Some(RowContext::Code),
            },
            BindingDef {
                command: &command::suggest,
                keys: vec![Single('e'.into())],
                scope: DiffOnly,
                context: Some(RowContext::Code),
            },
            BindingDef {
                command: &command::expand,
                keys: vec![Single('E'.into())],
                scope: DiffOnly,
                context: Some(RowContext::Code),
            },
            BindingDef {
                command: &command::visual,
                keys: vec![Single('v'.into()), Single('V'.into())],
                scope: DiffOnly,
                context: Some(RowContext::Code),
            },
            // ── Context: Comment ──────────────────────────────────────
            BindingDef {
                command: &command::toggle_comment,
                keys: vec![Single(KeyCode::Enter.into())],
                scope: DiffOnly,
                context: Some(RowContext::Comment),
            },
            BindingDef {
                command: &command::comment_on_line,
                keys: vec![Single('c'.into())],
                scope: DiffOnly,
                context: Some(RowContext::Comment),
            },
            BindingDef {
                command: &command::resolve,
                keys: vec![Single('r'.into())],
                scope: DiffOnly,
                context: Some(RowContext::Comment),
            },
            BindingDef {
                command: &command::discard,
                keys: vec![Single('x'.into())],
                scope: DiffOnly,
                context: Some(RowContext::Comment),
            },
            // ── Context: Suggestion ───────────────────────────────────
            BindingDef {
                command: &command::accept_suggestion,
                keys: vec![Single('y'.into())],
                scope: DiffOnly,
                context: Some(RowContext::Suggestion),
            },
            // ── Pending sequences ─────────────────────────────────────
            BindingDef {
                command: &command::goto_first,
                keys: vec![Pending {
                    prefix: 'g',
                    key: 'g',
                }],
                scope: DiffOnly,
                context: None,
            },
            BindingDef {
                command: &command::center_cursor,
                keys: vec![Pending {
                    prefix: 'z',
                    key: 'z',
                }],
                scope: DiffOnly,
                context: None,
            },
            BindingDef {
                command: &command::scroll_cursor_top,
                keys: vec![Pending {
                    prefix: 'z',
                    key: 't',
                }],
                scope: DiffOnly,
                context: None,
            },
            BindingDef {
                command: &command::scroll_cursor_bottom,
                keys: vec![Pending {
                    prefix: 'z',
                    key: 'b',
                }],
                scope: DiffOnly,
                context: None,
            },
            BindingDef {
                command: &command::fold_open,
                keys: vec![Pending {
                    prefix: 'z',
                    key: 'o',
                }],
                scope: DiffOnly,
                context: None,
            },
            BindingDef {
                command: &command::fold_close,
                keys: vec![Pending {
                    prefix: 'z',
                    key: 'c',
                }],
                scope: DiffOnly,
                context: None,
            },
        ]
    }

    fn apply_overrides(defs: &mut Vec<BindingDef>, config: &UserConfig) {
        for (cmd_name, key_or_keys) in &config.keys {
            let key_strings = key_or_keys.to_vec();

            let is_no_op = key_strings.iter().any(|s| s == "no_op");

            let mut new_keys = Vec::new();
            if !is_no_op {
                for s in &key_strings {
                    if let Some(kb) = parse_key_string(s) {
                        new_keys.push(kb);
                    } else {
                        eprintln!("warning: invalid key string: {s:?}");
                    }
                }
                if new_keys.is_empty() {
                    continue;
                }
            }

            let mut found = false;
            for def in defs.iter_mut() {
                if def.command.name == cmd_name {
                    def.keys = new_keys.clone();
                    found = true;
                }
            }
            if !found {
                // Check built-in commands, then aliases
                let resolved_cmd = command::Command::by_name(cmd_name).or_else(|| {
                    config
                        .aliases
                        .get(cmd_name)
                        .and_then(|target| command::Command::by_name(target))
                });
                if let Some(cmd) = resolved_cmd {
                    defs.push(BindingDef {
                        command: cmd,
                        keys: new_keys,
                        scope: Scope::DiffOnly,
                        context: None,
                    });
                } else {
                    eprintln!("warning: unknown command in config: {cmd_name}");
                }
            }
        }
    }

    fn filter_disabled(defs: &mut Vec<BindingDef>, disabled: &[String]) {
        if disabled.is_empty() {
            return;
        }
        defs.retain(|def| !disabled.iter().any(|d| d == def.command.name));
    }

    fn build(
        defs: Vec<BindingDef>,
        resolved: ResolvedActions,
        aliases: HashMap<String, String>,
        disabled_commands: Vec<String>,
    ) -> Self {
        let mut diff_view: HashMap<KeyCombo, Vec<Binding>> = HashMap::new();
        let mut file_picker: HashMap<KeyCombo, &'static Command> = HashMap::new();
        let mut pending: HashMap<char, HashMap<KeyCode, &'static Command>> = HashMap::new();
        let mut labels: HashMap<&'static str, Vec<String>> = HashMap::new();

        for def in &defs {
            let entry = labels.entry(def.command.name).or_default();
            for key in &def.keys {
                let label = format_key_binding(key);
                if !entry.contains(&label) {
                    entry.push(label);
                }
            }

            for key in &def.keys {
                match key {
                    KeyBinding::Single(combo) => match def.scope {
                        Scope::Global => {
                            diff_view
                                .entry(combo.clone())
                                .or_default()
                                .push(Binding {
                                    command: def.command,
                                    context: def.context,
                                });
                            file_picker.insert(combo.clone(), def.command);
                        }
                        Scope::DiffOnly => {
                            diff_view
                                .entry(combo.clone())
                                .or_default()
                                .push(Binding {
                                    command: def.command,
                                    context: def.context,
                                });
                        }
                        Scope::PickerOnly => {
                            file_picker.insert(combo.clone(), def.command);
                        }
                    },
                    KeyBinding::Pending { prefix, key } => {
                        pending
                            .entry(*prefix)
                            .or_default()
                            .insert(KeyCode::Char(*key), def.command);
                    }
                }
            }
        }

        Self {
            diff_view,
            file_picker,
            pending,
            labels,
            custom_actions: resolved.keyed,
            all_custom_actions: resolved.all,
            aliases,
            disabled_commands,
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
            },
            HashMap::new(),
            Vec::new(),
        )
    }
}
