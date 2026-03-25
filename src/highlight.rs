use ansi_to_tui::IntoText;
use arborium::theme::builtin;
use arborium::{AnsiHighlighter, detect_language};
use ratatui::text::{Line, Text};
use std::sync::{LazyLock, Mutex};

use crate::types::DiffFile;

static HIGHLIGHTER: LazyLock<Mutex<AnsiHighlighter>> =
    LazyLock::new(|| Mutex::new(AnsiHighlighter::new(builtin::github_dark())));

pub fn highlight_content(path: &str, content: &str) -> Vec<Line<'static>> {
    let lang = detect_language(path).unwrap_or("text");
    let mut highlighter = HIGHLIGHTER.lock().unwrap_or_else(|e| e.into_inner());
    let ansi = highlighter
        .highlight(lang, content)
        .unwrap_or_else(|_| content.to_string());
    ansi.as_bytes()
        .into_text()
        .unwrap_or_else(|_| Text::raw(content.to_string()))
        .lines
}

pub fn highlight_file(file: &mut DiffFile) {
    let lang = detect_language(&file.path).unwrap_or("text");
    let mut highlighter = HIGHLIGHTER.lock().unwrap_or_else(|e| e.into_inner());

    for hunk in &mut file.hunks {
        let content = hunk
            .lines
            .iter()
            .map(|line| line.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let ansi = highlighter
            .highlight(lang, &content)
            .unwrap_or_else(|_| content.clone());

        let highlighted_lines: Vec<Line<'static>> = ansi
            .as_bytes()
            .into_text()
            .unwrap_or_else(|_| Text::raw(content))
            .lines;

        for (line, highlighted) in hunk.lines.iter_mut().zip(highlighted_lines) {
            line.highlighted_content = Some(highlighted);
        }
    }
}
