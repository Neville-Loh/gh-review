mod draw;
mod navigation;

use crate::diff::renderer::{DisplayRow, build_display_rows};
use crate::search::{SearchDirection, SearchState};
use crate::types::{DiffFile, DiffMode, ExistingComment, ReviewComment};
use std::collections::HashSet;

pub struct DiffView {
    pub scroll_offset: usize,
    pub cursor: usize,
    pub mode: DiffMode,
    pub search: SearchState,
    pub(crate) display_rows: Vec<DisplayRow>,
    pub(crate) files: Vec<DiffFile>,
    expanded_comments: HashSet<usize>,
}

impl DiffView {
    pub fn new() -> Self {
        Self {
            scroll_offset: 0,
            cursor: 0,
            mode: DiffMode::Unified,
            search: SearchState::new(),
            display_rows: Vec::new(),
            files: Vec::new(),
            expanded_comments: HashSet::new(),
        }
    }

    pub fn rebuild_rows(
        &mut self,
        files: &[DiffFile],
        existing_comments: &[ExistingComment],
        pending_comments: &[ReviewComment],
    ) {
        self.files = files.to_vec();
        self.display_rows = build_display_rows(
            files,
            existing_comments,
            pending_comments,
            &self.expanded_comments,
        );
        self.search.recompute(&self.display_rows);
    }

    /// Compile pattern, find matches, and jump cursor to the first hit.
    pub fn apply_search(&mut self, pattern: &str, direction: SearchDirection) {
        if let Some(cursor) = self
            .search
            .apply(pattern, direction, &self.display_rows, self.cursor)
        {
            self.cursor = cursor;
        }
    }

    // --- Comment helpers ---

    /// If cursor is on a comment row, walk back to the nearest CommentHeader and
    /// return the GitHub API ID + author so we can post a reply.
    pub fn comment_reply_target(&self) -> Option<ReplyTarget> {
        match self.display_rows.get(self.cursor) {
            Some(DisplayRow::CommentHeader { .. })
            | Some(DisplayRow::CommentBodyLine { .. })
            | Some(DisplayRow::CommentFooter { .. }) => {}
            _ => return None,
        }

        for i in (0..=self.cursor).rev() {
            match self.display_rows.get(i) {
                Some(DisplayRow::CommentHeader {
                    github_id: Some(gid),
                    author,
                    ..
                }) => {
                    return Some(ReplyTarget {
                        github_id: *gid,
                        author: author.clone(),
                    });
                }
                Some(DisplayRow::CommentHeader {
                    github_id: None, ..
                }) => return None,
                Some(DisplayRow::CommentBodyLine { .. })
                | Some(DisplayRow::CommentFooter { .. }) => continue,
                _ => return None,
            }
        }
        None
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

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            DiffMode::Unified => DiffMode::SideBySide,
            DiffMode::SideBySide => DiffMode::Unified,
        };
    }

    // --- Query helpers ---

    /// Get info about the current cursor line for commenting.
    pub fn current_line_info(&self) -> Option<CommentTarget> {
        self.display_rows.get(self.cursor).and_then(|row| {
            if let DisplayRow::DiffLine { line, file_idx, .. } = row {
                let (lineno, side) = match line.kind {
                    crate::types::LineKind::Added | crate::types::LineKind::Context => {
                        (line.new_lineno?, crate::types::Side::Right)
                    }
                    crate::types::LineKind::Removed => (line.old_lineno?, crate::types::Side::Left),
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
                file_idx, hunk_idx, ..
            } = row
            {
                Some((*file_idx, *hunk_idx))
            } else {
                None
            }
        })
    }
}

#[derive(Debug, Clone)]
pub struct CommentTarget {
    pub file_idx: usize,
    pub line: usize,
    pub side: crate::types::Side,
}

pub struct ReplyTarget {
    pub github_id: u64,
    pub author: String,
}
