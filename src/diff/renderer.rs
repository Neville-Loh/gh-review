use ratatui::{
    style::Style,
    text::{Line, Span},
};

use crate::theme::Theme;
use crate::types::{DiffFile, DiffLine, ExistingComment, LineKind, ReviewComment};

/// A renderable line in the diff view — may be a hunk header, diff line,
/// expand hint, comment, or file header.
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
    ExistingComment {
        author: String,
        body: String,
    },
    PendingComment {
        body: String,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum ExpandDirection {
    Up,
    Down,
}

/// Build the flat list of display rows from structured diff files.
pub fn build_display_rows(
    files: &[DiffFile],
    existing_comments: &[ExistingComment],
    pending_comments: &[ReviewComment],
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

                // Show existing comments anchored to this line
                let target_line = match line.kind {
                    LineKind::Added | LineKind::Context => line.new_lineno,
                    LineKind::Removed => line.old_lineno,
                };

                if let Some(lineno) = target_line {
                    for ec in existing_comments
                        .iter()
                        .filter(|c| c.path == file.path && c.line == Some(lineno))
                    {
                        rows.push(DisplayRow::ExistingComment {
                            author: ec.user.login.clone(),
                            body: ec.body.clone(),
                        });
                    }

                    for pc in pending_comments
                        .iter()
                        .filter(|c| c.path == file.path && c.line == lineno)
                    {
                        rows.push(DisplayRow::PendingComment {
                            body: pc.body.clone(),
                        });
                    }
                }
            }
        }
    }

    rows
}

const LINE_NUM_WIDTH: usize = 5;

/// Render a single display row as a unified diff line.
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
        DisplayRow::ExistingComment { author, body } => {
            let first_line = body.lines().next().unwrap_or("");
            Line::from(vec![
                Span::styled(
                    format!("{:>pad$}  {:>pad$} ", "", "", pad = LINE_NUM_WIDTH),
                    Theme::line_number(),
                ),
                Span::styled(format!("💬 {author}: "), Theme::comment_marker()),
                Span::styled(first_line.to_string(), Theme::comment_body()),
            ])
        }
        DisplayRow::PendingComment { body } => {
            let first_line = body.lines().next().unwrap_or("");
            Line::from(vec![
                Span::styled(
                    format!("{:>pad$}  {:>pad$} ", "", "", pad = LINE_NUM_WIDTH),
                    Theme::line_number(),
                ),
                Span::styled("📝 (pending) ", Theme::pending_count()),
                Span::styled(first_line.to_string(), Theme::comment_body()),
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

/// Render a single display row as side-by-side diff.
/// Returns (left_line, right_line) or a single spanning line.
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

fn render_sbs_diff_line(
    line: &DiffLine,
    _half_width: u16,
    is_selected: bool,
) -> (Line<'static>, Line<'static>) {
    let sel = if is_selected {
        Theme::selected_line()
    } else {
        Style::default()
    };

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
            let left = Line::from(vec![
                Span::styled(num_l, Theme::line_number().patch(sel)),
                Span::styled("  ", style),
                Span::styled(line.content.clone(), style),
            ]);
            let right = Line::from(vec![
                Span::styled(num_r, Theme::line_number().patch(sel)),
                Span::styled("  ", style),
                Span::styled(line.content.clone(), style),
            ]);
            (left, right)
        }
        LineKind::Removed => {
            let num = line
                .old_lineno
                .map(|n| format!("{n:>LINE_NUM_WIDTH$}"))
                .unwrap_or(" ".repeat(LINE_NUM_WIDTH));
            let style = Theme::removed_line_bg().patch(sel);
            let left = Line::from(vec![
                Span::styled(num, Theme::line_number().patch(sel)),
                Span::styled(" -", style),
                Span::styled(line.content.clone(), style),
            ]);
            let right = Line::default();
            (left, right)
        }
        LineKind::Added => {
            let num = line
                .new_lineno
                .map(|n| format!("{n:>LINE_NUM_WIDTH$}"))
                .unwrap_or(" ".repeat(LINE_NUM_WIDTH));
            let style = Theme::added_line_bg().patch(sel);
            let left = Line::default();
            let right = Line::from(vec![
                Span::styled(num, Theme::line_number().patch(sel)),
                Span::styled(" +", style),
                Span::styled(line.content.clone(), style),
            ]);
            (left, right)
        }
    }
}
