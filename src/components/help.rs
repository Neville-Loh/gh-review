use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Widget},
};

use crate::app::keymap::Keymap;
use crate::theme::Theme;

pub struct HelpOverlay;

impl HelpOverlay {
    pub fn draw(area: Rect, buf: &mut Buffer, keymap: &Keymap) {
        let bindings = keymap.help_bindings();

        let width = 60u16.min(area.width.saturating_sub(4));
        let height = (bindings.len() as u16 + 5).min(area.height.saturating_sub(4));
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let popup_area = Rect::new(x, y, width, height);

        Widget::render(Clear, popup_area, buf);

        let block = Block::default()
            .title(" Keybindings ")
            .borders(Borders::ALL)
            .border_style(Theme::border_focused())
            .padding(Padding::new(2, 2, 1, 1));

        let inner = block.inner(popup_area);
        Widget::render(block, popup_area, buf);

        let lines: Vec<Line> = bindings
            .iter()
            .map(|(key, desc): &(String, &str)| {
                Line::from(vec![
                    Span::styled(format!("{key:>16}"), Theme::help_key()),
                    Span::styled("  ", Theme::help_desc()),
                    Span::styled(desc.to_string(), Theme::help_desc()),
                ])
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        Widget::render(paragraph, inner, buf);
    }
}
