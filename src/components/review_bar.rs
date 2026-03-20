use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::theme::Theme;

pub struct ReviewBar;

impl ReviewBar {
    pub fn draw(area: Rect, buf: &mut Buffer, pending_count: usize, status_msg: &str, status_is_error: bool) {
        let mut spans = vec![
            Span::styled(" [c]", Theme::review_bar_key()),
            Span::styled("omment ", Theme::review_bar_label()),
            Span::styled(" [a]", Theme::review_bar_key()),
            Span::styled("pprove ", Theme::review_bar_label()),
            Span::styled(" [r]", Theme::review_bar_key()),
            Span::styled("equest changes ", Theme::review_bar_label()),
            Span::styled(" [s]", Theme::review_bar_key()),
            Span::styled("ubmit ", Theme::review_bar_label()),
            Span::styled(" [t]", Theme::review_bar_key()),
            Span::styled("oggle view ", Theme::review_bar_label()),
            Span::styled(" [?]", Theme::review_bar_key()),
            Span::styled(" help ", Theme::review_bar_label()),
        ];

        if pending_count > 0 {
            spans.push(Span::styled("│ ", Theme::review_bar_label()));
            spans.push(Span::styled(
                format!("{pending_count} pending"),
                Theme::pending_count(),
            ));
        }

        if !status_msg.is_empty() {
            let style = if status_is_error { Theme::error() } else { Theme::status_added() };
            spans.push(Span::styled(" │ ", Theme::review_bar_label()));
            spans.push(Span::styled(status_msg.to_string(), style));
        }

        let line = Line::from(spans);
        let bar = Paragraph::new(line).style(Theme::review_bar());
        Widget::render(bar, area, buf);
    }
}
