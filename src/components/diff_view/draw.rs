use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::diff::renderer::{DisplayRow, render_sbs_row, render_unified_row};
use crate::theme::Theme;
use crate::types::DiffMode;

use super::DiffView;

impl DiffView {
    pub fn draw(&self, area: Rect, buf: &mut Buffer, focused: bool) {
        let border_style = if focused {
            Theme::border_focused()
        } else {
            Theme::border()
        };

        let mode_label = match self.mode {
            DiffMode::Unified => "unified",
            DiffMode::SideBySide => "side-by-side",
        };

        let title = format!(" Diff ({mode_label}) ");
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(area);
        Widget::render(block, area, buf);

        for y in inner.y..inner.y + inner.height {
            for x in inner.x..inner.x + inner.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.reset();
                }
            }
        }

        let visible_height = inner.height as usize;
        let scroll = self.effective_scroll(visible_height);

        match self.mode {
            DiffMode::Unified => self.draw_unified(inner, buf, scroll, visible_height),
            DiffMode::SideBySide => self.draw_sbs(inner, buf, scroll, visible_height),
        }
    }

    fn effective_scroll(&self, visible_height: usize) -> usize {
        let mut s = self.scroll_offset;
        if self.cursor < s {
            s = self.cursor;
        } else if self.cursor >= s + visible_height {
            s = self.cursor - visible_height + 1;
        }
        s
    }

    fn draw_unified(&self, area: Rect, buf: &mut Buffer, scroll: usize, visible_height: usize) {
        let end = (scroll + visible_height).min(self.display_rows.len());
        let visible = &self.display_rows[scroll..end];

        let lines: Vec<Line> = visible
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let global_idx = scroll + i;
                let selected = global_idx == self.cursor;
                let line = render_unified_row(row, &self.files, area.width, selected);
                self.search.highlight(line, global_idx)
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        Widget::render(paragraph, area, buf);
    }

    fn draw_sbs(&self, area: Rect, buf: &mut Buffer, scroll: usize, visible_height: usize) {
        let half_width = area.width / 2;
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(half_width),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(area);

        let end = (scroll + visible_height).min(self.display_rows.len());
        let visible = &self.display_rows[scroll..end];

        let mut left_lines: Vec<Line> = Vec::new();
        let mut right_lines: Vec<Line> = Vec::new();

        let mut i = 0;
        while i < visible.len() {
            let global_idx = scroll + i;

            match &visible[i] {
                DisplayRow::DiffLine { line, .. }
                    if line.kind == crate::types::LineKind::Removed =>
                {
                    let mut removed = Vec::new();
                    let mut j = i;
                    while j < visible.len() {
                        if let DisplayRow::DiffLine { line: l, .. } = &visible[j]
                            && l.kind == crate::types::LineKind::Removed
                        {
                            removed.push((scroll + j, &visible[j]));
                            j += 1;
                            continue;
                        }
                        break;
                    }
                    let mut added = Vec::new();
                    while j < visible.len() {
                        if let DisplayRow::DiffLine { line: l, .. } = &visible[j]
                            && l.kind == crate::types::LineKind::Added
                        {
                            added.push((scroll + j, &visible[j]));
                            j += 1;
                            continue;
                        }
                        break;
                    }
                    let max_len = removed.len().max(added.len());
                    for k in 0..max_len {
                        let sel_left = removed.get(k).is_some_and(|(gi, _)| *gi == self.cursor);
                        let sel_right = added.get(k).is_some_and(|(gi, _)| *gi == self.cursor);
                        let selected = sel_left || sel_right;

                        let mut left = removed
                            .get(k)
                            .map(|(_, row)| {
                                render_sbs_row(row, &self.files, half_width, selected).0
                            })
                            .unwrap_or_default();
                        let mut right = added
                            .get(k)
                            .map(|(_, row)| {
                                render_sbs_row(row, &self.files, half_width, selected).1
                            })
                            .unwrap_or_default();

                        if let Some((gi, _)) = removed.get(k) {
                            left = self.search.highlight(left, *gi);
                        }
                        if let Some((gi, _)) = added.get(k) {
                            right = self.search.highlight(right, *gi);
                        }

                        left_lines.push(left);
                        right_lines.push(right);
                    }
                    i = j;
                }
                DisplayRow::DiffLine { .. } => {
                    let selected = global_idx == self.cursor;
                    let (l, r) = render_sbs_row(&visible[i], &self.files, half_width, selected);
                    left_lines.push(self.search.highlight(l, global_idx));
                    right_lines.push(self.search.highlight(r, global_idx));
                    i += 1;
                }
                _ => {
                    let selected = global_idx == self.cursor;
                    let unified =
                        render_unified_row(&visible[i], &self.files, area.width, selected);
                    left_lines.push(self.search.highlight(unified, global_idx));
                    right_lines.push(Line::default());
                    i += 1;
                }
            }
        }

        let left_para = Paragraph::new(left_lines);
        let right_para = Paragraph::new(right_lines);

        Widget::render(left_para, layout[0], buf);

        for y in 0..area.height {
            if let Some(cell) = buf.cell_mut((layout[1].x, layout[1].y + y)) {
                cell.set_char('│');
                cell.set_style(Theme::border());
            }
        }

        Widget::render(right_para, layout[2], buf);
    }

    pub fn ensure_visible(&mut self, visible_height: usize) {
        if self.cursor < self.scroll_offset {
            self.scroll_offset = self.cursor;
        } else if self.cursor >= self.scroll_offset + visible_height {
            self.scroll_offset = self.cursor - visible_height + 1;
        }
    }
}
