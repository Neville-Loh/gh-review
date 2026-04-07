use ratatui::style::Color;
use ratatui::text::{Line, Span};

use crate::highlight::highlight_content;
use crate::theme::Theme;
use crate::types::{DiffLine, LineKind};

use super::model::DisplayRow;

pub fn extract(body: &str) -> Option<String> {
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

pub fn strip_block(body: &str) -> String {
    let mut result = Vec::new();
    let mut in_suggestion = false;
    for line in body.lines() {
        if !in_suggestion
            && (line.trim() == "```suggestion" || line.trim().starts_with("```suggestion "))
        {
            in_suggestion = true;
            continue;
        }
        if in_suggestion {
            if line.trim() == "```" {
                in_suggestion = false;
            }
            continue;
        }
        result.push(line);
    }
    result.join("\n")
}

/// Collect original lines for a suggestion from the hunk, handling multi-line ranges.
pub fn collect_original_lines<'a>(
    hunk_lines: &'a [DiffLine],
    current_line: &'a DiffLine,
    lineno: usize,
    start_line: Option<usize>,
) -> Vec<&'a str> {
    let strip = |s: &'a str| -> &'a str { s.strip_prefix(' ').unwrap_or(s) };

    if let Some(start) = start_line {
        hunk_lines
            .iter()
            .filter(|hl| {
                let ln = match hl.kind {
                    LineKind::Added | LineKind::Context => hl.new_lineno,
                    LineKind::Removed => hl.old_lineno,
                };
                ln.is_some_and(|n| n >= start && n <= lineno)
            })
            .map(|hl| strip(&hl.content))
            .collect()
    } else {
        vec![strip(&current_line.content)]
    }
}

/// Build highlighted suggestion DisplayRows for an expanded comment box.
pub fn build_rows(
    file_path: &str,
    original_lines: &[&str],
    suggested: &str,
    is_resolved: bool,
) -> Vec<DisplayRow> {
    let (orig_hl, sug_hl) = highlight_block(
        file_path,
        original_lines,
        suggested,
        Theme::suggestion_removed_bg(),
        Theme::suggestion_removed_highlight_bg(),
        Theme::suggestion_added_bg(),
        Theme::suggestion_added_highlight_bg(),
    );

    let mut rows = Vec::new();

    rows.push(DisplayRow::CommentBodyLine {
        line: Line::from(Span::styled("✏ Suggested change:", Theme::comment_marker())),
        is_reply: false,
        is_resolved,
        is_pending: false,
        is_suggestion: true,
    });

    for hl_line in orig_hl {
        let mut spans = vec![Span::styled("- ", Theme::suggestion_removed())];
        spans.extend(hl_line.spans);
        rows.push(DisplayRow::CommentBodyLine {
            line: Line::from(spans),
            is_reply: false,
            is_resolved,
            is_pending: false,
            is_suggestion: true,
        });
    }

    for hl_line in sug_hl {
        let mut spans = vec![Span::styled("+ ", Theme::suggestion_added())];
        spans.extend(hl_line.spans);
        rows.push(DisplayRow::CommentBodyLine {
            line: Line::from(spans),
            is_reply: false,
            is_resolved,
            is_pending: false,
            is_suggestion: true,
        });
    }

    rows
}

// --- Internal helpers ---

fn highlight_block(
    path: &str,
    original_lines: &[&str],
    suggested_text: &str,
    removed_bg: Color,
    removed_highlight_bg: Color,
    added_bg: Color,
    added_highlight_bg: Color,
) -> (Vec<Line<'static>>, Vec<Line<'static>>) {
    let original_joined = original_lines.join("\n");
    let suggested_lines_raw: Vec<&str> = suggested_text.lines().collect();

    let orig_highlighted = highlight_content(path, &original_joined);
    let sug_highlighted = highlight_content(path, suggested_text);

    let max_pairs = original_lines.len().min(suggested_lines_raw.len());

    let mut orig_result: Vec<Line<'static>> = Vec::new();
    for (i, hl) in orig_highlighted.iter().enumerate() {
        if i < max_pairs {
            orig_result.push(apply_char_diff(
                hl,
                original_lines[i],
                suggested_lines_raw[i],
                removed_bg,
                removed_highlight_bg,
            ));
        } else {
            orig_result.push(overlay_bg(hl, removed_bg));
        }
    }

    let mut sug_result: Vec<Line<'static>> = Vec::new();
    for (i, hl) in sug_highlighted.iter().enumerate() {
        if i < max_pairs {
            sug_result.push(apply_char_diff(
                hl,
                suggested_lines_raw[i],
                original_lines[i],
                added_bg,
                added_highlight_bg,
            ));
        } else {
            sug_result.push(overlay_bg(hl, added_bg));
        }
    }

    (orig_result, sug_result)
}

fn overlay_bg(line: &Line<'static>, bg: Color) -> Line<'static> {
    Line::from(
        line.spans
            .iter()
            .map(|s| Span::styled(s.content.clone(), s.style.bg(bg)))
            .collect::<Vec<_>>(),
    )
}

fn apply_char_diff(
    line: &Line<'static>,
    raw_text: &str,
    other_text: &str,
    base_bg: Color,
    highlight_bg: Color,
) -> Line<'static> {
    let (prefix_len, suffix_len) = common_affixes(raw_text, other_text);
    let change_start = prefix_len;
    let change_end = raw_text.len().saturating_sub(suffix_len);

    if change_start >= change_end {
        return overlay_bg(line, base_bg);
    }

    let mut result: Vec<Span<'static>> = Vec::new();
    let mut char_pos = 0;

    for span in &line.spans {
        let span_len = span.content.len();
        let span_end = char_pos + span_len;

        if span_end <= change_start || char_pos >= change_end {
            result.push(Span::styled(span.content.clone(), span.style.bg(base_bg)));
        } else if char_pos >= change_start && span_end <= change_end {
            result.push(Span::styled(
                span.content.clone(),
                span.style.bg(highlight_bg),
            ));
        } else {
            let content = span.content.as_ref();
            let rel_start = change_start.saturating_sub(char_pos);
            let rel_end = (change_end - char_pos).min(span_len);

            if rel_start > 0 {
                result.push(Span::styled(
                    content[..rel_start].to_string(),
                    span.style.bg(base_bg),
                ));
            }
            result.push(Span::styled(
                content[rel_start..rel_end].to_string(),
                span.style.bg(highlight_bg),
            ));
            if rel_end < span_len {
                result.push(Span::styled(
                    content[rel_end..].to_string(),
                    span.style.bg(base_bg),
                ));
            }
        }
        char_pos = span_end;
    }

    Line::from(result)
}

fn common_affixes(a: &str, b: &str) -> (usize, usize) {
    let prefix: usize = a
        .chars()
        .zip(b.chars())
        .take_while(|(x, y)| x == y)
        .map(|(c, _)| c.len_utf8())
        .sum();

    let a_rem = &a[prefix..];
    let b_rem = &b[prefix..];
    let suffix: usize = a_rem
        .chars()
        .rev()
        .zip(b_rem.chars().rev())
        .take_while(|(x, y)| x == y)
        .map(|(c, _)| c.len_utf8())
        .sum();

    (prefix, suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_affixes_ascii() {
        assert_eq!(common_affixes("hello world", "hello rust!"), (6, 0));
        assert_eq!(common_affixes("abcXdef", "abcYdef"), (3, 3));
    }

    #[test]
    fn common_affixes_multibyte_no_panic() {
        let a = "┌──────────────────────────────────────────────┐";
        let b = "┌──────────────────────────────────────────────┘";
        let (prefix, suffix) = common_affixes(a, b);
        assert!(a.is_char_boundary(prefix));
        assert!(a.is_char_boundary(a.len() - suffix));
        let _ = &a[prefix..];
        let _ = &b[prefix..];
    }

    #[test]
    fn common_affixes_identical() {
        let s = "── same ──";
        let (prefix, suffix) = common_affixes(s, s);
        assert_eq!(prefix + suffix, s.len());
    }
}
