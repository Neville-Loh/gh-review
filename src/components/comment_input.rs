use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Clear, Widget},
};
use tui_textarea::{Input, Key, TextArea};

use crate::theme::Theme;

pub struct CommentInput {
    pub textarea: TextArea<'static>,
    pub visible: bool,
    pub file_path: String,
    pub line: usize,
    pub side: crate::types::Side,
    pub reply_to_id: Option<u64>,
    pub reply_author: String,
}

pub enum CommentAction {
    None,
    Submit(String),
    Cancel,
}

impl CommentInput {
    pub fn new() -> Self {
        let mut textarea = TextArea::default();
        textarea.set_cursor_line_style(Style::default());
        Self {
            textarea,
            visible: false,
            file_path: String::new(),
            line: 0,
            side: crate::types::Side::Right,
            reply_to_id: None,
            reply_author: String::new(),
        }
    }

    pub fn open(&mut self, file_path: String, line: usize, side: crate::types::Side) {
        self.textarea = TextArea::default();
        self.textarea.set_cursor_line_style(Style::default());
        self.file_path = file_path;
        self.line = line;
        self.side = side;
        self.reply_to_id = None;
        self.reply_author.clear();
        self.visible = true;
    }

    pub fn open_reply(&mut self, comment_id: u64, author: String) {
        self.textarea = TextArea::default();
        self.textarea.set_cursor_line_style(Style::default());
        self.reply_to_id = Some(comment_id);
        self.reply_author = author;
        self.visible = true;
    }

    pub fn handle_input(&mut self, input: Input) -> CommentAction {
        match input {
            Input { key: Key::Esc, .. } => {
                self.visible = false;
                CommentAction::Cancel
            }
            Input {
                key: Key::Char('s'),
                ctrl: true,
                ..
            } => {
                let text = self.textarea.lines().join("\n").trim().to_string();
                if text.is_empty() {
                    self.visible = false;
                    return CommentAction::Cancel;
                }
                self.visible = false;
                CommentAction::Submit(text)
            }
            input => {
                self.textarea.input(input);
                CommentAction::None
            }
        }
    }

    pub fn draw(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }

        // Center the comment box in the available area
        let width = (area.width * 2 / 3).max(40).min(area.width);
        let height = 8u16.min(area.height);
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let popup_area = Rect::new(x, y, width, height);

        // Clear the background
        Widget::render(Clear, popup_area, buf);

        let title = if self.reply_to_id.is_some() {
            format!(" Reply to @{} ", self.reply_author)
        } else {
            format!(" Comment on {}:{} ", self.file_path, self.line)
        };
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Theme::comment_marker());

        let inner = block.inner(popup_area);
        Widget::render(block, popup_area, buf);
        #[allow(deprecated)]
        self.textarea.widget().render(inner, buf);

        // Help text at the bottom of the popup
        let help_y = popup_area.y + popup_area.height.saturating_sub(1);
        if help_y < area.y + area.height {
            let help = " Ctrl+S save │ Esc cancel ";
            let help_x = popup_area.x + 1;
            buf.set_string(help_x, help_y, help, Theme::help_desc());
        }
    }
}
