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
    pub editing_pending_idx: Option<usize>,
    pub is_suggestion: bool,
    pub start_line: Option<usize>,
    pub start_side: Option<crate::types::Side>,
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
            editing_pending_idx: None,
            is_suggestion: false,
            start_line: None,
            start_side: None,
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
        self.editing_pending_idx = None;
        self.is_suggestion = false;
        self.start_line = None;
        self.start_side = None;
        self.visible = true;
    }

    pub fn open_reply(&mut self, comment_id: u64, author: String) {
        self.textarea = TextArea::default();
        self.textarea.set_cursor_line_style(Style::default());
        self.reply_to_id = Some(comment_id);
        self.reply_author = author;
        self.editing_pending_idx = None;
        self.is_suggestion = false;
        self.start_line = None;
        self.start_side = None;
        self.visible = true;
    }

    pub fn open_range(
        &mut self,
        file_path: String,
        start_line: usize,
        start_side: crate::types::Side,
        end_line: usize,
        end_side: crate::types::Side,
    ) {
        self.textarea = TextArea::default();
        self.textarea.set_cursor_line_style(Style::default());
        self.file_path = file_path;
        self.line = end_line;
        self.side = end_side;
        self.reply_to_id = None;
        self.reply_author.clear();
        self.editing_pending_idx = None;
        self.is_suggestion = false;
        self.start_line = Some(start_line);
        self.start_side = Some(start_side);
        self.visible = true;
    }

    pub fn open_edit(
        &mut self,
        pending_idx: usize,
        file_path: String,
        line: usize,
        side: crate::types::Side,
        body: &str,
    ) {
        let lines: Vec<String> = body.lines().map(String::from).collect();
        let lines = if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        };
        self.textarea = TextArea::new(lines);
        self.textarea.set_cursor_line_style(Style::default());
        self.file_path = file_path;
        self.line = line;
        self.side = side;
        self.reply_to_id = None;
        self.reply_author.clear();
        self.editing_pending_idx = Some(pending_idx);
        self.is_suggestion = false;
        self.start_line = None;
        self.start_side = None;
        self.visible = true;
    }

    fn submit(&mut self) -> CommentAction {
        let text = self.textarea.lines().join("\n").trim().to_string();
        if text.is_empty() {
            self.visible = false;
            return CommentAction::Cancel;
        }
        self.visible = false;
        CommentAction::Submit(text)
    }

    pub fn handle_input(&mut self, input: Input) -> CommentAction {
        match input {
            Input { key: Key::Esc, .. } => {
                self.visible = false;
                CommentAction::Cancel
            }
            Input {
                key: Key::Enter,
                ctrl: true,
                ..
            } => {
                self.textarea.insert_newline();
                CommentAction::None
            }
            Input {
                key: Key::Enter, ..
            } => self.submit(),
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
        } else if self.editing_pending_idx.is_some() {
            format!(" Edit comment on {}:{} ", self.file_path, self.line)
        } else if self.is_suggestion {
            format!(" Suggest change on {}:{} ", self.file_path, self.line)
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
            let help = " Enter save │ Ctrl+Enter newline │ Esc cancel ";
            let help_x = popup_area.x + 1;
            buf.set_string(help_x, help_y, help, Theme::help_desc());
        }
    }
}
