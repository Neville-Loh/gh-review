use ansi_to_tui::IntoText;
use arborium::theme::builtin;
use arborium::{AnsiHighlighter, detect_language};
use ratatui::text::{Line, Text};
use std::sync::{LazyLock, Mutex};

use crate::types::DiffLine;

static HIGHLIGHTER: LazyLock<Mutex<AnsiHighlighter>> =
    LazyLock::new(|| Mutex::new(AnsiHighlighter::new(builtin::github_dark())));

pub fn highlight(line: &DiffLine, path: &str) -> Line<'static> {
    let lang = detect_language(path).unwrap_or("text");
    let content = &line.content;

    let mut highlighter = HIGHLIGHTER.lock().unwrap();
    let highlighted_ansi = highlighter
        .highlight(lang, content)
        .unwrap_or_else(|_| content.to_string());

    let tui_text = highlighted_ansi
        .as_bytes()
        .into_text()
        .unwrap_or_else(|_| Text::raw(content.to_string()));

    tui_text.lines.into_iter().next().unwrap_or_default()
}
