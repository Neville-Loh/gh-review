use ratatui::{
    style::Style,
    text::{Line, Span},
};
use regex::Regex;
use std::collections::HashSet;

use crate::diff::renderer::DisplayRow;
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchDirection {
    Forward,
    Backward,
}

/// Search engine that tracks matches over a set of display rows.
///
/// Owns the compiled regex, match indices, and the "current" match cursor.
/// Does **not** own the display rows — callers pass `&[DisplayRow]` when
/// the match set needs to be (re)computed.
pub struct SearchState {
    regex: Option<Regex>,
    match_rows: Vec<usize>,
    match_set: HashSet<usize>,
    current_match: Option<usize>,
    anchor: Option<usize>,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            regex: None,
            match_rows: Vec::new(),
            match_set: HashSet::new(),
            current_match: None,
            anchor: None,
        }
    }

    // --- Anchor (pre-search cursor position) ---

    pub fn set_anchor(&mut self, cursor: usize) {
        self.anchor = Some(cursor);
    }

    pub fn anchor(&self) -> Option<usize> {
        self.anchor
    }

    // --- Apply / recompute ---

    /// Compile `pattern`, find all matching rows, and position `current_match`
    /// relative to the anchor (or `fallback_cursor`).
    ///
    /// Uses smart-case: case-insensitive unless the pattern contains uppercase.
    /// Invalid regex is silently escaped to a literal.
    ///
    /// Returns the cursor position of the first hit, or `None` when the
    /// pattern is empty (search cleared) or no matches exist.
    pub fn apply(
        &mut self,
        pattern: &str,
        direction: SearchDirection,
        rows: &[DisplayRow],
        fallback_cursor: usize,
    ) -> Option<usize> {
        if pattern.is_empty() {
            self.clear();
            return None;
        }

        let case_insensitive = pattern.chars().all(|c| !c.is_uppercase());
        let effective = if case_insensitive {
            format!("(?i){pattern}")
        } else {
            pattern.to_string()
        };

        let regex = Regex::new(&effective).or_else(|_| Regex::new(&regex::escape(pattern)));

        match regex {
            Ok(re) => {
                self.regex = Some(re);
                self.recompute(rows);

                let anchor = self.anchor.unwrap_or(fallback_cursor);
                self.current_match = match direction {
                    SearchDirection::Forward => {
                        self.match_rows.iter().position(|&r| r >= anchor).or(
                            if !self.match_rows.is_empty() {
                                Some(0)
                            } else {
                                None
                            },
                        )
                    }
                    SearchDirection::Backward => {
                        self.match_rows.iter().rposition(|&r| r <= anchor).or(
                            if !self.match_rows.is_empty() {
                                Some(self.match_rows.len() - 1)
                            } else {
                                None
                            },
                        )
                    }
                };

                self.current_match
                    .and_then(|i| self.match_rows.get(i).copied())
            }
            Err(_) => None,
        }
    }

    /// Re-scan `rows` with the existing regex (e.g. after display rows are rebuilt).
    pub fn recompute(&mut self, rows: &[DisplayRow]) {
        self.match_rows.clear();
        self.match_set.clear();

        if let Some(ref re) = self.regex {
            for (idx, row) in rows.iter().enumerate() {
                if let Some(text) = searchable_text(row) {
                    if re.is_match(text) {
                        self.match_rows.push(idx);
                        self.match_set.insert(idx);
                    }
                }
            }
        }

        if let Some(mi) = self.current_match {
            if mi >= self.match_rows.len() {
                self.current_match = if self.match_rows.is_empty() {
                    None
                } else {
                    Some(self.match_rows.len() - 1)
                };
            }
        }
    }

    // --- Navigation ---

    /// Advance to the next match (forward / down). Returns new cursor position.
    pub fn next_match(&mut self) -> Option<usize> {
        if self.match_rows.is_empty() {
            return None;
        }
        let idx = match self.current_match {
            Some(i) => (i + 1) % self.match_rows.len(),
            None => 0,
        };
        self.current_match = Some(idx);
        self.match_rows.get(idx).copied()
    }

    /// Advance to the previous match (backward / up). Returns new cursor position.
    pub fn prev_match(&mut self) -> Option<usize> {
        if self.match_rows.is_empty() {
            return None;
        }
        let idx = match self.current_match {
            Some(0) | None => self.match_rows.len() - 1,
            Some(i) => i - 1,
        };
        self.current_match = Some(idx);
        self.match_rows.get(idx).copied()
    }

    // --- Query ---

    pub fn clear(&mut self) {
        self.regex = None;
        self.match_rows.clear();
        self.match_set.clear();
        self.current_match = None;
        self.anchor = None;
    }

    /// `(current_index, total_matches)`
    pub fn match_info(&self) -> (usize, usize) {
        (self.current_match.unwrap_or(0), self.match_rows.len())
    }

    /// True when a compiled regex exists **and** at least one row matches.
    pub fn is_active(&self) -> bool {
        self.regex.is_some() && !self.match_rows.is_empty()
    }

    // --- Rendering helpers ---

    /// If `global_idx` is a matched row, return the line with highlighted spans;
    /// otherwise return the line unchanged.
    pub fn highlight(&self, line: Line<'static>, global_idx: usize) -> Line<'static> {
        if let Some(ref regex) = self.regex {
            if self.match_set.contains(&global_idx) {
                let is_current = self
                    .current_match
                    .map_or(false, |mi| self.match_rows.get(mi) == Some(&global_idx));
                return highlight_line(line, regex, is_current);
            }
        }
        line
    }
}

// --- Free functions ---

/// Extract the searchable text content from a display row.
pub fn searchable_text(row: &DisplayRow) -> Option<&str> {
    match row {
        DisplayRow::FileHeader { path, .. } => Some(path),
        DisplayRow::HunkHeader { text, .. } => Some(text),
        DisplayRow::DiffLine { line, .. } => Some(&line.content),
        _ => None,
    }
}

fn highlight_line(line: Line<'static>, regex: &Regex, is_current: bool) -> Line<'static> {
    let match_style = if is_current {
        Theme::search_current()
    } else {
        Theme::search_match()
    };

    let new_spans: Vec<Span> = line
        .spans
        .into_iter()
        .flat_map(|span| split_span_at_matches(span, regex, match_style))
        .collect();

    Line::from(new_spans)
}

fn split_span_at_matches(
    span: Span<'static>,
    regex: &Regex,
    match_style: Style,
) -> Vec<Span<'static>> {
    let text = span.content.to_string();
    let base_style = span.style;

    let matches: Vec<regex::Match> = regex.find_iter(&text).collect();
    if matches.is_empty() {
        return vec![span];
    }

    let mut result = Vec::new();
    let mut last_end = 0;

    for m in &matches {
        if m.start() > last_end {
            result.push(Span::styled(
                text[last_end..m.start()].to_string(),
                base_style,
            ));
        }
        result.push(Span::styled(
            text[m.start()..m.end()].to_string(),
            base_style.patch(match_style),
        ));
        last_end = m.end();
    }

    if last_end < text.len() {
        result.push(Span::styled(text[last_end..].to_string(), base_style));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DiffLine, LineKind};
    use ratatui::style::Color;

    fn diff_row(content: &str) -> DisplayRow {
        DisplayRow::DiffLine {
            line: DiffLine {
                kind: LineKind::Added,
                old_lineno: None,
                new_lineno: Some(1),
                content: content.to_string(),
                highlighted_content: None,
            },
            file_idx: 0,
            hunk_idx: 0,
            line_idx: 0,
        }
    }

    fn file_header(path: &str) -> DisplayRow {
        DisplayRow::FileHeader {
            path: path.to_string(),
            file_idx: 0,
        }
    }

    fn hunk_header(text: &str) -> DisplayRow {
        DisplayRow::HunkHeader {
            text: text.to_string(),
            file_idx: 0,
        }
    }

    fn sample_rows() -> Vec<DisplayRow> {
        vec![
            file_header("src/main.rs"),
            hunk_header("@@ -1,5 +1,6 @@"),
            diff_row("use std::io;"),
            diff_row("fn main() {"),
            diff_row("    println!(\"hello world\");"),
            diff_row("}"),
            file_header("src/lib.rs"),
            hunk_header("@@ -10,3 +10,4 @@"),
            diff_row("pub fn hello() -> String {"),
            diff_row("    \"hello\".to_string()"),
            diff_row("}"),
        ]
    }

    // --- searchable_text ---

    #[test]
    fn searchable_text_extracts_file_header_path() {
        let row = file_header("src/app.rs");
        assert_eq!(searchable_text(&row), Some("src/app.rs"));
    }

    #[test]
    fn searchable_text_extracts_hunk_header() {
        let row = hunk_header("@@ -1,5 +1,6 @@");
        assert_eq!(searchable_text(&row), Some("@@ -1,5 +1,6 @@"));
    }

    #[test]
    fn searchable_text_extracts_diff_line_content() {
        let row = diff_row("let x = 42;");
        assert_eq!(searchable_text(&row), Some("let x = 42;"));
    }

    #[test]
    fn searchable_text_returns_none_for_comment_rows() {
        let row = DisplayRow::CommentFooter { is_reply: false };
        assert_eq!(searchable_text(&row), None);
    }

    // --- SearchState::new ---

    #[test]
    fn new_state_is_inactive() {
        let s = SearchState::new();
        assert!(!s.is_active());
        assert_eq!(s.match_info(), (0, 0));
        assert_eq!(s.anchor(), None);
    }

    // --- apply: basic forward ---

    #[test]
    fn apply_forward_finds_matches() {
        let rows = sample_rows();
        let mut s = SearchState::new();
        let cursor = s.apply("hello", SearchDirection::Forward, &rows, 0);
        assert!(cursor.is_some());
        assert!(s.is_active());
        let (_, total) = s.match_info();
        assert_eq!(total, 3); // "hello world", "hello()", "hello".to_string()
    }

    #[test]
    fn apply_forward_lands_on_first_match_at_or_after_cursor() {
        let rows = sample_rows();
        let mut s = SearchState::new();

        // cursor=0 → first match is row 4 ("hello world")
        let cursor = s.apply("hello", SearchDirection::Forward, &rows, 0);
        assert_eq!(cursor, Some(4));

        // cursor=5 → next match is row 8 ("hello()")
        let mut s2 = SearchState::new();
        let cursor2 = s2.apply("hello", SearchDirection::Forward, &rows, 5);
        assert_eq!(cursor2, Some(8));
    }

    // --- apply: backward ---

    #[test]
    fn apply_backward_lands_on_last_match_at_or_before_cursor() {
        let rows = sample_rows();
        let mut s = SearchState::new();

        // cursor at end → last match
        let cursor = s.apply("hello", SearchDirection::Backward, &rows, 10);
        assert_eq!(cursor, Some(9)); // "hello".to_string()
    }

    // --- apply: anchor ---

    #[test]
    fn apply_uses_anchor_over_fallback_cursor() {
        let rows = sample_rows();
        let mut s = SearchState::new();
        s.set_anchor(6); // anchor at file_header "src/lib.rs"

        // fallback_cursor=0, but anchor=6, so forward search starts at row 6
        let cursor = s.apply("hello", SearchDirection::Forward, &rows, 0);
        assert_eq!(cursor, Some(8)); // "hello()" in lib.rs
    }

    // --- apply: smart-case ---

    #[test]
    fn apply_case_insensitive_when_pattern_all_lowercase() {
        let rows = vec![diff_row("Hello World"), diff_row("hello world")];
        let mut s = SearchState::new();
        s.apply("hello", SearchDirection::Forward, &rows, 0);
        assert_eq!(s.match_info().1, 2); // matches both
    }

    #[test]
    fn apply_case_sensitive_when_pattern_has_uppercase() {
        let rows = vec![diff_row("Hello World"), diff_row("hello world")];
        let mut s = SearchState::new();
        s.apply("Hello", SearchDirection::Forward, &rows, 0);
        assert_eq!(s.match_info().1, 1); // only "Hello World"
    }

    // --- apply: regex ---

    #[test]
    fn apply_supports_regex_patterns() {
        let rows = vec![diff_row("foo123"), diff_row("bar456"), diff_row("foo789")];
        let mut s = SearchState::new();
        s.apply("foo\\d+", SearchDirection::Forward, &rows, 0);
        assert_eq!(s.match_info().1, 2);
    }

    #[test]
    fn apply_falls_back_to_literal_on_invalid_regex() {
        let rows = vec![diff_row("value is [ok]"), diff_row("nothing here")];
        let mut s = SearchState::new();
        let cursor = s.apply("[ok]", SearchDirection::Forward, &rows, 0);
        // "[ok]" is valid regex matching o or k — matches both rows
        // but if it were escaped literal "[ok]", only first row matches
        // Since "[ok]" is valid regex, it matches "o" or "k" in both
        assert!(cursor.is_some());
        assert!(s.is_active());
    }

    #[test]
    fn apply_escapes_truly_invalid_regex() {
        let rows = vec![diff_row("a]b[c"), diff_row("nothing")];
        let mut s = SearchState::new();
        // "]b[" is invalid regex — falls back to literal
        let cursor = s.apply("]b[", SearchDirection::Forward, &rows, 0);
        assert_eq!(cursor, Some(0));
        assert_eq!(s.match_info().1, 1);
    }

    // --- apply: empty pattern ---

    #[test]
    fn apply_empty_pattern_clears_search() {
        let rows = sample_rows();
        let mut s = SearchState::new();
        s.apply("hello", SearchDirection::Forward, &rows, 0);
        assert!(s.is_active());

        let cursor = s.apply("", SearchDirection::Forward, &rows, 0);
        assert_eq!(cursor, None);
        assert!(!s.is_active());
    }

    // --- apply: no matches ---

    #[test]
    fn apply_no_matches_returns_none() {
        let rows = sample_rows();
        let mut s = SearchState::new();
        let cursor = s.apply("zzzznotfound", SearchDirection::Forward, &rows, 0);
        assert_eq!(cursor, None);
        assert!(!s.is_active()); // regex exists but no matches → not active
    }

    // --- apply: forward wraps when cursor past last match ---

    #[test]
    fn apply_forward_wraps_to_first_match() {
        let rows = vec![diff_row("match here"), diff_row("no"), diff_row("no")];
        let mut s = SearchState::new();
        let cursor = s.apply("match", SearchDirection::Forward, &rows, 2);
        // cursor=2 is past the only match at row 0, wraps to 0
        assert_eq!(cursor, Some(0));
    }

    // --- next_match / prev_match ---

    #[test]
    fn next_match_cycles_forward() {
        let rows = sample_rows();
        let mut s = SearchState::new();
        s.apply("hello", SearchDirection::Forward, &rows, 0);
        // 3 matches: rows 4, 8, 9

        let first = s.match_info().0;
        assert_eq!(s.match_rows[first], 4);

        assert_eq!(s.next_match(), Some(8));
        assert_eq!(s.next_match(), Some(9));
        assert_eq!(s.next_match(), Some(4)); // wraps
    }

    #[test]
    fn prev_match_cycles_backward() {
        let rows = sample_rows();
        let mut s = SearchState::new();
        s.apply("hello", SearchDirection::Forward, &rows, 0);
        // starts at match index 0 (row 4)

        assert_eq!(s.prev_match(), Some(9)); // wraps to last
        assert_eq!(s.prev_match(), Some(8));
        assert_eq!(s.prev_match(), Some(4)); // back to first
    }

    #[test]
    fn next_match_returns_none_when_no_matches() {
        let mut s = SearchState::new();
        assert_eq!(s.next_match(), None);
    }

    #[test]
    fn prev_match_returns_none_when_no_matches() {
        let mut s = SearchState::new();
        assert_eq!(s.prev_match(), None);
    }

    // --- clear ---

    #[test]
    fn clear_resets_all_state() {
        let rows = sample_rows();
        let mut s = SearchState::new();
        s.set_anchor(5);
        s.apply("hello", SearchDirection::Forward, &rows, 0);
        assert!(s.is_active());

        s.clear();
        assert!(!s.is_active());
        assert_eq!(s.match_info(), (0, 0));
        assert_eq!(s.anchor(), None);
    }

    // --- recompute ---

    #[test]
    fn recompute_updates_matches_after_row_change() {
        let rows = vec![diff_row("hello"), diff_row("world")];
        let mut s = SearchState::new();
        s.apply("hello", SearchDirection::Forward, &rows, 0);
        assert_eq!(s.match_info().1, 1);

        // Simulate rows changing (e.g. after comment added)
        let new_rows = vec![
            diff_row("hello"),
            diff_row("world"),
            diff_row("hello again"),
        ];
        s.recompute(&new_rows);
        assert_eq!(s.match_info().1, 2);
    }

    #[test]
    fn recompute_clamps_current_match_if_out_of_bounds() {
        let rows = vec![diff_row("a"), diff_row("a"), diff_row("a")];
        let mut s = SearchState::new();
        s.apply("a", SearchDirection::Forward, &rows, 0);
        s.next_match(); // index 1
        s.next_match(); // index 2

        // Now recompute with fewer matching rows
        let fewer = vec![diff_row("a"), diff_row("b")];
        s.recompute(&fewer);
        assert_eq!(s.match_info().1, 1);
        // current_match was 2, clamped to 0
        assert_eq!(s.match_info().0, 0);
    }

    // --- highlight ---

    #[test]
    fn highlight_leaves_non_match_rows_unchanged() {
        let rows = vec![diff_row("hello"), diff_row("world")];
        let mut s = SearchState::new();
        s.apply("hello", SearchDirection::Forward, &rows, 0);

        let line = Line::from("world");
        let result = s.highlight(line.clone(), 1); // row 1 is not a match
        assert_eq!(result.spans.len(), line.spans.len());
    }

    #[test]
    fn highlight_splits_matched_row_spans() {
        let rows = vec![diff_row("hello world")];
        let mut s = SearchState::new();
        s.apply("world", SearchDirection::Forward, &rows, 0);

        let line = Line::from(Span::styled("hello world", Style::default()));
        let result = s.highlight(line, 0);
        // Should split into "hello ", "world" (highlighted)
        assert!(result.spans.len() >= 2);
        let texts: Vec<&str> = result.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(texts.contains(&"world"));
    }

    #[test]
    fn highlight_current_match_gets_different_style() {
        let rows = vec![diff_row("aaa"), diff_row("aaa")];
        let mut s = SearchState::new();
        s.apply("aaa", SearchDirection::Forward, &rows, 0);
        // current_match is index 0 → row 0

        let line0 = Line::from(Span::styled("aaa", Style::default()));
        let line1 = Line::from(Span::styled("aaa", Style::default()));

        let r0 = s.highlight(line0, 0); // current match
        let r1 = s.highlight(line1, 1); // non-current match

        // Both are highlighted but with different styles
        let style0 = r0.spans[0].style;
        let style1 = r1.spans[0].style;
        assert_ne!(style0, style1);
    }

    // --- split_span_at_matches ---

    #[test]
    fn split_span_no_match_returns_original() {
        let span = Span::styled("no match here", Style::default());
        let re = Regex::new("xyz").unwrap();
        let result = split_span_at_matches(span.clone(), &re, Style::default().fg(Color::Red));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].content.as_ref(), "no match here");
    }

    #[test]
    fn split_span_match_at_start() {
        let span = Span::styled("hello world", Style::default());
        let re = Regex::new("hello").unwrap();
        let hl = Style::default().fg(Color::Red);
        let result = split_span_at_matches(span, &re, hl);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].content.as_ref(), "hello");
        assert_eq!(result[1].content.as_ref(), " world");
    }

    #[test]
    fn split_span_match_at_end() {
        let span = Span::styled("hello world", Style::default());
        let re = Regex::new("world").unwrap();
        let hl = Style::default().fg(Color::Red);
        let result = split_span_at_matches(span, &re, hl);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].content.as_ref(), "hello ");
        assert_eq!(result[1].content.as_ref(), "world");
    }

    #[test]
    fn split_span_match_in_middle() {
        let span = Span::styled("say hello friend", Style::default());
        let re = Regex::new("hello").unwrap();
        let hl = Style::default().fg(Color::Red);
        let result = split_span_at_matches(span, &re, hl);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].content.as_ref(), "say ");
        assert_eq!(result[1].content.as_ref(), "hello");
        assert_eq!(result[2].content.as_ref(), " friend");
    }

    #[test]
    fn split_span_multiple_matches() {
        let span = Span::styled("aXbXc", Style::default());
        let re = Regex::new("X").unwrap();
        let hl = Style::default().fg(Color::Red);
        let result = split_span_at_matches(span, &re, hl);
        // "a", "X", "b", "X", "c"
        assert_eq!(result.len(), 5);
        assert_eq!(result[0].content.as_ref(), "a");
        assert_eq!(result[1].content.as_ref(), "X");
        assert_eq!(result[2].content.as_ref(), "b");
        assert_eq!(result[3].content.as_ref(), "X");
        assert_eq!(result[4].content.as_ref(), "c");
    }

    #[test]
    fn split_span_entire_string_matches() {
        let span = Span::styled("hello", Style::default());
        let re = Regex::new("hello").unwrap();
        let hl = Style::default().fg(Color::Red);
        let result = split_span_at_matches(span, &re, hl);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].content.as_ref(), "hello");
        assert_ne!(result[0].style, Style::default()); // has highlight applied
    }

    // --- searches across row types ---

    #[test]
    fn search_matches_file_headers() {
        let rows = vec![
            file_header("src/main.rs"),
            diff_row("some code"),
            file_header("src/lib.rs"),
        ];
        let mut s = SearchState::new();
        s.apply("lib", SearchDirection::Forward, &rows, 0);
        assert_eq!(s.match_info().1, 1);
        assert_eq!(s.next_match(), Some(2)); // wraps from current → next is index 0 which wraps to itself, then goes again
    }

    #[test]
    fn search_matches_hunk_headers() {
        let rows = vec![hunk_header("@@ -1,5 +1,6 @@ fn main"), diff_row("code")];
        let mut s = SearchState::new();
        s.apply("fn main", SearchDirection::Forward, &rows, 0);
        assert_eq!(s.match_info().1, 1);
    }

    #[test]
    fn search_skips_comment_footer() {
        let rows = vec![
            diff_row("real match"),
            DisplayRow::CommentFooter { is_reply: false },
        ];
        let mut s = SearchState::new();
        s.apply("match", SearchDirection::Forward, &rows, 0);
        assert_eq!(s.match_info().1, 1); // only the diff row
    }

    // --- edge case: single row ---

    #[test]
    fn single_match_next_prev_return_same() {
        let rows = vec![diff_row("only one")];
        let mut s = SearchState::new();
        s.apply("only", SearchDirection::Forward, &rows, 0);
        assert_eq!(s.next_match(), Some(0));
        assert_eq!(s.next_match(), Some(0));
        assert_eq!(s.prev_match(), Some(0));
    }
}
