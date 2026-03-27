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
    ("n / N", "Next/prev match (or file)"),
    ("zz / zt / zb", "Center / top / bottom cursor"),
    ("Tab", "Switch focus: files ↔ diff"),
    ("t", "Toggle unified / side-by-side"),
    ("e", "Suggest change on current line"),
    ("E", "Expand context (+10 lines)"),
    ("", ""),
    ("/", "Search forward in diff"),
    ("?", "Search backward in diff"),
    ("Esc", "Clear search / cancel / quit"),
    ("", ""),
    ("c", "Comment (or edit pending)"),
    ("v", "Visual select (multi-line)"),
    ("x", "Discard pending comment"),
    ("r", "Resolve / unresolve thread"),
    ("y", "Accept suggestion"),
    ("Ctrl+S", "Save comment / submit review"),
    ("a", "Approve PR"),
    ("s", "Submit review (comment only)"),
    ("u", "Unapprove (dismiss approval)"),
    ("o", "Open in browser"),
    ("!", "This help"),
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
