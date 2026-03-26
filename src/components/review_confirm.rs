use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};
use tui_textarea::{Input, TextArea};

use crate::theme::Theme;
use crate::types::ReviewEvent;

pub struct ReviewConfirm {
    pub visible: bool,
    pub event: ReviewEvent,
    pub pending_count: usize,
    pub textarea: TextArea<'static>,
}

impl ReviewConfirm {
    pub fn new() -> Self {
        let mut textarea = TextArea::default();
        textarea.set_cursor_line_style(Style::default());
        Self {
            visible: false,
            event: ReviewEvent::Comment,
            pending_count: 0,
            textarea,
        }
    }

    pub fn show(&mut self, event: ReviewEvent, pending_count: usize) {
        self.event = event;
        self.pending_count = pending_count;
        self.textarea = TextArea::default();
        self.textarea.set_cursor_line_style(Style::default());
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn body_text(&self) -> String {
        self.textarea.lines().join("\n").trim().to_string()
    }

    pub fn handle_input(&mut self, input: Input) {
        self.textarea.input(input);
    }

    pub fn draw(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }

        let width = 60u16.min(area.width.saturating_sub(4));
        let height = 14u16.min(area.height.saturating_sub(4));
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let popup_area = Rect::new(x, y, width, height);

        Widget::render(Clear, popup_area, buf);

        let title = " Submit Review ";
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Theme::border_focused());

        let inner = block.inner(popup_area);
        Widget::render(block, popup_area, buf);

        let action_style = match self.event {
            ReviewEvent::Approve => Theme::status_added(),
            ReviewEvent::RequestChanges => Theme::status_deleted(),
            ReviewEvent::Comment => Theme::status_modified(),
            ReviewEvent::Unapprove => Theme::status_deleted(),
        };

        let comments_line = if self.pending_count > 0 {
            format!(
                "  with {} inline comment{}",
                self.pending_count,
                if self.pending_count == 1 { "" } else { "s" }
            )
        } else {
            "  with no inline comments".to_string()
        };

        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(inner);

        let header_lines = vec![
            Line::from(vec![
                Span::styled("  Action: ", Theme::review_bar_label()),
                Span::styled(self.event.label().to_string(), action_style),
            ]),
            Line::from(vec![Span::styled(comments_line, Theme::review_bar_label())]),
        ];
        Widget::render(Paragraph::new(header_lines), chunks[0], buf);

        let body_label = Line::from(vec![Span::styled(
            "  Review body (optional):",
            Theme::review_bar_label(),
        )]);
        Widget::render(Paragraph::new(vec![body_label]), chunks[1], buf);

        let ta_area = Rect::new(
            chunks[2].x + 2,
            chunks[2].y,
            chunks[2].width.saturating_sub(4),
            chunks[2].height,
        );
        #[allow(deprecated)]
        self.textarea.widget().render(ta_area, buf);

        let help = Line::from(vec![
            Span::styled("  Ctrl+S", Theme::review_bar_key()),
            Span::styled(" confirm  ", Theme::review_bar_label()),
            Span::styled("Esc", Theme::review_bar_key()),
            Span::styled(" cancel", Theme::review_bar_label()),
        ]);
        Widget::render(Paragraph::new(vec![help]), chunks[3], buf);
    }
}
