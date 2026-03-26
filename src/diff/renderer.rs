use ratatui::{
    style::{Color, Style, Stylize},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

use crate::theme::Theme;
use crate::types::{DiffFile, DiffLine, LineKind};

pub use super::model::{DisplayRow, build_display_rows};

const COMMENT_INDENT: &str = "              ";
const LINE_NUM_WIDTH: usize = 5;

const GUTTER_WIDTH: usize = LINE_NUM_WIDTH * 2 + 3;

pub fn render_unified_row(
    row: &DisplayRow,
    _files: &[DiffFile],
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
            is_resolved,
            expanded,
            reply_count,
            body_preview,
            is_reply,
            ..
        } => {
            let (bg, border_color, fg) = if *is_pending {
                (Theme::pending_bg(), Theme::pending_accent(), Theme::pending_fg())
            } else if *is_resolved {
                (Theme::resolved_bg(), Theme::resolved_accent(), Theme::resolved_fg())
            } else {
                (Theme::comment_bg(), Theme::comment_accent(), Theme::comment_fg())
            };
            let bs = Style::default().fg(border_color);
            let box_inner = (_width as usize).saturating_sub(GUTTER_WIDTH + 2);

            if *is_reply {
                let content = format!(" ↩ {author}");
                let cw = content.width();
                let pad = box_inner.saturating_sub(cw);
                Line::from(vec![
                    Span::styled(" ".repeat(GUTTER_WIDTH), Style::default()),
                    Span::styled("│", bs),
                    Span::styled(content, Style::default().fg(fg).bg(bg)),
                    Span::styled(" ".repeat(pad), Style::default().bg(bg)),
                    Span::styled("│", bs),
                ]).style(Style::default().bg(bg))
            } else {
                let toggle = if *expanded { "▼" } else { "▶" };
                let header = if *is_pending {
                    format!(" {toggle} 📝 pending  {body_preview}")
                } else {
                    let mut h = format!(" {toggle} 💬 {author}");
                    if *is_resolved {
                        h.push_str(" ✓ Resolved");
                    }
                    if *reply_count > 0 {
                        h.push_str(&format!(" ({reply_count} replies)"));
                    }
                    h
                };
                let hw = header.width();
                let fill = box_inner.saturating_sub(hw);

                if *expanded {
                    Line::from(vec![
                        Span::styled(" ".repeat(GUTTER_WIDTH), Style::default()),
                        Span::styled("┌", bs),
                        Span::styled(header, Style::default().fg(fg).bg(bg).add_modifier(ratatui::style::Modifier::BOLD)),
                        Span::styled(" ".repeat(fill), Style::default().bg(bg)),
                        Span::styled("┐", bs),
                    ]).style(Style::default().bg(bg))
                } else {
                    Line::from(vec![
                        Span::styled(" ".repeat(GUTTER_WIDTH), Style::default()),
                        Span::styled("╶", bs),
                        Span::styled(format!("{header} "), Style::default().fg(fg).bg(bg)),
                        Span::styled("─".repeat(fill.saturating_sub(1)), Style::default().fg(border_color).bg(bg)),
                        Span::styled("╴", bs),
                    ]).style(Style::default().bg(bg))
                }
            }
        }
        DisplayRow::CommentBodyLine { line, is_reply, is_resolved } => {
            let (bg, border_color) = if *is_resolved {
                (Theme::resolved_bg(), Theme::resolved_accent())
            } else {
                (Theme::comment_bg(), Theme::comment_accent())
            };
            let bs = Style::default().fg(border_color);
            let box_inner = (_width as usize).saturating_sub(GUTTER_WIDTH + 2);
            let prefix = if *is_reply { "   " } else { " " };

            let content_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            let content_w = prefix.width() + content_text.width();
            let pad = box_inner.saturating_sub(content_w);

            let mut spans = vec![
                Span::styled(" ".repeat(GUTTER_WIDTH), Style::default()),
                Span::styled("│", bs),
                Span::styled(prefix, Style::default().bg(bg)),
            ];
            for span in line.spans.iter() {
                spans.push(Span::styled(span.content.clone(), span.style.bg(bg)));
            }
            spans.push(Span::styled(" ".repeat(pad), Style::default().bg(bg)));
            spans.push(Span::styled("│", bs));
            Line::from(spans).style(Style::default().bg(bg))
        }
        DisplayRow::CommentFooter { is_resolved, .. } => {
            let (bg, border_color) = if *is_resolved {
                (Theme::resolved_bg(), Theme::resolved_accent())
            } else {
                (Theme::comment_bg(), Theme::comment_accent())
            };
            let bs = Style::default().fg(border_color);
            let box_inner = (_width as usize).saturating_sub(GUTTER_WIDTH + 2);
            Line::from(vec![
                Span::styled(" ".repeat(GUTTER_WIDTH), Style::default()),
                Span::styled("└", bs),
                Span::styled("─".repeat(box_inner), Style::default().fg(border_color).bg(bg)),
                Span::styled("┘", bs),
            ]).style(Style::default().bg(bg))
        }
        DisplayRow::SuggestionDiff {
            original, suggested, ..
        } => {
            let mut spans = vec![
                Span::styled(COMMENT_INDENT.to_string(), Theme::line_number()),
                Span::styled("  ✏ ", Theme::comment_marker()),
                Span::styled(original.clone(), Theme::suggestion_removed()),
                Span::styled(" → ", Theme::comment_marker()),
                Span::styled(suggested.clone(), Theme::suggestion_added()),
            ];
            if suggested.contains('\n') {
                spans = vec![
                    Span::styled(COMMENT_INDENT.to_string(), Theme::line_number()),
                    Span::styled("  ✏ suggestion (multi-line)", Theme::comment_marker()),
                ];
            }
            Line::from(spans)
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

fn render_unified_diff_line(line: &DiffLine) -> Line<'static> {
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

    let highlighted = line
        .highlighted_content
        .clone()
        .unwrap_or_else(|| Line::from(line.content.to_string()));

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
        DisplayRow::DiffLine { line, .. } => render_sbs_diff_line(line, half_width, is_selected),
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
            if remaining == 0 {
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
    half_width: u16,
    is_selected: bool,
) -> (Line<'static>, Line<'static>) {
    let sel = if is_selected {
        Theme::selected_line()
    } else {
        Style::default()
    };

    let content_max = (half_width as usize).saturating_sub(LINE_NUM_WIDTH + 3);

    let highlighted = line
        .highlighted_content
        .clone()
        .unwrap_or_else(|| Line::from(line.content.to_string()));
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
