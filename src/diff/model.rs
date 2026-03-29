use std::collections::HashMap;

use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::layout;
use super::wrap::wrap_spans;
use crate::stack::graphite;
use crate::types::{DiffFile, DiffLine, ExistingComment, LineKind, ReviewComment, Side, ThreadInfo};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum DisplayRow {
    FileHeader {
        path: String,
        file_idx: usize,
        collapsed: bool,
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
        github_id: Option<u64>,
        pending_idx: Option<usize>,
        thread_root_id: Option<u64>,
        thread_node_id: Option<String>,
        is_resolved: bool,
        expanded: bool,
        reply_count: usize,
        body_preview: String,
        is_reply: bool,
    },
    CommentBodyLine {
        line: Line<'static>,
        is_reply: bool,
        is_resolved: bool,
        is_pending: bool,
        is_suggestion: bool,
    },
    CommentFooter {
        is_reply: bool,
        is_resolved: bool,
        is_pending: bool,
    },
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum ExpandDirection {
    Up,
    Down,
}

use super::suggestion;

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

fn blank_body_row(is_resolved: bool, is_pending: bool) -> DisplayRow {
    DisplayRow::CommentBodyLine {
        line: Line::default(),
        is_reply: false,
        is_resolved,
        is_pending,
        is_suggestion: false,
    }
}

fn wrap_body_lines(
    md_lines: Vec<Line<'static>>,
    max_width: usize,
    is_reply: bool,
    is_resolved: bool,
    is_pending: bool,
) -> Vec<DisplayRow> {
    let mut rows = Vec::new();
    for ml in md_lines {
        let total_width: usize = ml.spans.iter().map(|s| s.content.width()).sum();
        if total_width <= max_width {
            rows.push(DisplayRow::CommentBodyLine {
                line: ml,
                is_reply,
                is_resolved,
                is_pending,
                is_suggestion: false,
            });
        } else {
            for wrapped_line in wrap_spans(&ml.spans, max_width) {
                rows.push(DisplayRow::CommentBodyLine {
                    line: wrapped_line,
                    is_reply,
                    is_resolved,
                    is_pending,
                    is_suggestion: false,
                });
            }
        }
    }
    rows
}

#[allow(clippy::too_many_arguments)]
pub fn build_display_rows(
    files: &[DiffFile],
    existing_comments: &[ExistingComment],
    pending_comments: &[ReviewComment],
    expanded_threads: &std::collections::HashSet<u64>,
    expanded_pending: &std::collections::HashSet<usize>,
    thread_map: &HashMap<u64, ThreadInfo>,
    wrap_width: usize,
    collapsed_files: &std::collections::HashSet<usize>,
) -> Vec<DisplayRow> {
    let mut rows = Vec::new();
    let body_max_width = layout::comment_body_width(wrap_width);

    for (file_idx, file) in files.iter().enumerate() {
        let collapsed = collapsed_files.contains(&file_idx);
        rows.push(DisplayRow::FileHeader {
            path: file.path.clone(),
            file_idx,
            collapsed,
        });

        if collapsed {
            continue;
        }

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
                    let line_comments: Vec<&ExistingComment> = existing_comments
                        .iter()
                        .filter(|c| {
                            c.path == file.path
                                && c.line == Some(lineno)
                                && !graphite::is_graphite_stack_comment(&c.body)
                                && matches!(
                                    (c.side.as_deref(), &target_side),
                                    (Some("LEFT"), Side::Left)
                                        | (Some("RIGHT"), Side::Right)
                                        | (None, _)
                                )
                        })
                        .collect();

                    let mut threads: std::collections::BTreeMap<u64, Vec<&ExistingComment>> =
                        std::collections::BTreeMap::new();
                    for ec in &line_comments {
                        let root_id = ec.in_reply_to_id.unwrap_or(ec.id);
                        threads.entry(root_id).or_default().push(ec);
                    }

                    for (root_id, thread_comments) in &threads {
                        let root = thread_comments[0];
                        let reply_count = thread_comments.len().saturating_sub(1);
                        let thread_info = thread_map.get(root_id);
                        let thread_node_id = thread_info.map(|t| t.thread_node_id.clone());
                        let is_resolved = thread_info.map(|t| t.is_resolved).unwrap_or(false);
                        let default_open = !is_resolved;
                        let is_expanded = if expanded_threads.contains(root_id) {
                            !default_open
                        } else {
                            default_open
                        };
                        let preview = root.body.lines().next().unwrap_or("").to_string();

                        rows.push(DisplayRow::CommentHeader {
                            author: root.user.login.clone(),
                            is_pending: false,
                            github_id: Some(root.id),
                            pending_idx: None,
                            thread_root_id: Some(*root_id),
                            thread_node_id,
                            is_resolved,
                            expanded: is_expanded,
                            reply_count,
                            body_preview: preview,
                            is_reply: false,
                        });

                        if is_expanded {
                            rows.push(blank_body_row(is_resolved, false));

                            let sug_text = suggestion::extract(&root.body);
                            let body_text = if sug_text.is_some() {
                                suggestion::strip_block(&root.body)
                            } else {
                                root.body.clone()
                            };

                            if !body_text.trim().is_empty() {
                                let md_lines = render_markdown_to_lines(&body_text);
                                rows.extend(wrap_body_lines(md_lines, body_max_width, false, is_resolved, false));
                            }

                            if let Some(ref suggested) = sug_text {
                                let original_lines = suggestion::collect_original_lines(
                                    &hunk.lines, line, lineno, root.start_line,
                                );
                                rows.extend(suggestion::build_rows(
                                    &file.path, &original_lines, suggested, is_resolved,
                                ));
                            }

                            for reply in thread_comments.iter().skip(1) {
                                rows.push(blank_body_row(is_resolved, false));

                                rows.push(DisplayRow::CommentHeader {
                                    author: reply.user.login.clone(),
                                    is_pending: false,
                                    github_id: Some(reply.id),
                                    pending_idx: None,
                                    thread_root_id: Some(*root_id),
                                    thread_node_id: None,
                                    is_resolved,
                                    expanded: true,
                                    reply_count: 0,
                                    body_preview: String::new(),
                                    is_reply: true,
                                });

                                let reply_lines = render_markdown_to_lines(&reply.body);
                                rows.extend(wrap_body_lines(reply_lines, body_max_width, true, is_resolved, false));
                            }

                            rows.push(blank_body_row(is_resolved, false));
                            rows.push(DisplayRow::CommentFooter {
                                is_reply: false,
                                is_resolved,
                                is_pending: false,
                            });
                        }
                    }

                    for (pc_idx, pc) in pending_comments.iter().enumerate().filter(|(_, c)| {
                        c.path == file.path && c.line == lineno && c.side == target_side
                    }) {
                        let is_expanded = expanded_pending.contains(&pc_idx);
                        let preview = pc.body.lines().next().unwrap_or("").to_string();

                        rows.push(DisplayRow::CommentHeader {
                            author: String::new(),
                            is_pending: true,
                            github_id: None,
                            pending_idx: Some(pc_idx),
                            thread_root_id: None,
                            thread_node_id: None,
                            is_resolved: false,
                            expanded: is_expanded,
                            reply_count: 0,
                            body_preview: preview,
                            is_reply: false,
                        });

                        if is_expanded {
                            rows.push(blank_body_row(false, true));

                            let sug_text = suggestion::extract(&pc.body);
                            let body_text = if sug_text.is_some() {
                                suggestion::strip_block(&pc.body)
                            } else {
                                pc.body.clone()
                            };

                            if !body_text.trim().is_empty() {
                                let md_lines = render_markdown_to_lines(&body_text);
                                rows.extend(wrap_body_lines(md_lines, body_max_width, false, false, true));
                            }

                            if let Some(ref suggested) = sug_text {
                                let original_lines = suggestion::collect_original_lines(
                                    &hunk.lines, line, lineno, None,
                                );
                                rows.extend(suggestion::build_rows(
                                    &file.path, &original_lines, suggested, false,
                                ));
                            }

                            rows.push(blank_body_row(false, true));
                            rows.push(DisplayRow::CommentFooter {
                                is_reply: false,
                                is_resolved: false,
                                is_pending: true,
                            });
                        }
                    }
                }
            }
        }
    }

    // Collect file-level comments (line: None) -- not attached to any diff line.
    // Filter out auto-generated Graphite stack comments.
    let orphan_comments: Vec<&ExistingComment> = existing_comments
        .iter()
        .filter(|c| c.line.is_none() && !graphite::is_graphite_stack_comment(&c.body))
        .collect();

    if !orphan_comments.is_empty() {
        let mut threads: std::collections::BTreeMap<u64, Vec<&ExistingComment>> =
            std::collections::BTreeMap::new();
        for ec in &orphan_comments {
            let root_id = ec.in_reply_to_id.unwrap_or(ec.id);
            threads.entry(root_id).or_default().push(ec);
        }

        for (root_id, thread_comments) in &threads {
            let root = thread_comments[0];
            let reply_count = thread_comments.len().saturating_sub(1);
            let thread_info = thread_map.get(root_id);
            let thread_node_id = thread_info.map(|t| t.thread_node_id.clone());
            let is_resolved = thread_info.map(|t| t.is_resolved).unwrap_or(false);
            let default_open = !is_resolved;
            let is_expanded = if expanded_threads.contains(root_id) {
                !default_open
            } else {
                default_open
            };
            let preview = root.body.lines().next().unwrap_or("").to_string();

            rows.push(DisplayRow::CommentHeader {
                author: root.user.login.clone(),
                is_pending: false,
                github_id: Some(root.id),
                pending_idx: None,
                thread_root_id: Some(*root_id),
                thread_node_id,
                is_resolved,
                expanded: is_expanded,
                reply_count,
                body_preview: preview,
                is_reply: false,
            });

            if is_expanded {
                rows.push(blank_body_row(is_resolved, false));

                let md_lines = render_markdown_to_lines(&root.body);
                rows.extend(wrap_body_lines(md_lines, body_max_width, false, is_resolved, false));

                for reply in thread_comments.iter().skip(1) {
                    rows.push(blank_body_row(is_resolved, false));

                    rows.push(DisplayRow::CommentHeader {
                        author: reply.user.login.clone(),
                        is_pending: false,
                        github_id: Some(reply.id),
                        pending_idx: None,
                        thread_root_id: Some(*root_id),
                        thread_node_id: None,
                        is_resolved,
                        expanded: true,
                        reply_count: 0,
                        body_preview: String::new(),
                        is_reply: true,
                    });

                    let reply_lines = render_markdown_to_lines(&reply.body);
                    rows.extend(wrap_body_lines(reply_lines, body_max_width, true, is_resolved, false));
                }

                rows.push(blank_body_row(is_resolved, false));
                rows.push(DisplayRow::CommentFooter {
                    is_reply: false,
                    is_resolved,
                    is_pending: false,
                });
            }
        }
    }

    rows
}
