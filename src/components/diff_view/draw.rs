use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
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

        let visual_label = if self.is_visual_mode() {
            " -- VISUAL --"
        } else {
            ""
        };
        let title = format!(" Diff ({mode_label}){visual_label} ");
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
        let visual = self.visual_range();

        let lines: Vec<Line> = visible
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let global_idx = scroll + i;
                let selected = global_idx == self.cursor;
                let mut line = render_unified_row(row, &self.files, area.width, selected);
                line = self.search.highlight(line, global_idx);
                if let Some((lo, hi)) = visual
                    && global_idx >= lo && global_idx <= hi && !selected
                {
                    line = Line::from(
                        line.spans
                            .into_iter()
                            .map(|s| Span::styled(s.content, s.style.patch(Theme::visual_select())))
                            .collect::<Vec<_>>(),
                    );
                }
                line
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

        let mut left_lines: Vec<Line> = Vec::new();
        let mut right_lines: Vec<Line> = Vec::new();

        let visual = self.visual_range();
        let apply_visual = |line: Line<'static>, gi: usize, selected: bool| -> Line<'static> {
            if let Some((lo, hi)) = visual
                && gi >= lo && gi <= hi && !selected
            {
                return Line::from(
                    line.spans
                        .into_iter()
                        .map(|s| Span::styled(s.content, s.style.patch(Theme::visual_select())))
                        .collect::<Vec<_>>(),
                );
            }
            line
        };

        let mut i = scroll;
        let mut last_line_kind: Option<crate::types::LineKind> = None;

        while i < self.display_rows.len() && left_lines.len() < visible_height {
            match &self.display_rows[i] {
                DisplayRow::DiffLine { line, .. }
                    if line.kind == crate::types::LineKind::Removed =>
                {
                    let mut removed: Vec<usize> = Vec::new();
                    let mut j = i;
                    while j < self.display_rows.len() {
                        if let DisplayRow::DiffLine { line: l, .. } = &self.display_rows[j]
                            && l.kind == crate::types::LineKind::Removed
                        {
                            removed.push(j);
                            j += 1;
                            continue;
                        }
                        break;
                    }
                    let mut added: Vec<usize> = Vec::new();
                    while j < self.display_rows.len() {
                        if let DisplayRow::DiffLine { line: l, .. } = &self.display_rows[j]
                            && l.kind == crate::types::LineKind::Added
                        {
                            added.push(j);
                            j += 1;
                            continue;
                        }
                        break;
                    }
                    let max_len = removed.len().max(added.len());
                    for k in 0..max_len {
                        if left_lines.len() >= visible_height {
                            break;
                        }
                        let sel_left = removed.get(k).is_some_and(|gi| *gi == self.cursor);
                        let sel_right = added.get(k).is_some_and(|gi| *gi == self.cursor);
                        let selected = sel_left || sel_right;

                        let mut left = removed
                            .get(k)
                            .map(|gi| {
                                render_sbs_row(
                                    &self.display_rows[*gi],
                                    &self.files,
                                    half_width,
                                    selected,
                                )
                                .0
                            })
                            .unwrap_or_default();
                        let mut right = added
                            .get(k)
                            .map(|gi| {
                                render_sbs_row(
                                    &self.display_rows[*gi],
                                    &self.files,
                                    half_width,
                                    selected,
                                )
                                .1
                            })
                            .unwrap_or_default();

                        if let Some(gi) = removed.get(k) {
                            left = self.search.highlight(left, *gi);
                            left = apply_visual(left, *gi, sel_left);
                        }
                        if let Some(gi) = added.get(k) {
                            right = self.search.highlight(right, *gi);
                            right = apply_visual(right, *gi, sel_right);
                        }

                        left_lines.push(left);
                        right_lines.push(right);
                    }
                    last_line_kind = if !added.is_empty() {
                        Some(crate::types::LineKind::Added)
                    } else {
                        Some(crate::types::LineKind::Removed)
                    };
                    i = j;
                }
                DisplayRow::DiffLine { line, .. } => {
                    let selected = i == self.cursor;
                    let (l, r) =
                        render_sbs_row(&self.display_rows[i], &self.files, half_width, selected);
                    let l = apply_visual(self.search.highlight(l, i), i, selected);
                    let r = apply_visual(self.search.highlight(r, i), i, selected);
                    left_lines.push(l);
                    right_lines.push(r);
                    last_line_kind = Some(line.kind.clone());
                    i += 1;
                }
                row => {
                    let selected = i == self.cursor;
                    let is_comment = matches!(
                        row,
                        DisplayRow::CommentHeader { .. }
                            | DisplayRow::CommentBodyLine { .. }
                            | DisplayRow::CommentFooter { .. }
                    );
                    let w = if is_comment { half_width } else { area.width };
                    let unified =
                        render_unified_row(&self.display_rows[i], &self.files, w, selected);
                    let highlighted = self.search.highlight(unified, i);

                    if is_comment
                        && matches!(
                            last_line_kind,
                            Some(crate::types::LineKind::Added)
                                | Some(crate::types::LineKind::Context)
                        )
                    {
                        left_lines.push(Line::default());
                        right_lines.push(highlighted);
                    } else {
                        left_lines.push(highlighted);
                        right_lines.push(Line::default());
                    }
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
