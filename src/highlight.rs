use ansi_to_tui::IntoText;
use arborium::theme::builtin;
use arborium::{detect_language, AnsiHighlighter};
use ratatui::text::{Line, Text};
use std::sync::{LazyLock, Mutex};

use crate::types::DiffFile;

static HIGHLIGHTER: LazyLock<Mutex<AnsiHighlighter>> =
    LazyLock::new(|| Mutex::new(AnsiHighlighter::new(builtin::github_dark())));

pub fn highlight_file(file: &mut DiffFile) {
    let lang = detect_language(&file.path).unwrap_or("text");

    let content: String = file
        .hunks
        .iter()
        .flat_map(|hunk| &hunk.lines)
        .map(|line| line.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let mut highlighter = HIGHLIGHTER.lock().unwrap();
    let highlighted_ansi = highlighter
        .highlight(lang, &content)
        .unwrap_or_else(|_| content.clone());

    let tui_text = highlighted_ansi
        .as_bytes()
        .into_text()
        .unwrap_or_else(|_| Text::raw(content));

    let highlighted_lines: Vec<Option<Line<'static>>> =
        tui_text.lines.into_iter().map(Some).collect();

    let mut idx = 0;
    for hunk in &mut file.hunks {
        for line in &mut hunk.lines {
            line.highlighted_content = highlighted_lines.get(idx).cloned().flatten();
            idx += 1;
        }
    }
}
