use ratatui::{
    style::Style,
    text::{Line, Span},
};

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
    },
    CommentBodyLine {
        line: Line<'static>,
    },
    CommentFooter,
}

#[derive(Debug, Clone, Copy)]
pub enum ExpandDirection {
    Up,
    Down,
}

const COMMENT_INDENT: &str = "              ";
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
                                (Some("LEFT"), Side::Left)
                                | (Some("RIGHT"), Side::Right) => true,
                                (None, _) => true,
                                _ => false,
                            }
                    })
                    {
                        let cid = comment_id_counter;
                        comment_id_counter += 1;
                        let is_expanded = expanded_comments.contains(&cid);
                        let preview = ec.body.lines().next().unwrap_or("").to_string();
                        let body_lines = ec.body.lines().count();

                        rows.push(DisplayRow::CommentHeader {
                            author: ec.user.login.clone(),
                            is_pending: false,
                            comment_id: cid,
                            github_id: Some(ec.id),
                            expanded: is_expanded,
                            body_preview: preview,
                            body_lines,
                        });

                        if is_expanded {
                            let md_lines = render_markdown_to_lines(&ec.body);
                            for ml in md_lines {
                                rows.push(DisplayRow::CommentBodyLine { line: ml });
                            }
                            rows.push(DisplayRow::CommentFooter);
                        }
                    }

                    for pc in pending_comments
                        .iter()
                        .filter(|c| c.path == file.path && c.line == lineno && c.side == target_side)
                    {
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
                        });

                        if is_expanded {
                            let md_lines = render_markdown_to_lines(&pc.body);
                            for ml in md_lines {
                                rows.push(DisplayRow::CommentBodyLine { line: ml });
                            }
                            rows.push(DisplayRow::CommentFooter);
                        }
                    }
                }
            }
        }
    }

    rows
}

const LINE_NUM_WIDTH: usize = 5;

pub fn render_unified_row(row: &DisplayRow, _width: u16, is_selected: bool) -> Line<'static> {
    let base_line = match row {
        DisplayRow::FileHeader { path, .. } => {
            Line::from(vec![Span::styled(
                format!("─── {path} ───"),
                Theme::file_header(),
            )])
        }
        DisplayRow::HunkHeader { text, .. } => {
            Line::from(vec![Span::styled(text.clone(), Theme::hunk_header())])
        }
        DisplayRow::DiffLine { line, .. } => render_unified_diff_line(line),
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
            ..
        } => {
            let toggle = if *body_lines > 1 {
                if *expanded { "▼" } else { "▶" }
            } else {
                " "
            };

            if *is_pending {
                if *expanded {
                    Line::from(vec![
                        Span::styled(COMMENT_INDENT, Theme::line_number()),
                        Span::styled(
                            format!("{toggle} ┌─ 📝 pending "),
                            Theme::pending_count(),
                        ),
                        Span::styled(
                            "─".repeat(30),
                            Theme::pending_count(),
                        ),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(COMMENT_INDENT, Theme::line_number()),
                        Span::styled(
                            format!("{toggle} 📝 (pending) "),
                            Theme::pending_count(),
                        ),
                        Span::styled(body_preview.clone(), Theme::comment_body()),
                    ])
                }
            } else if *expanded {
                Line::from(vec![
                    Span::styled(COMMENT_INDENT, Theme::line_number()),
                    Span::styled(
                        format!("{toggle} ┌─ 💬 {author} "),
                        Theme::comment_marker(),
                    ),
                    Span::styled(
                        "─".repeat(30),
                        Theme::comment_marker(),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::styled(COMMENT_INDENT, Theme::line_number()),
                    Span::styled(
                        format!("{toggle} 💬 {author}: "),
                        Theme::comment_marker(),
                    ),
                    Span::styled(body_preview.clone(), Theme::comment_body()),
                ])
            }
        }
        DisplayRow::CommentBodyLine { line } => {
            let mut spans = vec![
                Span::styled(COMMENT_INDENT, Theme::line_number()),
                Span::styled("  │ ", Theme::comment_marker()),
            ];
            spans.extend(line.spans.iter().cloned());
            Line::from(spans)
        }
        DisplayRow::CommentFooter => {
            Line::from(vec![
                Span::styled(COMMENT_INDENT, Theme::line_number()),
                Span::styled(
                    format!("  └{}",  "─".repeat(34)),
                    Theme::comment_marker(),
                ),
            ])
        }
    };

    if is_selected {
        let mut spans = vec![Span::styled("▌", Theme::selected_cursor())];
        spans.extend(base_line.spans.into_iter().map(|s| {
            Span::styled(s.content, s.style.patch(Theme::selected_line()))
        }));
        Line::from(spans)
    } else {
        base_line
    }
}

fn render_unified_diff_line(line: &DiffLine) -> Line<'static> {
    let old_num = line
        .old_lineno
        .map(|n| format!("{n:>LINE_NUM_WIDTH$}"))
        .unwrap_or(" ".repeat(LINE_NUM_WIDTH));
    let new_num = line
        .new_lineno
        .map(|n| format!("{n:>LINE_NUM_WIDTH$}"))
        .unwrap_or(" ".repeat(LINE_NUM_WIDTH));

    let (marker, style) = match line.kind {
        LineKind::Added => ("+", Theme::added_line_bg()),
        LineKind::Removed => ("-", Theme::removed_line_bg()),
        LineKind::Context => (" ", Theme::context_line()),
    };

    Line::from(vec![
        Span::styled(old_num, Theme::line_number()),
        Span::styled("  ", Theme::line_number()),
        Span::styled(new_num, Theme::line_number()),
        Span::styled(format!(" {marker} "), style),
        Span::styled(line.content.clone(), style),
    ])
}

pub fn render_sbs_row(
    row: &DisplayRow,
    half_width: u16,
    is_selected: bool,
) -> (Line<'static>, Line<'static>) {
    match row {
        DisplayRow::DiffLine { line, .. } => render_sbs_diff_line(line, half_width, is_selected),
        _ => {
            let unified = render_unified_row(row, half_width * 2, is_selected);
            (unified.clone(), Line::default())
        }
    }
}

fn truncate_to_width(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{truncated}…")
    }
}

fn render_sbs_diff_line(
    line: &DiffLine,
    half_width: u16,
    is_selected: bool,
) -> (Line<'static>, Line<'static>) {
    let sel = if is_selected {
        Theme::selected_line()
    } else {
        Style::default()
    };

    let content_max = (half_width as usize).saturating_sub(LINE_NUM_WIDTH + 3);

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

            let content = truncate_to_width(&line.content, content_max);
            let style = Theme::context_line().patch(sel);
            let left = Line::from(vec![
                Span::styled(num_l, Theme::line_number().patch(sel)),
                Span::styled("  ", style),
                Span::styled(content.clone(), style),
            ]);
            let right = Line::from(vec![
                Span::styled(num_r, Theme::line_number().patch(sel)),
                Span::styled("  ", style),
                Span::styled(content, style),
            ]);
            (left, right)
        }
        LineKind::Removed => {
            let num = line
                .old_lineno
                .map(|n| format!("{n:>LINE_NUM_WIDTH$}"))
                .unwrap_or(" ".repeat(LINE_NUM_WIDTH));
            let content = truncate_to_width(&line.content, content_max);
            let style = Theme::removed_line_bg().patch(sel);
            let left = Line::from(vec![
                Span::styled(num, Theme::line_number().patch(sel)),
                Span::styled(" -", style),
                Span::styled(content, style),
            ]);
            let right = Line::default();
            (left, right)
        }
        LineKind::Added => {
            let num = line
                .new_lineno
                .map(|n| format!("{n:>LINE_NUM_WIDTH$}"))
                .unwrap_or(" ".repeat(LINE_NUM_WIDTH));
            let content = truncate_to_width(&line.content, content_max);
            let style = Theme::added_line_bg().patch(sel);
            let left = Line::default();
            let right = Line::from(vec![
                Span::styled(num, Theme::line_number().patch(sel)),
                Span::styled(" +", style),
                Span::styled(content, style),
            ]);
            (left, right)
        }
    }
}

