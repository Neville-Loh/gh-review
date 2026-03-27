mod keys;
mod runtime;

pub use keys::{KeyBinding, format_key_binding, parse_key_string};
pub use runtime::Config;

use serde::Deserialize;
use std::collections::HashMap;

/// Top-level user config deserialized from `config.toml`.
///
/// All sections are optional. An empty file or missing file
/// results in all defaults.
#[derive(Deserialize, Default)]
pub struct UserConfig {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub keys: HashMap<String, KeyOrKeys>,
    #[serde(default)]
    pub aliases: HashMap<String, String>,
    #[serde(default)]
    pub disabled_commands: Vec<String>,
    #[serde(default)]
    pub actions: Vec<RawAction>,
}

/// General (non-keybinding) settings.
#[derive(Deserialize, Default)]
pub struct GeneralConfig {
    pub smooth_scroll: Option<bool>,
}

/// A single key string or an array of key strings.
///
/// ```toml
/// quit = "q"                     # Single
/// scroll_down = ["j", "down"]    # Multiple
/// ```
#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum KeyOrKeys {
    Single(String),
    Multiple(Vec<String>),
}

impl KeyOrKeys {
    pub fn to_vec(&self) -> Vec<String> {
        match self {
            KeyOrKeys::Single(s) => vec![s.clone()],
            KeyOrKeys::Multiple(v) => v.clone(),
        }
    }
}

/// A raw custom action as deserialized from TOML `[[actions]]`.
#[derive(Deserialize, Clone, Debug)]
pub struct RawAction {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub key: String,
    pub command: String,
    #[serde(default)]
    pub description: String,
}

/// Load the user config from the standard config path.
///
/// Returns `UserConfig::default()` if the file is missing or unparseable.
pub fn load_user_config() -> UserConfig {
    let path = crate::dirs::config_dir().join("config.toml");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return UserConfig::default(),
    };
    match toml::from_str(&content) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("warning: failed to parse {}: {e}", path.display());
            UserConfig::default()
        }
    }
}
