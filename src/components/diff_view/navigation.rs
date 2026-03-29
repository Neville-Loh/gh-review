use crate::diff::renderer::DisplayRow;

use super::DiffView;

impl DiffView {
    #[allow(dead_code)]
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

    pub fn page_down(&mut self, page_size: usize, smooth: bool) {
        let max = self.display_rows.len().saturating_sub(1);
        let target_scroll = (self.scroll_offset + page_size).min(max);
        let target_cursor = (self.cursor + page_size).min(max);

        if smooth && page_size > 1 {
            let step = (page_size / 8).max(1);
            self.scroll_animation = Some(super::ScrollAnimation {
                target_scroll,
                target_cursor,
                step,
            });
        } else {
            self.scroll_offset = target_scroll;
            self.cursor = target_cursor;
        }
    }

    pub fn page_up(&mut self, page_size: usize, smooth: bool) {
        let target_scroll = self.scroll_offset.saturating_sub(page_size);
        let target_cursor = self.cursor.saturating_sub(page_size);

        if smooth && page_size > 1 {
            let step = (page_size / 8).max(1);
            self.scroll_animation = Some(super::ScrollAnimation {
                target_scroll,
                target_cursor,
                step,
            });
        } else {
            self.scroll_offset = target_scroll;
            self.cursor = target_cursor;
        }
    }

    pub fn is_animating(&self) -> bool {
        self.scroll_animation.is_some()
    }

    pub fn step_animation(&mut self) {
        let Some(ref anim) = self.scroll_animation else {
            return;
        };
        let target_scroll = anim.target_scroll;
        let target_cursor = anim.target_cursor;
        let step = anim.step;

        if self.scroll_offset < target_scroll {
            self.scroll_offset = (self.scroll_offset + step).min(target_scroll);
        } else if self.scroll_offset > target_scroll {
            let moved = self.scroll_offset.saturating_sub(step);
            self.scroll_offset = moved.max(target_scroll);
        }

        if self.cursor < target_cursor {
            self.cursor = (self.cursor + step).min(target_cursor);
        } else if self.cursor > target_cursor {
            let moved = self.cursor.saturating_sub(step);
            self.cursor = moved.max(target_cursor);
        }

        if self.scroll_offset == target_scroll && self.cursor == target_cursor {
            self.scroll_animation = None;
        }
    }

    pub fn finish_animation(&mut self) {
        if let Some(anim) = self.scroll_animation.take() {
            self.scroll_offset = anim.target_scroll;
            self.cursor = anim.target_cursor;
        }
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
            if let DisplayRow::FileHeader { file_idx: fi, .. } = row
                && *fi == file_idx
            {
                self.cursor = i;
                return;
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

    /// Jump to next hunk header.
    pub fn next_hunk(&mut self) {
        let start = self.cursor + 1;
        for i in start..self.display_rows.len() {
            if matches!(self.display_rows[i], DisplayRow::HunkHeader { .. }) {
                self.cursor = i;
                return;
            }
        }
    }

    /// Jump to previous hunk header.
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
            if let DisplayRow::DiffLine { line, .. } = &self.display_rows[i]
                && line.kind != crate::types::LineKind::Context
            {
                self.cursor = i;
                return;
            }
        }
    }

    /// Jump to previous change (added or removed line).
    pub fn prev_change(&mut self) {
        if self.cursor == 0 {
            return;
        }
        for i in (0..self.cursor).rev() {
            if let DisplayRow::DiffLine { line, .. } = &self.display_rows[i]
                && line.kind != crate::types::LineKind::Context
            {
                self.cursor = i;
                return;
            }
        }
    }

    pub fn screen_top(&mut self) {
        self.cursor = self.scroll_offset;
    }

    pub fn screen_middle(&mut self, visible_height: usize) {
        let mid = self.scroll_offset + visible_height / 2;
        self.cursor = mid.min(self.display_rows.len().saturating_sub(1));
    }

    pub fn screen_bottom(&mut self, visible_height: usize) {
        let bot = (self.scroll_offset + visible_height).saturating_sub(1);
        self.cursor = bot.min(self.display_rows.len().saturating_sub(1));
    }

    pub fn center_cursor(&mut self, visible_height: usize) {
        self.scroll_offset = self.cursor.saturating_sub(visible_height / 2);
    }

    /// Jump to the next comment thread header (root, not reply).
    pub fn next_comment(&mut self) {
        let start = self.cursor + 1;
        for i in start..self.display_rows.len() {
            if matches!(
                self.display_rows[i],
                DisplayRow::CommentHeader { is_reply: false, .. }
            ) {
                self.cursor = i;
                return;
            }
        }
    }

    /// Jump to the previous comment thread header (root, not reply).
    pub fn prev_comment(&mut self) {
        if self.cursor == 0 {
            return;
        }
        for i in (0..self.cursor).rev() {
            if matches!(
                self.display_rows[i],
                DisplayRow::CommentHeader { is_reply: false, .. }
            ) {
                self.cursor = i;
                return;
            }
        }
    }

    /// Jump to the next blank line in the diff content.
    /// Like vim `}` — skip non-blank lines, land on the next blank one.
    pub fn next_paragraph(&mut self) {
        let start = self.cursor + 1;
        let mut was_blank = Self::is_blank_row(&self.display_rows, self.cursor);
        for i in start..self.display_rows.len() {
            let blank = Self::is_blank_row(&self.display_rows, i);
            if blank && !was_blank {
                self.cursor = i;
                return;
            }
            was_blank = blank;
        }
        self.cursor = self.display_rows.len().saturating_sub(1);
    }

    /// Jump to the previous blank line in the diff content.
    /// Like vim `{` — skip non-blank lines backwards, land on blank.
    pub fn prev_paragraph(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let mut was_blank = Self::is_blank_row(&self.display_rows, self.cursor);
        for i in (0..self.cursor).rev() {
            let blank = Self::is_blank_row(&self.display_rows, i);
            if blank && !was_blank {
                self.cursor = i;
                return;
            }
            was_blank = blank;
        }
        self.cursor = 0;
    }

    fn is_blank_row(rows: &[DisplayRow], idx: usize) -> bool {
        match rows.get(idx) {
            Some(DisplayRow::DiffLine { line, .. }) => line.content.trim().is_empty(),
            Some(DisplayRow::FileHeader { .. })
            | Some(DisplayRow::HunkHeader { .. }) => true,
            _ => false,
        }
    }
}
