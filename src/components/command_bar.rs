use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::app::command::Command;
use crate::app::keymap::Keymap;
use crate::theme::Theme;

const MAX_COMPLETION_ROWS: u16 = 8;

/// An entry in the command bar completion list (either built-in or custom).
pub struct CompletionEntry {
    pub name: String,
    pub doc: String,
}

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

    pub fn matching_commands(&self, keymap: &Keymap) -> Vec<CompletionEntry> {
        let mut entries: Vec<CompletionEntry> = Vec::new();

        for cmd in Command::typable_commands() {
            if keymap.is_disabled(cmd.name) {
                continue;
            }
            if self.input.is_empty() || cmd.name.starts_with(&self.input) {
                entries.push(CompletionEntry {
                    name: cmd.name.to_string(),
                    doc: cmd.doc.to_string(),
                });
            }
        }

        for (alias, target) in keymap.alias_entries() {
            if self.input.is_empty() || alias.starts_with(&self.input) {
                let doc = Command::by_name(target)
                    .map(|c| c.doc.to_string())
                    .unwrap_or_default();
                entries.push(CompletionEntry {
                    name: alias.clone(),
                    doc,
                });
            }
        }

        for action in keymap.named_custom_actions() {
            if self.input.is_empty() || action.name.starts_with(&self.input) {
                entries.push(CompletionEntry {
                    name: action.name.clone(),
                    doc: action.description.clone(),
                });
            }
        }

        entries.sort_by(|a, b| a.name.len().cmp(&b.name.len()).then(a.name.cmp(&b.name)));
        entries
    }

    pub fn cycle_completion(&mut self, keymap: &Keymap) {
        let matches = self.matching_commands(keymap);
        if matches.is_empty() {
            return;
        }
        let next = match self.completion_idx {
            Some(i) => (i + 1) % matches.len(),
            None => 0,
        };
        self.input = matches[next].name.clone();
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

    pub fn completion_height(&self, keymap: &Keymap) -> u16 {
        let matches = self.matching_commands(keymap);
        (matches.len() as u16).min(MAX_COMPLETION_ROWS)
    }

    pub fn draw_completions(&self, area: Rect, buf: &mut Buffer, keymap: &Keymap) {
        let matches = self.matching_commands(keymap);
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
            .map(|(i, entry)| {
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
                    Span::styled(format!(" {:<20}", entry.name), name_style),
                    Span::styled(&entry.doc, doc_style),
                ])
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        Widget::render(paragraph, inner, buf);
    }

    pub fn draw_input(&self, area: Rect, buf: &mut Buffer, keymap: &Keymap) {
        let matches = self.matching_commands(keymap);
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
