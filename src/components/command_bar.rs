use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::app::command::Command;
use crate::theme::Theme;

pub struct CommandBar {
    pub active: bool,
    pub input: String,
    completion_idx: Option<usize>,
}

impl CommandBar {
    pub fn new() -> Self {
        Self {
            active: false,
            input: String::new(),
            completion_idx: None,
        }
    }

    pub fn open(&mut self) {
        self.active = true;
        self.input.clear();
        self.completion_idx = None;
    }

    pub fn close(&mut self) {
        self.active = false;
        self.input.clear();
        self.completion_idx = None;
    }

    pub fn push_char(&mut self, c: char) {
        self.input.push(c);
        self.completion_idx = None;
    }

    pub fn pop_char(&mut self) {
        self.input.pop();
        self.completion_idx = None;
    }

    fn matching_commands(&self) -> Vec<&'static Command> {
        if self.input.is_empty() {
            return Command::typable_commands().collect();
        }
        Command::typable_commands()
            .filter(|c| c.name.starts_with(&self.input))
            .collect()
    }

    pub fn cycle_completion(&mut self) {
        let matches = self.matching_commands();
        if matches.is_empty() {
            return;
        }
        let next = match self.completion_idx {
            Some(i) => (i + 1) % matches.len(),
            None => 0,
        };
        self.input = matches[next].name.to_string();
        self.completion_idx = Some(next);
    }

    pub fn resolve(&self) -> Option<&'static Command> {
        let trimmed = self.input.trim();
        if trimmed.is_empty() {
            return None;
        }
        // Exact match first
        if let Some(cmd) = Command::typable_commands().find(|c| c.name == trimmed) {
            return Some(cmd);
        }
        // Unique prefix match
        let matches: Vec<_> = Command::typable_commands()
            .filter(|c| c.name.starts_with(trimmed))
            .collect();
        if matches.len() == 1 {
            return Some(matches[0]);
        }
        // Alias: "q" -> "quit"
        if trimmed == "q" {
            return Command::by_name("quit");
        }
        None
    }

    pub fn draw(&self, area: Rect, buf: &mut Buffer) {
        let matches = self.matching_commands();
        let hint = if matches.len() == 1 && !self.input.is_empty() {
            let rest = &matches[0].name[self.input.len()..];
            rest.to_string()
        } else {
            String::new()
        };

        let mut spans = vec![
            Span::styled(":", Theme::search_prompt()),
            Span::styled(self.input.clone(), Theme::search_prompt()),
        ];

        if !hint.is_empty() {
            spans.push(Span::styled(hint, Theme::search_count()));
        }

        spans.push(Span::styled("█", Theme::search_prompt()));

        if !self.input.is_empty() {
            if matches.is_empty() {
                spans.push(Span::styled("  [unknown command]", Theme::error()));
            } else if matches.len() > 1 {
                let names: Vec<&str> = matches.iter().take(5).map(|c| c.name).collect();
                let hint_str = if matches.len() > 5 {
                    format!("  [{}... +{}]", names.join(", "), matches.len() - 5)
                } else {
                    format!("  [{}]", names.join(", "))
                };
                spans.push(Span::styled(hint_str, Theme::search_count()));
            }
        }

        let line = Line::from(spans);
        let bar = Paragraph::new(line).style(Theme::review_bar());
        Widget::render(bar, area, buf);
    }
}
