use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::app::Focus;
use crate::app::keymap::Keymap;
use crate::components::status_line::StatusLine;
use crate::theme::Theme;
use crate::types::RowContext;

pub struct ReviewBar;

impl ReviewBar {
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        area: Rect,
        buf: &mut Buffer,
        context: RowContext,
        pending_count: usize,
        status: &StatusLine,
        keymap: &Keymap,
        focus: Focus,
        has_stack: bool,
        ai_available: bool,
    ) {
        let has_pending = pending_count > 0;
        let pairs = keymap.bar_hints(focus, context, has_stack, has_pending, ai_available);
        let mut spans: Vec<Span<'static>> = Vec::new();
        for (hint, key_label) in pairs {
            spans.push(Span::styled(
                format!(" [{key_label}]"),
                Theme::review_bar_key(),
            ));
            spans.push(Span::styled(format!(" {hint} "), Theme::review_bar_label()));
        }

        let help_key = keymap.key_label("help");
        spans.push(Span::styled(
            format!("[{help_key}]"),
            Theme::review_bar_key(),
        ));
        spans.push(Span::styled(" help ", Theme::review_bar_label()));

        if pending_count > 0 {
            spans.push(Span::styled("│ ", Theme::review_bar_label()));
            spans.push(Span::styled(
                format!("{pending_count} pending"),
                Theme::pending_count(),
            ));
        }

        if !status.is_empty() {
            let style = if status.is_error() {
                Theme::error()
            } else {
                Theme::status_added()
            };
            spans.push(Span::styled(" │ ", Theme::review_bar_label()));
            spans.push(Span::styled(status.text().to_string(), style));
        }

        let line = Line::from(spans);
        let bar = Paragraph::new(line).style(Theme::review_bar());
        Widget::render(bar, area, buf);
    }
}
