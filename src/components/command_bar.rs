use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::app::command::Command;
use crate::theme::Theme;

const MAX_COMPLETION_ROWS: u16 = 8;

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

    pub fn matching_commands(&self) -> Vec<&'static Command> {
        let mut matches: Vec<_> = if self.input.is_empty() {
            Command::typable_commands().collect()
        } else {
            Command::typable_commands()
                .filter(|c| c.name.starts_with(&self.input))
                .collect()
        };
        matches.sort_by(|a, b| a.name.len().cmp(&b.name.len()).then(a.name.cmp(b.name)));
        matches
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
        if let Some(cmd) = Command::typable_commands().find(|c| c.name == trimmed) {
            return Some(cmd);
        }
        let matches: Vec<_> = Command::typable_commands()
            .filter(|c| c.name.starts_with(trimmed))
            .collect();
        if matches.len() == 1 {
            return Some(matches[0]);
        }
        if trimmed == "q" {
            return Command::by_name("quit");
        }
        None
    }

    pub fn completion_height(&self) -> u16 {
        let matches = self.matching_commands();
        (matches.len() as u16).min(MAX_COMPLETION_ROWS)
    }

    pub fn draw_completions(&self, area: Rect, buf: &mut Buffer) {
        let matches = self.matching_commands();
        if matches.is_empty() {
            return;
        }

        Widget::render(Clear, area, buf);

        let block = Block::default()
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .border_style(Theme::border());
        let inner = block.inner(area);
        Widget::render(block, area, buf);

        let selected = self.completion_idx;
        let visible: Vec<_> = matches
            .iter()
            .take(MAX_COMPLETION_ROWS as usize)
            .enumerate()
            .collect();
        let lines: Vec<Line> = visible
            .iter()
            .rev()
            .map(|(i, cmd)| {
                let is_selected = selected == Some(*i);
                let name_style = if is_selected {
                    Theme::file_list_selected()
                } else {
                    Theme::review_bar_key()
                };
                let doc_style = if is_selected {
                    Theme::file_list_selected()
                } else {
                    Theme::help_desc()
                };
                Line::from(vec![
                    Span::styled(format!(" {:<20}", cmd.name), name_style),
                    Span::styled(cmd.doc, doc_style),
                ])
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        Widget::render(paragraph, inner, buf);
    }

    pub fn draw_input(&self, area: Rect, buf: &mut Buffer) {
        let matches = self.matching_commands();
        let hint = if matches.len() == 1 && !self.input.is_empty() {
            matches[0].name[self.input.len()..].to_string()
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

        if !self.input.is_empty() && matches.is_empty() {
            spans.push(Span::styled("  [unknown command]", Theme::error()));
        }

        let line = Line::from(spans);
        let bar = Paragraph::new(line).style(Theme::review_bar());
        Widget::render(bar, area, buf);
    }
}
