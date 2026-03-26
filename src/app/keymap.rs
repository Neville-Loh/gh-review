use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::Focus;
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
        Self {
            code: event.code,
            modifiers: event.modifiers,
        }
    }
}

pub struct Keymap {
    diff_view: HashMap<KeyCombo, &'static Command>,
    file_picker: HashMap<KeyCombo, &'static Command>,
    pending: HashMap<char, HashMap<KeyCode, &'static Command>>,
}

impl Keymap {
    pub fn lookup(&self, key: &KeyEvent, focus: Focus) -> Option<&'static Command> {
        let combo = KeyCombo::from(key);
        match focus {
            Focus::DiffView => self.diff_view.get(&combo),
            Focus::FilePicker => self.file_picker.get(&combo),
        }
        .copied()
    }

    pub fn lookup_pending(&self, prefix: char, code: KeyCode) -> Option<&'static Command> {
        self.pending
            .get(&prefix)
            .and_then(|m| m.get(&code))
            .copied()
    }
}

impl Default for Keymap {
    fn default() -> Self {
        let mut diff_view = HashMap::new();
        let mut file_picker = HashMap::new();
        let mut pending: HashMap<char, HashMap<KeyCode, &'static Command>> = HashMap::new();

        macro_rules! both {
            ($key:expr, $cmd:expr) => {
                diff_view.insert($key, $cmd);
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
        diff_view.insert(KeyCode::Char('/').into(), &command::search_forward);
        file_picker.insert(KeyCode::Char('/').into(), &command::file_filter);

        diff_view.insert(KeyCode::Char('?').into(), &command::search_backward);
        file_picker.insert(KeyCode::Char('?').into(), &command::help);

        diff_view.insert(KeyCode::Char('j').into(), &command::scroll_down);
        diff_view.insert(KeyCode::Down.into(), &command::scroll_down);
        file_picker.insert(KeyCode::Char('j').into(), &command::picker_down);
        file_picker.insert(KeyCode::Down.into(), &command::picker_down);

        diff_view.insert(KeyCode::Char('k').into(), &command::scroll_up);
        diff_view.insert(KeyCode::Up.into(), &command::scroll_up);
        file_picker.insert(KeyCode::Char('k').into(), &command::picker_up);
        file_picker.insert(KeyCode::Up.into(), &command::picker_up);

        // --- Diff-only keys ---
        diff_view.insert(KeyCombo::ctrl('d'), &command::half_page_down);
        diff_view.insert(KeyCombo::ctrl('u'), &command::half_page_up);
        diff_view.insert(KeyCombo::ctrl('f'), &command::full_page_down);
        diff_view.insert(KeyCombo::ctrl('b'), &command::full_page_up);

        diff_view.insert(KeyCode::Char('g').into(), &command::pending_g);
        diff_view.insert(KeyCode::Char('z').into(), &command::pending_z);

        diff_view.insert(KeyCode::Char('G').into(), &command::goto_last);
        diff_view.insert(KeyCode::Char('H').into(), &command::screen_top);
        diff_view.insert(KeyCode::Char('M').into(), &command::screen_middle);
        diff_view.insert(KeyCode::Char('L').into(), &command::screen_bottom);

        diff_view.insert(KeyCode::Char(']').into(), &command::next_hunk);
        diff_view.insert(KeyCode::Char('}').into(), &command::next_hunk);
        diff_view.insert(KeyCode::Char('[').into(), &command::prev_hunk);
        diff_view.insert(KeyCode::Char('{').into(), &command::prev_hunk);

        diff_view.insert(KeyCode::Char(')').into(), &command::next_change);
        diff_view.insert(KeyCode::Char('(').into(), &command::prev_change);

        diff_view.insert(KeyCode::Char('n').into(), &command::next_match_or_file);
        diff_view.insert(KeyCode::Char('N').into(), &command::prev_match_or_file);

        diff_view.insert(KeyCode::Enter.into(), &command::toggle_comment);
        diff_view.insert(KeyCode::Char('t').into(), &command::toggle_view);
        diff_view.insert(KeyCode::Char('c').into(), &command::comment);
        diff_view.insert(KeyCode::Char('e').into(), &command::suggest);
        diff_view.insert(KeyCode::Char('E').into(), &command::expand);

        diff_view.insert(KeyCode::Char('a').into(), &command::approve);
        diff_view.insert(KeyCode::Char('r').into(), &command::request_changes);
        diff_view.insert(KeyCode::Char('s').into(), &command::submit);

        diff_view.insert(KeyCode::Char('x').into(), &command::discard);
        diff_view.insert(KeyCode::Char('R').into(), &command::resolve);
        diff_view.insert(KeyCode::Char('y').into(), &command::accept_suggestion);
        diff_view.insert(KeyCode::Char('v').into(), &command::visual);
        diff_view.insert(KeyCode::Char('u').into(), &command::unapprove);
        diff_view.insert(KeyCode::Char('o').into(), &command::open_browser);
        diff_view.insert(KeyCode::Char(':').into(), &command::open_command_mode);

        // --- Two-key (pending) sequences ---
        let mut g_map = HashMap::new();
        g_map.insert(KeyCode::Char('g'), &command::goto_first as &'static Command);
        pending.insert('g', g_map);

        let mut z_map = HashMap::new();
        z_map.insert(KeyCode::Char('z'), &command::center_cursor as &'static Command);
        z_map.insert(KeyCode::Char('t'), &command::scroll_cursor_top as &'static Command);
        z_map.insert(KeyCode::Char('b'), &command::scroll_cursor_bottom as &'static Command);
        pending.insert('z', z_map);

        Self {
            diff_view,
            file_picker,
            pending,
        }
    }
}
