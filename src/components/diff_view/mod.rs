mod draw;
mod navigation;

use crate::diff::renderer::{DisplayRow, build_display_rows};
use crate::search::{SearchDirection, SearchState};
use crate::types::{DiffFile, DiffMode, ExistingComment, ReviewComment, ThreadInfo};
use std::collections::{HashMap, HashSet};

pub struct DiffView {
    pub scroll_offset: usize,
    pub cursor: usize,
    pub mode: DiffMode,
    pub search: SearchState,
    pub(crate) display_rows: Vec<DisplayRow>,
    pub(crate) files: Vec<DiffFile>,
    pub(crate) expanded_threads: HashSet<u64>,
    pub(crate) expanded_pending: HashSet<usize>,
    pub visual_anchor: Option<usize>,
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
            expanded_threads: HashSet::new(),
            expanded_pending: HashSet::new(),
            visual_anchor: None,
        }
    }

    pub fn rebuild_rows(
        &mut self,
        files: &[DiffFile],
        existing_comments: &[ExistingComment],
        pending_comments: &[ReviewComment],
        thread_map: &HashMap<u64, ThreadInfo>,
    ) {
        self.files = files.to_vec();
        self.display_rows = build_display_rows(
            files,
            existing_comments,
            pending_comments,
            &self.expanded_threads,
            &self.expanded_pending,
            thread_map,
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

    /// Toggle expand/collapse for the thread at cursor. Returns true if toggled.
    pub fn toggle_comment_expand(&mut self) -> bool {
        let row = self.display_rows.get(self.cursor);
        match row {
            Some(DisplayRow::CommentHeader {
                thread_root_id: Some(root_id),
                ..
            }) => {
                let id = *root_id;
                if self.expanded_threads.contains(&id) {
                    self.expanded_threads.remove(&id);
                } else {
                    self.expanded_threads.insert(id);
                }
                true
            }
            Some(DisplayRow::CommentHeader {
                is_pending: true,
                pending_idx: Some(idx),
                ..
            }) => {
                let id = *idx;
                if self.expanded_pending.contains(&id) {
                    self.expanded_pending.remove(&id);
                } else {
                    self.expanded_pending.insert(id);
                }
                true
            }
            _ => false,
        }
    }

    /// If cursor is on a pending comment header, return its index in pending_comments.
    pub fn pending_comment_at_cursor(&self) -> Option<PendingCommentTarget> {
        match self.display_rows.get(self.cursor) {
            Some(DisplayRow::CommentHeader {
                is_pending: true,
                pending_idx: Some(idx),
                ..
            }) => Some(PendingCommentTarget {
                pending_idx: *idx,
            }),
            _ => {
                for i in (0..self.cursor).rev() {
                    match self.display_rows.get(i) {
                        Some(DisplayRow::CommentHeader {
                            is_pending: true,
                            pending_idx: Some(idx),
                            ..
                        }) => {
                            return Some(PendingCommentTarget {
                                pending_idx: *idx,
                            });
                        }
                        Some(DisplayRow::CommentBodyLine { .. })
                        | Some(DisplayRow::CommentFooter { .. }) => continue,
                        _ => return None,
                    }
                }
                None
            }
        }
    }

    /// If cursor is on a comment row, find the thread's node_id and resolve status.
    pub fn thread_resolve_target(&self) -> Option<ThreadResolveTarget> {
        let row = self.display_rows.get(self.cursor)?;
        let search_from = match row {
            DisplayRow::CommentHeader { thread_node_id, is_resolved, .. } => {
                if let Some(id) = thread_node_id {
                    return Some(ThreadResolveTarget {
                        thread_node_id: id.clone(),
                        is_resolved: *is_resolved,
                    });
                }
                return None;
            }
            DisplayRow::CommentBodyLine { .. } | DisplayRow::CommentFooter { .. } => self.cursor,
            _ => return None,
        };

        for i in (0..search_from).rev() {
            if let Some(DisplayRow::CommentHeader { thread_node_id, is_resolved, .. }) = self.display_rows.get(i) {
                if let Some(id) = thread_node_id {
                    return Some(ThreadResolveTarget {
                        thread_node_id: id.clone(),
                        is_resolved: *is_resolved,
                    });
                }
                return None;
            }
            match self.display_rows.get(i) {
                Some(DisplayRow::CommentBodyLine { .. }) | Some(DisplayRow::CommentFooter { .. }) => continue,
                _ => return None,
            }
        }
        None
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

    /// If cursor is on a SuggestionDiff row, return info for accepting it.
    pub fn suggestion_at_cursor(&self) -> Option<SuggestionTarget> {
        if let Some(DisplayRow::SuggestionDiff { suggested, github_id: Some(gid), .. }) = self.display_rows.get(self.cursor) {
            for i in (0..self.cursor).rev() {
                if let Some(DisplayRow::DiffLine { line, file_idx, .. }) = self.display_rows.get(i) {
                    let lineno = match line.kind {
                        crate::types::LineKind::Added | crate::types::LineKind::Context => line.new_lineno,
                        crate::types::LineKind::Removed => line.old_lineno,
                    };
                    if let Some(ln) = lineno {
                        return Some(SuggestionTarget {
                            github_id: *gid,
                            suggested: suggested.clone(),
                            file_idx: *file_idx,
                            line: ln,
                        });
                    }
                }
                if let Some(DisplayRow::FileHeader { .. } | DisplayRow::HunkHeader { .. }) = self.display_rows.get(i) {
                    break;
                }
            }
        }
        None
    }

    pub fn is_visual_mode(&self) -> bool {
        self.visual_anchor.is_some()
    }

    pub fn start_visual(&mut self) {
        self.visual_anchor = Some(self.cursor);
    }

    pub fn cancel_visual(&mut self) {
        self.visual_anchor = None;
    }

    /// Returns (start_row, end_row) range of the visual selection, inclusive.
    pub fn visual_range(&self) -> Option<(usize, usize)> {
        self.visual_anchor.map(|anchor| {
            let lo = anchor.min(self.cursor);
            let hi = anchor.max(self.cursor);
            (lo, hi)
        })
    }

    /// Get the line info at a specific display row index.
    fn line_info_at(&self, row_idx: usize) -> Option<CommentTarget> {
        self.display_rows.get(row_idx).and_then(|row| {
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

    /// Get start and end line info for the visual selection (for multi-line comments).
    pub fn visual_selection_targets(&self) -> Option<(CommentTarget, CommentTarget)> {
        let (lo, hi) = self.visual_range()?;
        let mut start = None;
        let mut end = None;
        for i in lo..=hi {
            if let Some(target) = self.line_info_at(i) {
                if start.is_none() {
                    start = Some(target);
                } else {
                    end = Some(target);
                }
            }
        }
        let start = start?;
        let end = end.unwrap_or_else(|| start.clone());
        Some((start, end))
    }

    /// Get the content of the diff line at cursor (for suggestion editing).
    pub fn current_line_content(&self) -> Option<String> {
        if let Some(DisplayRow::DiffLine { line, .. }) = self.display_rows.get(self.cursor) {
            Some(line.content.clone())
        } else {
            None
        }
    }

    /// Get the concatenated content of all diff lines in the visual selection.
    pub fn visual_selection_content(&self) -> Option<String> {
        let (lo, hi) = self.visual_range()?;
        let lines: Vec<&str> = (lo..=hi)
            .filter_map(|i| {
                if let Some(DisplayRow::DiffLine { line, .. }) = self.display_rows.get(i) {
                    let content = line.content.as_str();
                    Some(content.strip_prefix(' ').unwrap_or(content))
                } else {
                    None
                }
            })
            .collect();
        if lines.is_empty() {
            None
        } else {
            Some(lines.join("\n"))
        }
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

pub struct PendingCommentTarget {
    pub pending_idx: usize,
}

pub struct ThreadResolveTarget {
    pub thread_node_id: String,
    pub is_resolved: bool,
}

pub struct SuggestionTarget {
    #[allow(dead_code)]
    pub github_id: u64,
    pub suggested: String,
    pub file_idx: usize,
    pub line: usize,
}
