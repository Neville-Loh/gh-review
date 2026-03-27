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

#[allow(dead_code)]
pub fn config_dir() -> PathBuf {
    project_dirs()
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| {
            dirs_fallback().join(".config").join(APP)
        })
}

#[allow(dead_code)]
fn dirs_fallback() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir())
}
