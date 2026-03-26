use std::collections::HashMap;

use ratatui::text::{Line, Span};

use crate::types::{DiffFile, DiffLine, ExistingComment, LineKind, ReviewComment, Side, ThreadInfo};

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
    },
    CommentFooter {
        is_reply: bool,
        is_resolved: bool,
    },
    SuggestionDiff {
        original: String,
        suggested: String,
        github_id: Option<u64>,
    },
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum ExpandDirection {
    Up,
    Down,
}

fn extract_suggestion(body: &str) -> Option<String> {
    let mut in_suggestion = false;
    let mut suggestion_lines = Vec::new();

    for line in body.lines() {
        if line.trim() == "```suggestion" || line.trim().starts_with("```suggestion ") {
            in_suggestion = true;
            continue;
        }
        if in_suggestion {
            if line.trim() == "```" {
                return Some(suggestion_lines.join("\n"));
            }
            suggestion_lines.push(line.to_string());
        }
    }
    None
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
    expanded_threads: &std::collections::HashSet<u64>,
    expanded_pending: &std::collections::HashSet<usize>,
    thread_map: &HashMap<u64, ThreadInfo>,
) -> Vec<DisplayRow> {
    let mut rows = Vec::new();

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
                    let line_comments: Vec<&ExistingComment> = existing_comments
                        .iter()
                        .filter(|c| {
                            c.path == file.path
                                && c.line == Some(lineno)
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
                        let is_expanded = expanded_threads.contains(root_id);
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

                        if let Some(suggested) = extract_suggestion(&root.body) {
                            rows.push(DisplayRow::SuggestionDiff {
                                original: line.content.clone(),
                                suggested,
                                github_id: Some(root.id),
                            });
                        }

                        if is_expanded {
                            let md_lines = render_markdown_to_lines(&root.body);
                            for ml in md_lines {
                                rows.push(DisplayRow::CommentBodyLine {
                                    line: ml,
                                    is_reply: false,
                                    is_resolved,
                                });
                            }

                            for reply in thread_comments.iter().skip(1) {
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
                                for ml in reply_lines {
                                    rows.push(DisplayRow::CommentBodyLine {
                                        line: ml,
                                        is_reply: true,
                                        is_resolved,
                                    });
                                }
                            }

                            rows.push(DisplayRow::CommentFooter {
                                is_reply: false,
                                is_resolved,
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
                            let md_lines = render_markdown_to_lines(&pc.body);
                            for ml in md_lines {
                                rows.push(DisplayRow::CommentBodyLine {
                                    line: ml,
                                    is_reply: false,
                                    is_resolved: false,
                                });
                            }
                            rows.push(DisplayRow::CommentFooter {
                                is_reply: false,
                                is_resolved: false,
                            });
                        }
                    }
                }
            }
        }
    }

    rows
}
