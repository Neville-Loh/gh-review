use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::theme::Theme;
use crate::types::ReviewEvent;

pub struct ReviewConfirm {
    pub visible: bool,
    pub event: ReviewEvent,
    pub pending_count: usize,
}

impl ReviewConfirm {
    pub fn new() -> Self {
        Self {
            visible: false,
            event: ReviewEvent::Comment,
            pending_count: 0,
        }
    }

    pub fn show(&mut self, event: ReviewEvent, pending_count: usize) {
        self.event = event;
        self.pending_count = pending_count;
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn draw(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }

        let width = 50u16.min(area.width.saturating_sub(4));
        let height = 9u16.min(area.height.saturating_sub(4));
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

        let lines = vec![
            Line::default(),
            Line::from(vec![
                Span::styled("  Action: ", Theme::review_bar_label()),
                Span::styled(self.event.label().to_string(), action_style),
            ]),
            Line::from(vec![Span::styled(comments_line, Theme::review_bar_label())]),
            Line::default(),
            Line::from(vec![
                Span::styled("  Enter", Theme::review_bar_key()),
                Span::styled(" confirm  ", Theme::review_bar_label()),
                Span::styled("Esc", Theme::review_bar_key()),
                Span::styled(" cancel", Theme::review_bar_label()),
            ]),
        ];

        let paragraph = Paragraph::new(lines);
        Widget::render(paragraph, inner, buf);
    }
}
