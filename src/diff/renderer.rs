use ratatui::{
    style::{Color, Style, Stylize},
    text::{Line, Span},
};

use crate::highlight::highlight;
use crate::theme::Theme;
use crate::types::{DiffFile, DiffLine, ExistingComment, LineKind, ReviewComment, Side};

#[derive(Debug, Clone)]
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
pub enum ExpandDirection {
    Up,
    Down,
}

const COMMENT_INDENT: &str = "              ";
const REPLY_EXTRA: &str = "    ";
const BOX_PADDING: &str = "  ";

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
                            && match (c.side.as_deref(), &target_side) {
                                (Some("LEFT"), Side::Left) | (Some("RIGHT"), Side::Right) => true,
                                (None, _) => true,
                                _ => false,
                            }
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

const LINE_NUM_WIDTH: usize = 5;

pub fn render_unified_row(
    row: &DisplayRow,
    files: &[DiffFile],
    _width: u16,
    is_selected: bool,
) -> Line<'static> {
    let base_line = match row {
        DisplayRow::FileHeader { path, .. } => Line::from(vec![Span::styled(
            format!("─── {path} ───"),
            Theme::file_header(),
        )]),
        DisplayRow::HunkHeader { text, .. } => {
            Line::from(vec![Span::styled(text.clone(), Theme::hunk_header())])
        }
        DisplayRow::DiffLine { line, file_idx, .. } => {
            let path = files.get(*file_idx).map(|f| f.path.as_str()).unwrap_or("");
            render_unified_diff_line(line, path)
        }
        DisplayRow::ExpandHint {
            available_lines, ..
        } => Line::from(vec![Span::styled(
            format!(
                "{:>pad$}  {:>pad$}   ↕ {available_lines} lines hidden — press e to expand",
                "",
                "",
                pad = LINE_NUM_WIDTH
            ),
            Theme::expand_hint(),
        )]),
        DisplayRow::CommentHeader {
            author,
            is_pending,
            expanded,
            body_preview,
            body_lines,
            is_reply,
            ..
        } => {
            let indent = if *is_reply {
                format!("{COMMENT_INDENT}{REPLY_EXTRA}")
            } else {
                COMMENT_INDENT.to_string()
            };
            let toggle = if *body_lines > 1 {
                if *expanded { "▼" } else { "▶" }
            } else {
                " "
            };
            let marker = if *is_reply { "↩" } else { "💬" };

            if *is_pending {
                if *expanded {
                    Line::from(vec![
                        Span::styled(indent, Theme::line_number()),
                        Span::styled(format!("{toggle} ┌─ 📝 pending "), Theme::pending_count()),
                        Span::styled("─".repeat(30), Theme::pending_count()),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(indent, Theme::line_number()),
                        Span::styled(format!("{toggle} 📝 (pending) "), Theme::pending_count()),
                        Span::styled(body_preview.clone(), Theme::comment_body()),
                    ])
                }
            } else if *expanded {
                Line::from(vec![
                    Span::styled(indent, Theme::line_number()),
                    Span::styled(
                        format!("{toggle} ┌─ {marker} {author} "),
                        Theme::comment_marker(),
                    ),
                    Span::styled("─".repeat(30), Theme::comment_marker()),
                ])
            } else {
                Line::from(vec![
                    Span::styled(indent, Theme::line_number()),
                    Span::styled(
                        format!("{toggle} {marker} {author}: "),
                        Theme::comment_marker(),
                    ),
                    Span::styled(body_preview.clone(), Theme::comment_body()),
                ])
            }
        }
        DisplayRow::CommentBodyLine { line, is_reply } => {
            let indent = if *is_reply {
                format!("{COMMENT_INDENT}{REPLY_EXTRA}")
            } else {
                COMMENT_INDENT.to_string()
            };
            let mut spans = vec![
                Span::styled(indent, Theme::line_number()),
                Span::styled("  │ ", Theme::comment_marker()),
            ];
            spans.extend(line.spans.iter().cloned());
            Line::from(spans)
        }
        DisplayRow::CommentFooter { is_reply } => {
            let indent = if *is_reply {
                format!("{COMMENT_INDENT}{REPLY_EXTRA}")
            } else {
                COMMENT_INDENT.to_string()
            };
            Line::from(vec![
                Span::styled(indent, Theme::line_number()),
                Span::styled(format!("  └{}", "─".repeat(34)), Theme::comment_marker()),
            ])
        }
    };

    if is_selected {
        let mut spans = vec![Span::styled("▌", Theme::selected_cursor())];
        spans.extend(
            base_line
                .spans
                .into_iter()
                .map(|s| Span::styled(s.content, s.style.patch(Theme::selected_line()))),
        );
        Line::from(spans)
    } else {
        base_line
    }
}

fn render_unified_diff_line(line: &DiffLine, path: &str) -> Line<'static> {
    let old_num = line
        .old_lineno
        .map(|n| format!("{n:>LINE_NUM_WIDTH$}"))
        .unwrap_or(" ".repeat(LINE_NUM_WIDTH));
    let new_num = line
        .new_lineno
        .map(|n| format!("{n:>LINE_NUM_WIDTH$}"))
        .unwrap_or(" ".repeat(LINE_NUM_WIDTH));

    let (marker, style, bg_color) = match line.kind {
        LineKind::Added => ("+", Theme::added_line_bg(), Theme::added_line_bg_color()),
        LineKind::Removed => (
            "-",
            Theme::removed_line_bg(),
            Theme::removed_line_bg_color(),
        ),
        LineKind::Context => (" ", Theme::context_line(), Color::Reset),
    };

    let highlighted = highlight(&line, path);

    let mut spans = vec![
        Span::styled(old_num, Theme::line_number()),
        Span::styled("  ", Theme::line_number()),
        Span::styled(new_num, Theme::line_number()),
        Span::styled(format!(" {marker} "), style),
    ];

    spans.extend(
        highlighted
            .clone()
            .iter()
            .map(|span| span.clone().bg(bg_color)),
    );

    Line::from(spans)
}

pub fn render_sbs_row(
    row: &DisplayRow,
    files: &[DiffFile],
    half_width: u16,
    is_selected: bool,
) -> (Line<'static>, Line<'static>) {
    match row {
        DisplayRow::DiffLine { line, file_idx, .. } => {
            let path = files.get(*file_idx).map(|f| f.path.as_str()).unwrap_or("");
            render_sbs_diff_line(line, path, half_width, is_selected)
        }
        _ => {
            let unified = render_unified_row(row, files, half_width * 2, is_selected);
            (unified.clone(), Line::default())
        }
    }
}

fn truncate_spans(spans: &[Span<'static>], max_chars: usize) -> Vec<Span<'static>> {
    let mut remaining = max_chars;
    spans
        .iter()
        .filter_map(|span| {
            if remaining <= 0 {
                return None;
            }
            let taken = span.content.chars().take(remaining).collect::<String>();
            remaining = remaining.saturating_sub(taken.len());
            if taken.is_empty() {
                None
            } else {
                Some(Span::styled(taken, span.style))
            }
        })
        .collect()
}

fn render_sbs_diff_line(
    line: &DiffLine,
    path: &str,
    half_width: u16,
    is_selected: bool,
) -> (Line<'static>, Line<'static>) {
    let sel = if is_selected {
        Theme::selected_line()
    } else {
        Style::default()
    };

    let content_max = (half_width as usize).saturating_sub(LINE_NUM_WIDTH + 3);

    let highlighted = highlight(line, path);
    let truncated_spans = truncate_spans(highlighted.spans.as_slice(), content_max);

    match line.kind {
        LineKind::Context => {
            let num_l = line
                .old_lineno
                .map(|n| format!("{n:>LINE_NUM_WIDTH$}"))
                .unwrap_or(" ".repeat(LINE_NUM_WIDTH));
            let num_r = line
                .new_lineno
                .map(|n| format!("{n:>LINE_NUM_WIDTH$}"))
                .unwrap_or(" ".repeat(LINE_NUM_WIDTH));

            let style = Theme::context_line().patch(sel);
            let mut left_spans = vec![
                Span::styled(num_l, Theme::line_number().patch(sel)),
                Span::styled("  ", style),
            ];
            left_spans.extend(truncated_spans.clone());
            let mut right_spans = vec![
                Span::styled(num_r, Theme::line_number().patch(sel)),
                Span::styled("  ", style),
            ];
            right_spans.extend(truncated_spans);

            (Line::from(left_spans), Line::from(right_spans))
        }
        LineKind::Removed => {
            let num = line
                .old_lineno
                .map(|n| format!("{n:>LINE_NUM_WIDTH$}"))
                .unwrap_or(" ".repeat(LINE_NUM_WIDTH));
            let bg_color = Theme::removed_line_bg_color();
            let style = Theme::removed_line_bg().patch(sel);

            let colored_spans: Vec<Span<'static>> = truncated_spans
                .into_iter()
                .map(|s| Span::styled(s.content, s.style.bg(bg_color)))
                .collect();

            let mut left_spans = vec![
                Span::styled(num, Theme::line_number().patch(sel)),
                Span::styled(" -", style),
            ];
            left_spans.extend(colored_spans);

            (Line::from(left_spans), Line::default())
        }
        LineKind::Added => {
            let num = line
                .new_lineno
                .map(|n| format!("{n:>LINE_NUM_WIDTH$}"))
                .unwrap_or(" ".repeat(LINE_NUM_WIDTH));
            let bg_color = Theme::added_line_bg_color();
            let style = Theme::added_line_bg().patch(sel);

            let colored_spans: Vec<Span<'static>> = truncated_spans
                .into_iter()
                .map(|s| Span::styled(s.content, s.style.bg(bg_color)))
                .collect();

            let mut right_spans = vec![
                Span::styled(num, Theme::line_number().patch(sel)),
                Span::styled(" +", style),
            ];
            right_spans.extend(colored_spans);

            (Line::default(), Line::from(right_spans))
        }
    }
}
