use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::search::SearchDirection;
use crate::theme::Theme;

pub struct SearchBar {
    pub active: bool,
    pub input: String,
    pub direction: SearchDirection,
}

impl SearchBar {
    pub fn new() -> Self {
        Self {
            active: false,
            input: String::new(),
            direction: SearchDirection::Forward,
        }
    }

    pub fn open(&mut self, direction: SearchDirection) {
        self.active = true;
        self.input.clear();
        self.direction = direction;
    }

    pub fn close(&mut self) {
        self.active = false;
    }

    pub fn push_char(&mut self, c: char) {
        self.input.push(c);
    }

    pub fn pop_char(&mut self) {
        self.input.pop();
    }

    pub fn draw(&self, area: Rect, buf: &mut Buffer, match_current: usize, match_total: usize) {
        let prefix = match self.direction {
            SearchDirection::Forward => "/",
            SearchDirection::Backward => "?",
        };

        let mut spans = vec![
            Span::styled(prefix, Theme::search_prompt()),
            Span::styled(self.input.clone(), Theme::search_prompt()),
            Span::styled("█", Theme::search_prompt()),
        ];

        if !self.input.is_empty() {
            if match_total > 0 {
                spans.push(Span::styled(
                    format!("  [{}/{}]", match_current + 1, match_total),
                    Theme::search_count(),
                ));
            } else {
                spans.push(Span::styled("  [no matches]", Theme::error()));
            }
        }

        let line = Line::from(spans);
        let bar = Paragraph::new(line).style(Theme::review_bar());
        Widget::render(bar, area, buf);
    }
}
