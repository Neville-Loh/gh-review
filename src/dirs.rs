use std::path::PathBuf;

use directories::ProjectDirs;

const QUALIFIER: &str = "";
const ORG: &str = "";
const APP: &str = "gh-review";

fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from(QUALIFIER, ORG, APP)
}

/// Scratch files: editor temp files, safe to delete.
/// `~/.cache/gh-review/` on Linux, `~/Library/Caches/gh-review/` on macOS.
pub fn cache_dir() -> PathBuf {
    project_dirs()
        .map(|d| d.cache_dir().to_path_buf())
        .unwrap_or_else(|| std::env::temp_dir().join(APP))
}

#[allow(dead_code)]
pub fn state_dir() -> PathBuf {
    project_dirs()
        .map(|d| {
            d.state_dir()
                .unwrap_or_else(|| d.data_local_dir())
                .to_path_buf()
        })
        .unwrap_or_else(|| std::env::temp_dir().join(APP))
}

pub fn config_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join(APP);
    }
    if cfg!(windows)
        && let Ok(appdata) = std::env::var("APPDATA")
    {
        return PathBuf::from(appdata).join(APP);
    }
    dirs_fallback().join(".config").join(APP)
}

fn dirs_fallback() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir())
}
