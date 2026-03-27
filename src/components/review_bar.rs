use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::app::keymap::Keymap;
use crate::theme::Theme;
use crate::types::RowContext;

pub struct ReviewBar;

impl ReviewBar {
    pub fn draw(
        area: Rect,
        buf: &mut Buffer,
        context: RowContext,
        pending_count: usize,
        status_msg: &str,
        status_is_error: bool,
        keymap: &Keymap,
    ) {
        let mut spans = Self::context_hints(context, keymap);

        let help_key = keymap.key_label("help");
        spans.push(Span::styled(format!("[{help_key}]"), Theme::review_bar_key()));
        spans.push(Span::styled(" help ", Theme::review_bar_label()));

        if pending_count > 0 {
            spans.push(Span::styled("│ ", Theme::review_bar_label()));
            spans.push(Span::styled(
                format!("{pending_count} pending"),
                Theme::pending_count(),
            ));
        }

        if !status_msg.is_empty() {
            let style = if status_is_error {
                Theme::error()
            } else {
                Theme::status_added()
            };
            spans.push(Span::styled(" │ ", Theme::review_bar_label()));
            spans.push(Span::styled(status_msg.to_string(), style));
        }

        let line = Line::from(spans);
        let bar = Paragraph::new(line).style(Theme::review_bar());
        Widget::render(bar, area, buf);
    }

    fn context_hints(context: RowContext, keymap: &Keymap) -> Vec<Span<'static>> {
        let pairs = keymap.context_hint_pairs(context);
        let mut spans = Vec::new();
        for (desc, key_label) in pairs {
            spans.push(Span::styled(format!(" [{key_label}]"), Theme::review_bar_key()));
            spans.push(Span::styled(format!(" {desc} "), Theme::review_bar_label()));
        }
        spans
    }
}
