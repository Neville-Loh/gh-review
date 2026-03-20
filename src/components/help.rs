use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::theme::Theme;

const BINDINGS: &[(&str, &str)] = &[
    ("j/k, ↑/↓", "Scroll line"),
    ("gg / G", "Go to first / last line"),
    ("Ctrl+D / U", "Half page down / up"),
    ("Ctrl+F / B", "Full page down / up"),
    ("H / M / L", "Screen top / middle / bottom"),
    ("] / }", "Next hunk"),
    ("[ / {", "Previous hunk"),
    (") / (", "Next / previous change"),
    ("n / N", "Next / previous file"),
    ("zz / zt / zb", "Center / top / bottom cursor"),
    ("Tab", "Switch focus: files ↔ diff"),
    ("t", "Toggle unified / side-by-side"),
    ("e", "Expand context (+10 lines)"),
    ("", ""),
    ("c", "Comment on current line"),
    ("Ctrl+S", "Save comment"),
    ("Esc", "Cancel / close overlay"),
    ("a", "Approve PR"),
    ("r", "Request changes"),
    ("s", "Submit review (comment only)"),
    ("o", "Open in browser"),
    ("?", "This help"),
    ("q", "Quit"),
];

pub struct HelpOverlay;

impl HelpOverlay {
    pub fn draw(area: Rect, buf: &mut Buffer) {
        let width = 55u16.min(area.width.saturating_sub(4));
        let height = (BINDINGS.len() as u16 + 3).min(area.height.saturating_sub(4));
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let popup_area = Rect::new(x, y, width, height);

        Widget::render(Clear, popup_area, buf);

        let block = Block::default()
            .title(" Keybindings ")
            .borders(Borders::ALL)
            .border_style(Theme::border_focused());

        let inner = block.inner(popup_area);
        Widget::render(block, popup_area, buf);

        let lines: Vec<Line> = BINDINGS
            .iter()
            .map(|(key, desc)| {
                Line::from(vec![
                    Span::styled(format!("{key:>14}"), Theme::help_key()),
                    Span::styled("  ", Theme::help_desc()),
                    Span::styled(desc.to_string(), Theme::help_desc()),
                ])
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        Widget::render(paragraph, inner, buf);
    }
}
