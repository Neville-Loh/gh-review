use crossterm::event::{KeyCode, KeyModifiers};
use crokey::{KeyCombination, KeyCombinationFormat, OneToThree};

use crate::app::keymap::KeyCombo;

/// A resolved key binding: either a single key combo or a two-key pending sequence.
#[derive(Clone, Debug)]
pub enum KeyBinding {
    Single(KeyCombo),
    Pending { prefix: char, key: char },
}

/// Parse a key string from the TOML config into a `KeyBinding`.
///
/// Resolution order:
/// 1. Single character → used literally, preserving case (`"K"`, `"q"`, `"/"`)
/// 2. Two all-lowercase chars → pending sequence (`"gg"`, `"zo"`)
/// 3. Everything else → crokey (`"Up"`, `"Ctrl-d"`, `"Cmd-s"`)
///
/// Named keys and modifiers must start with uppercase to distinguish
/// from pending sequences: `"Up"` = arrow key, `"up"` = u then p.
pub fn parse_key_string(s: &str) -> Option<KeyBinding> {
    if s.chars().count() == 1 {
        let c = s.chars().next().unwrap();
        return Some(KeyBinding::Single(KeyCombo {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::NONE,
        }));
    }

    let bytes = s.as_bytes();
    if bytes.len() == 2 && bytes[0].is_ascii_lowercase() && bytes[1].is_ascii_lowercase() {
        return Some(KeyBinding::Pending {
            prefix: bytes[0] as char,
            key: bytes[1] as char,
        });
    }

    let kc = crokey::parse(s).ok()?;
    let mut code = match kc.codes {
        OneToThree::One(c) => c,
        _ => return None,
    };
    let mut modifiers = kc.modifiers;

    // crokey lowercases everything, so restore uppercase from the original
    // input when the key part (after the last '-') is a single uppercase letter.
    // This lets "Ctrl-D" mean Ctrl+Shift+D (distinct from "Ctrl-d" = Ctrl+d).
    if let KeyCode::Char(_) = code {
        if let Some(key_part) = s.rsplit('-').next()
            && key_part.len() == 1
            && let Some(c) = key_part.chars().next()
            && c.is_ascii_uppercase()
        {
            code = KeyCode::Char(c);
        }
        // Strip SHIFT for Char to match KeyCombo::from(&KeyEvent) behavior
        modifiers.remove(KeyModifiers::SHIFT);
    }

    Some(KeyBinding::Single(KeyCombo { code, modifiers }))
}

fn display_format() -> KeyCombinationFormat {
    KeyCombinationFormat::default().with_implicit_shift()
}

/// Format a `KeyBinding` for display in help overlay / review bar hints.
pub fn format_key_binding(binding: &KeyBinding) -> String {
    match binding {
        KeyBinding::Pending { prefix, key } => format!("{prefix}{key}"),
        KeyBinding::Single(combo) => format_key_combo(combo),
    }
}

/// Format a `KeyCombo` for display (e.g. `Ctrl-d`, `Tab`, `G`).
pub fn format_key_combo(combo: &KeyCombo) -> String {
    let kc = KeyCombination::new(combo.code, combo.modifiers);
    display_format().to_string(kc)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_single(input: &str, expected_code: KeyCode, expected_mods: KeyModifiers) {
        let result = parse_key_string(input).unwrap_or_else(|| panic!("failed to parse: {input}"));
        match result {
            KeyBinding::Single(combo) => {
                assert_eq!(combo.code, expected_code, "code mismatch for {input:?}");
                assert_eq!(combo.modifiers, expected_mods, "modifiers mismatch for {input:?}");
            }
            KeyBinding::Pending { prefix, key } => {
                panic!("expected Single for {input:?}, got Pending({prefix}{key})");
            }
        }
    }

    fn assert_pending(input: &str, expected_prefix: char, expected_key: char) {
        let result = parse_key_string(input).unwrap_or_else(|| panic!("failed to parse: {input}"));
        match result {
            KeyBinding::Pending { prefix, key } => {
                assert_eq!(prefix, expected_prefix, "prefix mismatch for {input:?}");
                assert_eq!(key, expected_key, "key mismatch for {input:?}");
            }
            KeyBinding::Single(combo) => {
                panic!("expected Pending for {input:?}, got Single({:?})", combo);
            }
        }
    }

    #[test]
    fn named_key_uppercase_resolves_to_arrow() {
        assert_single("Up", KeyCode::Up, KeyModifiers::NONE);
        assert_single("Down", KeyCode::Down, KeyModifiers::NONE);
        assert_single("Enter", KeyCode::Enter, KeyModifiers::NONE);
        assert_single("Tab", KeyCode::Tab, KeyModifiers::NONE);
        assert_single("Esc", KeyCode::Esc, KeyModifiers::NONE);
        assert_single("F1", KeyCode::F(1), KeyModifiers::NONE);
        assert_single("Space", KeyCode::Char(' '), KeyModifiers::NONE);
    }

    #[test]
    fn two_lowercase_chars_resolve_to_pending() {
        assert_pending("up", 'u', 'p');
        assert_pending("gg", 'g', 'g');
        assert_pending("zo", 'z', 'o');
        assert_pending("zt", 'z', 't');
        assert_pending("tt", 't', 't');
    }

    #[test]
    fn single_uppercase_char_preserves_case() {
        assert_single("U", KeyCode::Char('U'), KeyModifiers::NONE);
        assert_single("G", KeyCode::Char('G'), KeyModifiers::NONE);
        assert_single("N", KeyCode::Char('N'), KeyModifiers::NONE);
    }

    #[test]
    fn single_lowercase_char() {
        assert_single("u", KeyCode::Char('u'), KeyModifiers::NONE);
        assert_single("q", KeyCode::Char('q'), KeyModifiers::NONE);
        assert_single("/", KeyCode::Char('/'), KeyModifiers::NONE);
        assert_single("!", KeyCode::Char('!'), KeyModifiers::NONE);
    }

    #[test]
    fn ctrl_lowercase_is_ctrl_char() {
        assert_single("Ctrl-d", KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert_single("Ctrl-u", KeyCode::Char('u'), KeyModifiers::CONTROL);
        assert_single("Ctrl-f", KeyCode::Char('f'), KeyModifiers::CONTROL);
    }

    #[test]
    fn ctrl_uppercase_is_ctrl_shift_char() {
        assert_single("Ctrl-D", KeyCode::Char('D'), KeyModifiers::CONTROL);
        assert_single("Ctrl-U", KeyCode::Char('U'), KeyModifiers::CONTROL);
    }

    #[test]
    fn ctrl_shift_same_as_ctrl_uppercase() {
        assert_single("Ctrl-Shift-D", KeyCode::Char('D'), KeyModifiers::CONTROL);
        assert_single("Ctrl-Shift-d", KeyCode::Char('D'), KeyModifiers::CONTROL);
    }

    #[test]
    fn cmd_ctrl_chord() {
        assert_single("Cmd-Ctrl-s", KeyCode::Char('s'), KeyModifiers::SUPER | KeyModifiers::CONTROL);
        assert_single("Cmd-s", KeyCode::Char('s'), KeyModifiers::SUPER);
    }

    #[test]
    fn alt_modifier() {
        assert_single("Alt-x", KeyCode::Char('x'), KeyModifiers::ALT);
        assert_single("Alt-Enter", KeyCode::Enter, KeyModifiers::ALT);
    }

    #[test]
    fn shift_named_key() {
        assert_single("Shift-Tab", KeyCode::Tab, KeyModifiers::SHIFT);
    }

    #[test]
    fn multiple_modifiers() {
        assert_single(
            "Cmd-Shift-1",
            KeyCode::Char('1'),
            KeyModifiers::SUPER,
        );
        assert_single(
            "Ctrl-Alt-d",
            KeyCode::Char('d'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        );
        assert_single(
            "Ctrl-Alt-D",
            KeyCode::Char('D'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        );
        assert_single(
            "Cmd-Ctrl-Shift-x",
            KeyCode::Char('X'),
            KeyModifiers::SUPER | KeyModifiers::CONTROL,
        );
        assert_single(
            "Ctrl-Shift-Enter",
            KeyCode::Enter,
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
    }
}
