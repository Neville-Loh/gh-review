use ratatui::text::{Line, Span};

use crate::types::{DiffFile, DiffLine, ExistingComment, LineKind, ReviewComment, Side};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum DisplayRow {
    FileHeader {
        path: String,
        file_idx: usize,
    },
    HunkHeader {
        text: String,
        file_idx: usize,
    },
    DiffLine {
        line: DiffLine,
        file_idx: usize,
        hunk_idx: usize,
        line_idx: usize,
    },
    ExpandHint {
        file_idx: usize,
        hunk_idx: usize,
        direction: ExpandDirection,
        available_lines: usize,
    },
    CommentHeader {
        author: String,
        is_pending: bool,
        comment_id: usize,
        github_id: Option<u64>,
        expanded: bool,
        body_preview: String,
        body_lines: usize,
        is_reply: bool,
    },
    CommentBodyLine {
        line: Line<'static>,
        is_reply: bool,
    },
    CommentFooter {
        is_reply: bool,
    },
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum ExpandDirection {
    Up,
    Down,
}

fn render_markdown_to_lines(body: &str) -> Vec<Line<'static>> {
    let text = tui_markdown::from_str(body);
    text.lines
        .into_iter()
        .map(|line| {
            Line::from(
                line.spans
                    .into_iter()
                    .map(|span| Span::styled(span.content.to_string(), span.style))
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

pub fn build_display_rows(
    files: &[DiffFile],
    existing_comments: &[ExistingComment],
    pending_comments: &[ReviewComment],
    expanded_comments: &std::collections::HashSet<usize>,
) -> Vec<DisplayRow> {
    let mut rows = Vec::new();
    let mut comment_id_counter: usize = 0;

    for (file_idx, file) in files.iter().enumerate() {
        rows.push(DisplayRow::FileHeader {
            path: file.path.clone(),
            file_idx,
        });

        for (hunk_idx, hunk) in file.hunks.iter().enumerate() {
            rows.push(DisplayRow::HunkHeader {
                text: hunk.header.clone(),
                file_idx,
            });

            for (line_idx, line) in hunk.lines.iter().enumerate() {
                rows.push(DisplayRow::DiffLine {
                    line: line.clone(),
                    file_idx,
                    hunk_idx,
                    line_idx,
                });

                let (target_line, target_side) = match line.kind {
                    LineKind::Added | LineKind::Context => (line.new_lineno, Side::Right),
                    LineKind::Removed => (line.old_lineno, Side::Left),
                };

                if let Some(lineno) = target_line {
                    for ec in existing_comments.iter().filter(|c| {
                        c.path == file.path
                            && c.line == Some(lineno)
                            && matches!(
                                (c.side.as_deref(), &target_side),
                                (Some("LEFT"), Side::Left)
                                    | (Some("RIGHT"), Side::Right)
                                    | (None, _)
                            )
                    }) {
                        let cid = comment_id_counter;
                        comment_id_counter += 1;
                        let is_expanded = expanded_comments.contains(&cid);
                        let preview = ec.body.lines().next().unwrap_or("").to_string();
                        let body_lines = ec.body.lines().count();
                        let is_reply = ec.in_reply_to_id.is_some();

                        rows.push(DisplayRow::CommentHeader {
                            author: ec.user.login.clone(),
                            is_pending: false,
                            comment_id: cid,
                            github_id: Some(ec.id),
                            expanded: is_expanded,
                            body_preview: preview,
                            body_lines,
                            is_reply,
                        });

                        if is_expanded {
                            let md_lines = render_markdown_to_lines(&ec.body);
                            for ml in md_lines {
                                rows.push(DisplayRow::CommentBodyLine { line: ml, is_reply });
                            }
                            rows.push(DisplayRow::CommentFooter { is_reply });
                        }
                    }

                    for pc in pending_comments.iter().filter(|c| {
                        c.path == file.path && c.line == lineno && c.side == target_side
                    }) {
                        let cid = comment_id_counter;
                        comment_id_counter += 1;
                        let is_expanded = expanded_comments.contains(&cid);
                        let preview = pc.body.lines().next().unwrap_or("").to_string();
                        let body_lines = pc.body.lines().count();

                        rows.push(DisplayRow::CommentHeader {
                            author: String::new(),
                            is_pending: true,
                            comment_id: cid,
                            github_id: None,
                            expanded: is_expanded,
                            body_preview: preview,
                            body_lines,
                            is_reply: false,
                        });

                        if is_expanded {
                            let md_lines = render_markdown_to_lines(&pc.body);
                            for ml in md_lines {
                                rows.push(DisplayRow::CommentBodyLine {
                                    line: ml,
                                    is_reply: false,
                                });
                            }
                            rows.push(DisplayRow::CommentFooter { is_reply: false });
                        }
                    }
                }
            }
        }
    }

    rows
}
