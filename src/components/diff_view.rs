use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::diff::renderer::{build_display_rows, render_sbs_row, render_unified_row, DisplayRow};
use crate::theme::Theme;
use crate::types::{DiffFile, DiffMode, ExistingComment, ReviewComment};
use std::collections::HashSet;

pub struct DiffView {
    pub scroll_offset: usize,
    pub cursor: usize,
    pub mode: DiffMode,
    display_rows: Vec<DisplayRow>,
    expanded_comments: HashSet<usize>,
}

impl DiffView {
    pub fn new() -> Self {
        Self {
            scroll_offset: 0,
            cursor: 0,
            mode: DiffMode::Unified,
            display_rows: Vec::new(),
            expanded_comments: HashSet::new(),
        }
    }

    pub fn rebuild_rows(
        &mut self,
        files: &[DiffFile],
        existing_comments: &[ExistingComment],
        pending_comments: &[ReviewComment],
    ) {
        self.display_rows = build_display_rows(
            files,
            existing_comments,
            pending_comments,
            &self.expanded_comments,
        );
    }

    /// Toggle expand/collapse if cursor is on a comment row. Returns true if toggled.
    pub fn toggle_comment_expand(&mut self) -> bool {
        let comment_id = match self.display_rows.get(self.cursor) {
            Some(DisplayRow::CommentHeader { comment_id, .. }) => Some(*comment_id),
            _ => None,
        };
        if let Some(cid) = comment_id {
            if self.expanded_comments.contains(&cid) {
                self.expanded_comments.remove(&cid);
            } else {
                self.expanded_comments.insert(cid);
            }
            true
        } else {
            false
        }
    }

    pub fn total_rows(&self) -> usize {
        self.display_rows.len()
    }

    pub fn scroll_down(&mut self, n: usize) {
        let max = self.display_rows.len().saturating_sub(1);
        self.cursor = (self.cursor + n).min(max);
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.cursor = self.cursor.saturating_sub(n);
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.scroll_down(page_size);
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.scroll_up(page_size);
    }

    pub fn goto_first(&mut self) {
        self.cursor = 0;
    }

    pub fn goto_last(&mut self) {
        self.cursor = self.display_rows.len().saturating_sub(1);
    }

    /// Jump to the first row of the given file index.
    pub fn goto_file(&mut self, file_idx: usize) {
        for (i, row) in self.display_rows.iter().enumerate() {
            if let DisplayRow::FileHeader { file_idx: fi, .. } = row {
                if *fi == file_idx {
                    self.cursor = i;
                    return;
                }
            }
        }
    }

    /// Jump to next file header.
    pub fn next_file(&mut self) {
        let start = self.cursor + 1;
        for i in start..self.display_rows.len() {
            if matches!(self.display_rows[i], DisplayRow::FileHeader { .. }) {
                self.cursor = i;
                return;
            }
        }
    }

    /// Jump to previous file header.
    pub fn prev_file(&mut self) {
        if self.cursor == 0 {
            return;
        }
        for i in (0..self.cursor).rev() {
            if matches!(self.display_rows[i], DisplayRow::FileHeader { .. }) {
                self.cursor = i;
                return;
            }
        }
    }

    /// Jump to next hunk header (`]` or `}`).
    pub fn next_hunk(&mut self) {
        let start = self.cursor + 1;
        for i in start..self.display_rows.len() {
            if matches!(self.display_rows[i], DisplayRow::HunkHeader { .. }) {
                self.cursor = i;
                return;
            }
        }
    }

    /// Jump to previous hunk header (`[` or `{`).
    pub fn prev_hunk(&mut self) {
        if self.cursor == 0 {
            return;
        }
        for i in (0..self.cursor).rev() {
            if matches!(self.display_rows[i], DisplayRow::HunkHeader { .. }) {
                self.cursor = i;
                return;
            }
        }
    }

    /// Jump to next change (added or removed line).
    pub fn next_change(&mut self) {
        let start = self.cursor + 1;
        for i in start..self.display_rows.len() {
            if let DisplayRow::DiffLine { line, .. } = &self.display_rows[i] {
                if line.kind != crate::types::LineKind::Context {
                    self.cursor = i;
                    return;
                }
            }
        }
    }

    /// Jump to previous change (added or removed line).
    pub fn prev_change(&mut self) {
        if self.cursor == 0 {
            return;
        }
        for i in (0..self.cursor).rev() {
            if let DisplayRow::DiffLine { line, .. } = &self.display_rows[i] {
                if line.kind != crate::types::LineKind::Context {
                    self.cursor = i;
                    return;
                }
            }
        }
    }

    /// Move cursor to top of visible area (H in vim).
    pub fn screen_top(&mut self) {
        self.cursor = self.scroll_offset;
    }

    /// Move cursor to middle of visible area (M in vim).
    pub fn screen_middle(&mut self, visible_height: usize) {
        let mid = self.scroll_offset + visible_height / 2;
        self.cursor = mid.min(self.display_rows.len().saturating_sub(1));
    }

    /// Move cursor to bottom of visible area (L in vim).
    pub fn screen_bottom(&mut self, visible_height: usize) {
        let bot = (self.scroll_offset + visible_height).saturating_sub(1);
        self.cursor = bot.min(self.display_rows.len().saturating_sub(1));
    }

    /// Center the viewport on the cursor (zz in vim).
    pub fn center_cursor(&mut self, visible_height: usize) {
        self.scroll_offset = self.cursor.saturating_sub(visible_height / 2);
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            DiffMode::Unified => DiffMode::SideBySide,
            DiffMode::SideBySide => DiffMode::Unified,
        };
    }

    /// Get info about the current cursor line for commenting.
    pub fn current_line_info(&self) -> Option<CommentTarget> {
        self.display_rows.get(self.cursor).and_then(|row| {
            if let DisplayRow::DiffLine {
                line, file_idx, ..
            } = row
            {
                let (lineno, side) = match line.kind {
                    crate::types::LineKind::Added | crate::types::LineKind::Context => {
                        (line.new_lineno?, crate::types::Side::Right)
                    }
                    crate::types::LineKind::Removed => {
                        (line.old_lineno?, crate::types::Side::Left)
                    }
                };
                Some(CommentTarget {
                    file_idx: *file_idx,
                    line: lineno,
                    side,
                })
            } else {
                None
            }
        })
    }

    /// Get the current file index at the cursor position.
    pub fn current_file_idx(&self) -> Option<usize> {
        self.display_rows.get(self.cursor).map(|row| match row {
            DisplayRow::FileHeader { file_idx, .. } => *file_idx,
            DisplayRow::HunkHeader { file_idx, .. } => *file_idx,
            DisplayRow::DiffLine { file_idx, .. } => *file_idx,
            DisplayRow::ExpandHint { file_idx, .. } => *file_idx,
            _ => 0,
        })
    }

    /// Get hunk index at cursor for expand operations.
    pub fn current_hunk_idx(&self) -> Option<(usize, usize)> {
        self.display_rows.get(self.cursor).and_then(|row| {
            if let DisplayRow::DiffLine {
                file_idx,
                hunk_idx,
                ..
            } = row
            {
                Some((*file_idx, *hunk_idx))
            } else {
                None
            }
        })
    }

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

        // Clear the inner area to prevent artifacts from previous frames
        for y in inner.y..inner.y + inner.height {
            for x in inner.x..inner.x + inner.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.reset();
                }
            }
        }

        // Ensure cursor is visible
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
                render_unified_row(row, area.width, selected)
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

        // Build paired SBS lines by grouping consecutive removed+added blocks
        let mut left_lines: Vec<Line> = Vec::new();
        let mut right_lines: Vec<Line> = Vec::new();

        let mut i = 0;
        while i < visible.len() {
            let global_idx = scroll + i;

            match &visible[i] {
                DisplayRow::DiffLine { line, .. }
                    if line.kind == crate::types::LineKind::Removed =>
                {
                    // Collect consecutive removed lines
                    let mut removed = Vec::new();
                    let mut j = i;
                    while j < visible.len() {
                        if let DisplayRow::DiffLine { line: l, .. } = &visible[j] {
                            if l.kind == crate::types::LineKind::Removed {
                                removed.push((scroll + j, &visible[j]));
                                j += 1;
                                continue;
                            }
                        }
                        break;
                    }
                    // Collect consecutive added lines that follow
                    let mut added = Vec::new();
                    while j < visible.len() {
                        if let DisplayRow::DiffLine { line: l, .. } = &visible[j] {
                            if l.kind == crate::types::LineKind::Added {
                                added.push((scroll + j, &visible[j]));
                                j += 1;
                                continue;
                            }
                        }
                        break;
                    }
                    // Pair them up
                    let max_len = removed.len().max(added.len());
                    for k in 0..max_len {
                        let sel_left = removed.get(k).map_or(false, |(gi, _)| *gi == self.cursor);
                        let sel_right = added.get(k).map_or(false, |(gi, _)| *gi == self.cursor);
                        let selected = sel_left || sel_right;

                        let left = removed
                            .get(k)
                            .map(|(_, row)| render_sbs_row(row, half_width, selected).0)
                            .unwrap_or_default();
                        let right = added
                            .get(k)
                            .map(|(_, row)| render_sbs_row(row, half_width, selected).1)
                            .unwrap_or_default();
                        left_lines.push(left);
                        right_lines.push(right);
                    }
                    i = j;
                }
                DisplayRow::DiffLine { .. } => {
                    let selected = global_idx == self.cursor;
                    let (l, r) = render_sbs_row(&visible[i], half_width, selected);
                    left_lines.push(l);
                    right_lines.push(r);
                    i += 1;
                }
                _ => {
                    let selected = global_idx == self.cursor;
                    let unified = render_unified_row(&visible[i], area.width, selected);
                    left_lines.push(unified);
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

    /// Update the internal scroll offset to keep cursor visible.
    /// Call this after any cursor movement.
    pub fn ensure_visible(&mut self, visible_height: usize) {
        if self.cursor < self.scroll_offset {
            self.scroll_offset = self.cursor;
        } else if self.cursor >= self.scroll_offset + visible_height {
            self.scroll_offset = self.cursor - visible_height + 1;
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommentTarget {
    pub file_idx: usize,
    pub line: usize,
    pub side: crate::types::Side,
}
