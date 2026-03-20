use ratatui::style::{Color, Modifier, Style};

pub struct Theme;

impl Theme {
    pub fn added_line() -> Style {
        Style::default().fg(Color::Green)
    }

    pub fn added_line_bg() -> Style {
        Style::default()
            .fg(Color::Green)
            .bg(Color::Rgb(0, 40, 0))
    }

    pub fn removed_line() -> Style {
        Style::default().fg(Color::Red)
    }

    pub fn removed_line_bg() -> Style {
        Style::default()
            .fg(Color::Red)
            .bg(Color::Rgb(40, 0, 0))
    }

    pub fn context_line() -> Style {
        Style::default().fg(Color::DarkGray)
    }

    pub fn line_number() -> Style {
        Style::default().fg(Color::DarkGray)
    }

    pub fn hunk_header() -> Style {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::DIM)
    }

    pub fn file_header() -> Style {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    }

    pub fn selected_line() -> Style {
        Style::default().bg(Color::Rgb(40, 40, 60))
    }

    pub fn file_list_selected() -> Style {
        Style::default()
            .fg(Color::White)
            .bg(Color::Rgb(40, 40, 60))
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
        Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::DIM)
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
}
