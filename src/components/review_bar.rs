use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::types::RowContext;
use crate::theme::Theme;

pub struct ReviewBar;

impl ReviewBar {
    pub fn draw(
        area: Rect,
        buf: &mut Buffer,
        context: RowContext,
        pending_count: usize,
        status_msg: &str,
        status_is_error: bool,
    ) {
        let mut spans = Self::context_hints(context);

        spans.push(Span::styled("[!]", Theme::review_bar_key()));
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

    fn context_hints(context: RowContext) -> Vec<Span<'static>> {
        match context {
            RowContext::File => vec![
                Span::styled(" [Enter]", Theme::review_bar_key()),
                Span::styled(" fold ", Theme::review_bar_label()),
                Span::styled("[a]", Theme::review_bar_key()),
                Span::styled("pprove ", Theme::review_bar_label()),
                Span::styled("[s]", Theme::review_bar_key()),
                Span::styled("ubmit ", Theme::review_bar_label()),
            ],
            RowContext::Code => vec![
                Span::styled(" [c]", Theme::review_bar_key()),
                Span::styled("omment ", Theme::review_bar_label()),
                Span::styled("[e]", Theme::review_bar_key()),
                Span::styled(" suggest ", Theme::review_bar_label()),
                Span::styled("[v]", Theme::review_bar_key()),
                Span::styled("isual ", Theme::review_bar_label()),
                Span::styled("[a]", Theme::review_bar_key()),
                Span::styled("pprove ", Theme::review_bar_label()),
                Span::styled("[s]", Theme::review_bar_key()),
                Span::styled("ubmit ", Theme::review_bar_label()),
            ],
            RowContext::Comment => vec![
                Span::styled(" [c]", Theme::review_bar_key()),
                Span::styled(" reply ", Theme::review_bar_label()),
                Span::styled("[r]", Theme::review_bar_key()),
                Span::styled("esolve ", Theme::review_bar_label()),
                Span::styled("[x]", Theme::review_bar_key()),
                Span::styled(" discard ", Theme::review_bar_label()),
                Span::styled("[Enter]", Theme::review_bar_key()),
                Span::styled(" toggle ", Theme::review_bar_label()),
            ],
            RowContext::Suggestion => vec![
                Span::styled(" [y]", Theme::review_bar_key()),
                Span::styled(" accept ", Theme::review_bar_label()),
                Span::styled("[c]", Theme::review_bar_key()),
                Span::styled(" reply ", Theme::review_bar_label()),
                Span::styled("[r]", Theme::review_bar_key()),
                Span::styled("esolve ", Theme::review_bar_label()),
            ],
        }
    }
}
