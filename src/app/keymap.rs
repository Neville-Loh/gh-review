use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::Focus;
use crate::types::RowContext;

use super::command::{self, Command};

#[derive(Clone, Hash, Eq, PartialEq)]
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

struct Binding {
    command: &'static Command,
    context: Option<RowContext>,
}

pub struct Keymap {
    diff_view: HashMap<KeyCombo, Vec<Binding>>,
    file_picker: HashMap<KeyCombo, &'static Command>,
    pending: HashMap<char, HashMap<KeyCode, &'static Command>>,
}

impl Keymap {
    pub fn lookup(
        &self,
        key: &KeyEvent,
        focus: Focus,
        context: RowContext,
    ) -> Option<&'static Command> {
        let combo = KeyCombo::from(key);
        match focus {
            Focus::DiffView => {
                let bindings = self.diff_view.get(&combo)?;
                bindings
                    .iter()
                    .find(|b| match b.context {
                        Some(ctx) => context.matches(ctx),
                        None => true,
                    })
                    .map(|b| b.command)
            }
            Focus::FilePicker => self.file_picker.get(&combo).copied(),
        }
    }

    pub fn lookup_pending(&self, prefix: char, code: KeyCode) -> Option<&'static Command> {
        self.pending
            .get(&prefix)
            .and_then(|m| m.get(&code))
            .copied()
    }

    #[allow(dead_code)]
    pub fn bindings_for_context(&self, context: RowContext) -> Vec<(&'static str, &'static str)> {
        let mut hints = Vec::new();
        for bindings in self.diff_view.values() {
            for b in bindings {
                if let Some(ctx) = b.context
                    && context.matches(ctx)
                {
                    hints.push((b.command.name, b.command.doc));
                }
            }
        }
        hints.sort_by_key(|(name, _)| *name);
        hints.dedup_by_key(|(name, _)| *name);
        hints
    }
}

impl Default for Keymap {
    fn default() -> Self {
        let mut diff_view: HashMap<KeyCombo, Vec<Binding>> = HashMap::new();
        let mut file_picker = HashMap::new();
        let mut pending: HashMap<char, HashMap<KeyCode, &'static Command>> = HashMap::new();

        macro_rules! bind {
            ($key:expr, $cmd:expr) => {
                diff_view
                    .entry($key)
                    .or_default()
                    .push(Binding { command: $cmd, context: None });
            };
            ($key:expr, $cmd:expr, $ctx:expr) => {
                diff_view
                    .entry($key)
                    .or_default()
                    .push(Binding { command: $cmd, context: Some($ctx) });
            };
        }

        macro_rules! both {
            ($key:expr, $cmd:expr) => {
                bind!($key, $cmd);
                file_picker.insert($key, $cmd);
            };
        }

        // --- Shared keys (both focus contexts) ---
        both!(KeyCode::Char('q').into(), &command::quit);
        both!(KeyCode::Esc.into(), &command::escape);
        both!(
            KeyCombo {
                code: KeyCode::Char('!'),
                modifiers: KeyModifiers::NONE
            },
            &command::help
        );
        both!(
            KeyCombo {
                code: KeyCode::F(1),
                modifiers: KeyModifiers::NONE
            },
            &command::help
        );
        both!(KeyCode::Tab.into(), &command::switch_focus);

        // --- Focus-dependent keys ---
        bind!(KeyCode::Char('/').into(), &command::search_forward);
        file_picker.insert(KeyCode::Char('/').into(), &command::file_filter);

        bind!(KeyCode::Char('?').into(), &command::search_backward);
        file_picker.insert(KeyCode::Char('?').into(), &command::help);

        bind!(KeyCode::Char('j').into(), &command::scroll_down);
        bind!(KeyCode::Down.into(), &command::scroll_down);
        file_picker.insert(KeyCode::Char('j').into(), &command::picker_down);
        file_picker.insert(KeyCode::Down.into(), &command::picker_down);

        bind!(KeyCode::Char('k').into(), &command::scroll_up);
        bind!(KeyCode::Up.into(), &command::scroll_up);
        file_picker.insert(KeyCode::Char('k').into(), &command::picker_up);
        file_picker.insert(KeyCode::Up.into(), &command::picker_up);

        // --- Navigation (any context) ---
        bind!(KeyCombo::ctrl('d'), &command::half_page_down);
        bind!(KeyCombo::ctrl('u'), &command::half_page_up);
        bind!(KeyCombo::ctrl('f'), &command::full_page_down);
        bind!(KeyCombo::ctrl('b'), &command::full_page_up);

        bind!(KeyCode::Char('g').into(), &command::pending_g);
        bind!(KeyCode::Char('z').into(), &command::pending_z);

        bind!(KeyCode::Char('G').into(), &command::goto_last);
        bind!(KeyCode::Char('H').into(), &command::screen_top);
        bind!(KeyCode::Char('M').into(), &command::screen_middle);
        bind!(KeyCode::Char('L').into(), &command::screen_bottom);

        bind!(KeyCode::Char(']').into(), &command::next_hunk);
        bind!(KeyCode::Char('}').into(), &command::next_hunk);
        bind!(KeyCode::Char('[').into(), &command::prev_hunk);
        bind!(KeyCode::Char('{').into(), &command::prev_hunk);

        bind!(KeyCode::Char(')').into(), &command::next_change);
        bind!(KeyCode::Char('(').into(), &command::prev_change);

        bind!(KeyCode::Char('n').into(), &command::next_match_or_file);
        bind!(KeyCode::Char('N').into(), &command::prev_match_or_file);

        bind!(KeyCode::Char('t').into(), &command::toggle_view);
        bind!(KeyCode::Char('o').into(), &command::open_browser);
        bind!(KeyCode::Char(':').into(), &command::open_command_mode);

        // --- Global review actions (any context) ---
        bind!(KeyCode::Char('a').into(), &command::approve);
        bind!(KeyCode::Char('s').into(), &command::submit);
        bind!(KeyCode::Char('u').into(), &command::unapprove);

        // --- Context-specific keys ---
        // Enter: fold on File, toggle comment on Comment
        bind!(KeyCode::Enter.into(), &command::fold_toggle, RowContext::File);
        bind!(KeyCode::Enter.into(), &command::toggle_comment, RowContext::Comment);

        // Code context
        bind!(KeyCode::Char('c').into(), &command::comment_on_line, RowContext::Code);
        bind!(KeyCode::Char('e').into(), &command::suggest, RowContext::Code);
        bind!(KeyCode::Char('E').into(), &command::expand, RowContext::Code);
        bind!(KeyCode::Char('v').into(), &command::visual, RowContext::Code);
        bind!(KeyCode::Char('V').into(), &command::visual, RowContext::Code);

        // Comment context
        bind!(KeyCode::Char('c').into(), &command::comment_on_line, RowContext::Comment);
        bind!(KeyCode::Char('r').into(), &command::resolve, RowContext::Comment);
        bind!(KeyCode::Char('x').into(), &command::discard, RowContext::Comment);

        // Suggestion context (sub-context of Comment)
        bind!(KeyCode::Char('y').into(), &command::accept_suggestion, RowContext::Suggestion);

        // --- Two-key (pending) sequences ---
        let mut g_map = HashMap::new();
        g_map.insert(KeyCode::Char('g'), &command::goto_first as &'static Command);
        pending.insert('g', g_map);

        let mut z_map = HashMap::new();
        z_map.insert(KeyCode::Char('z'), &command::center_cursor as &'static Command);
        z_map.insert(KeyCode::Char('t'), &command::scroll_cursor_top as &'static Command);
        z_map.insert(KeyCode::Char('b'), &command::scroll_cursor_bottom as &'static Command);
        z_map.insert(KeyCode::Char('o'), &command::fold_open as &'static Command);
        z_map.insert(KeyCode::Char('c'), &command::fold_close as &'static Command);
        pending.insert('z', z_map);

        Self {
            diff_view,
            file_picker,
            pending,
        }
    }
}
