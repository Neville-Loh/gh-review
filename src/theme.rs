use std::{process::Command, sync::LazyLock};

use ansi_to_tui::IntoText;
use ratatui::style::{Color, Modifier, Style};

static ADDED_LINE: LazyLock<Style> =
    LazyLock::new(|| git_diff_style("color.diff.new", Style::default().fg(Color::Green)));
static REMOVED_LINE: LazyLock<Style> =
    LazyLock::new(|| git_diff_style("color.diff.old", Style::default().fg(Color::Red)));

fn git_diff_style(name: &str, fallback: Style) -> Style {
    let Ok(output) = Command::new("git")
        .args(["config", "--get-color", name])
        .output()
    else {
        return fallback;
    };

    if !output.status.success() {
        return fallback;
    }

    style_from_ansi(&output.stdout)
        .map(|style| fallback.patch(style))
        .unwrap_or(fallback)
}

fn style_from_ansi(ansi: &[u8]) -> Option<Style> {
    let mut styled_space = ansi.to_vec();
    styled_space.push(b' ');
    styled_space
        .as_slice()
        .into_text()
        .ok()?
        .lines
        .first()?
        .spans
        .first()
        .map(|span| span.style)
}

pub struct Theme;

#[allow(dead_code)]
impl Theme {
    pub fn added_line_bg_color() -> Color {
        ADDED_LINE.bg.unwrap_or(Color::Reset)
    }

    pub fn removed_line_bg_color() -> Color {
        REMOVED_LINE.bg.unwrap_or(Color::Reset)
    }

    pub fn added_line() -> Style {
        Style::default().fg(ADDED_LINE.fg.unwrap_or(Color::Green))
    }

    pub fn added_line_bg() -> Style {
        *ADDED_LINE
    }

    pub fn removed_line() -> Style {
        Style::default().fg(REMOVED_LINE.fg.unwrap_or(Color::Red))
    }

    pub fn removed_line_bg() -> Style {
        *REMOVED_LINE
    }

    pub fn context_line() -> Style {
        Style::default().fg(Color::DarkGray)
    }

    pub fn line_number() -> Style {
        Style::default().fg(Color::DarkGray)
    }

    pub fn hunk_header() -> Style {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM)
    }

    pub fn file_header() -> Style {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    }

    pub fn selected_line() -> Style {
        Style::default()
            .bg(Color::Rgb(50, 50, 80))
            .add_modifier(Modifier::BOLD)
    }

    pub fn selected_cursor() -> Style {
        Style::default()
            .fg(Color::Rgb(100, 140, 255))
            .bg(Color::Rgb(50, 50, 80))
            .add_modifier(Modifier::BOLD)
    }

    pub fn file_list_selected() -> Style {
        Style::default()
            .fg(Color::White)
            .bg(Color::Rgb(50, 50, 80))
            .add_modifier(Modifier::BOLD)
    }

    pub fn file_list_normal() -> Style {
        Style::default().fg(Color::White)
    }

    pub fn status_added() -> Style {
        Style::default().fg(Color::Green)
    }

    pub fn status_deleted() -> Style {
        Style::default().fg(Color::Red)
    }

    pub fn status_modified() -> Style {
        Style::default().fg(Color::Yellow)
    }

    pub fn comment_marker() -> Style {
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD)
    }

    pub fn comment_body() -> Style {
        Style::default().fg(Color::Magenta)
    }

    pub fn review_bar() -> Style {
        Style::default().bg(Color::Rgb(30, 30, 30))
    }

    pub fn review_bar_key() -> Style {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    }

    pub fn review_bar_label() -> Style {
        Style::default().fg(Color::White)
    }

    pub fn pending_count() -> Style {
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD)
    }

    pub fn border() -> Style {
        Style::default().fg(Color::DarkGray)
    }

    pub fn border_focused() -> Style {
        Style::default().fg(Color::Blue)
    }

    pub fn title() -> Style {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    }

    pub fn expand_hint() -> Style {
        Style::default().fg(Color::Blue).add_modifier(Modifier::DIM)
    }

    pub fn help_key() -> Style {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    }

    pub fn help_desc() -> Style {
        Style::default().fg(Color::Gray)
    }

    pub fn error() -> Style {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    }

    pub fn search_match() -> Style {
        Style::default().bg(Color::Rgb(100, 80, 0)).fg(Color::White)
    }

    pub fn search_current() -> Style {
        Style::default()
            .bg(Color::Rgb(200, 150, 0))
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    }

    pub fn search_prompt() -> Style {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    }

    pub fn search_count() -> Style {
        Style::default().fg(Color::DarkGray)
    }

    pub fn resolved_comment() -> Style {
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM)
    }

    // Comment thread colors — light tinted backgrounds for clear card visibility.
    pub fn comment_bg() -> Color {
        Color::Rgb(30, 35, 55)
    }

    pub fn comment_accent() -> Color {
        Color::Blue
    }

    pub fn comment_fg() -> Color {
        Color::Rgb(180, 200, 240)
    }

    pub fn resolved_bg() -> Color {
        Color::Rgb(28, 40, 32)
    }

    pub fn resolved_accent() -> Color {
        Color::Green
    }

    pub fn resolved_fg() -> Color {
        Color::Rgb(140, 180, 150)
    }

    pub fn pending_bg() -> Color {
        Color::Rgb(45, 38, 25)
    }

    pub fn pending_accent() -> Color {
        Color::Yellow
    }

    pub fn pending_fg() -> Color {
        Color::Rgb(230, 200, 130)
    }

    pub fn suggestion_added() -> Style {
        Style::default().fg(Color::Green).bg(Color::Rgb(0, 30, 0))
    }

    pub fn suggestion_added_bg() -> Color {
        Color::Rgb(0, 30, 0)
    }

    pub fn suggestion_added_highlight_bg() -> Color {
        Color::Rgb(0, 100, 0)
    }

    pub fn suggestion_removed() -> Style {
        Style::default().fg(Color::Red).bg(Color::Rgb(30, 0, 0))
    }

    pub fn suggestion_removed_bg() -> Color {
        Color::Rgb(30, 0, 0)
    }

    pub fn suggestion_removed_highlight_bg() -> Color {
        Color::Rgb(100, 0, 0)
    }

    pub fn visual_select() -> Style {
        Style::default().bg(Color::Rgb(60, 60, 30))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_git_truecolor_style() {
        let style = style_from_ansi(b"\x1b[38;2;76;79;105;48;2;204;225;205m").unwrap();

        assert_eq!(style.fg, Some(Color::Rgb(76, 79, 105)));
        assert_eq!(style.bg, Some(Color::Rgb(204, 225, 205)));
    }
}
