use ansi_to_tui::IntoText;
use arborium::theme::builtin;
use arborium::{AnsiHighlighter, detect_language};
use ratatui::{
    style::Color,
    text::{Line, Span, Text},
};
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
    terminal_foreground(
        ansi.as_bytes()
            .into_text()
            .unwrap_or_else(|_| Text::raw(content.to_string()))
            .lines,
    )
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

        let highlighted_lines = terminal_foreground(
            ansi.as_bytes()
                .into_text()
                .unwrap_or_else(|_| Text::raw(content))
                .lines,
        );

        for (line, highlighted) in hunk.lines.iter_mut().zip(highlighted_lines) {
            line.highlighted_content = Some(highlighted);
        }
    }
}

fn terminal_foreground(lines: Vec<Line<'static>>) -> Vec<Line<'static>> {
    lines
        .into_iter()
        .map(|line| {
            Line::from(
                line.spans
                    .into_iter()
                    .map(|span| Span::styled(span.content, span.style.fg(Color::Reset)))
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syntax_highlighting_uses_terminal_foreground() {
        let lines = highlight_content("main.rs", "let answer = 42;");

        assert!(
            lines
                .iter()
                .flat_map(|line| &line.spans)
                .all(|span| span.style.fg == Some(Color::Reset))
        );
    }
}
