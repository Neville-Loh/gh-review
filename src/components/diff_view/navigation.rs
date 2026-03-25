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
}
