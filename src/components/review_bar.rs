use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::app::keymap::Keymap;
use crate::app::Focus;
use crate::components::description_panel::CursorRegion;
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
        desc_region: CursorRegion,
    ) {
        let mut spans = if focus == Focus::Description {
            Self::description_hints(desc_region, keymap)
        } else {
            Self::context_hints(context, keymap)
        };

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

    fn context_hints(context: RowContext, keymap: &Keymap) -> Vec<Span<'static>> {
        let pairs = keymap.context_hint_pairs(context);
        let mut spans = Vec::new();
        for (desc, key_label) in pairs {
            spans.push(Span::styled(format!(" [{key_label}]"), Theme::review_bar_key()));
            spans.push(Span::styled(format!(" {desc} "), Theme::review_bar_label()));
        }
        spans
    }

    fn description_hints(region: CursorRegion, keymap: &Keymap) -> Vec<Span<'static>> {
        let region_name = match region {
            CursorRegion::Title => "title",
            CursorRegion::Body => "body",
        };
        let edit_key = keymap.key_label("edit_description");
        vec![
            Span::styled(format!(" [{edit_key}]"), Theme::review_bar_key()),
            Span::styled(format!(" edit {region_name} "), Theme::review_bar_label()),
            Span::styled("[Esc]", Theme::review_bar_key()),
            Span::styled(" close ", Theme::review_bar_label()),
        ]
    }
}
