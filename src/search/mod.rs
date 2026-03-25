use ratatui::{
    style::Style,
    text::{Line, Span},
};
use regex::Regex;
use std::collections::HashSet;

use crate::diff::renderer::DisplayRow;
use crate::theme::Theme;

#[cfg(test)]
mod tests;

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
    pub(crate) match_rows: Vec<usize>,
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
                if let Some(text) = searchable_text(row)
                    && re.is_match(text)
                {
                    self.match_rows.push(idx);
                    self.match_set.insert(idx);
                }
            }
        }

        if let Some(mi) = self.current_match
            && mi >= self.match_rows.len()
        {
            self.current_match = if self.match_rows.is_empty() {
                None
            } else {
                Some(self.match_rows.len() - 1)
            };
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
        if let Some(ref regex) = self.regex
            && self.match_set.contains(&global_idx)
        {
            let is_current = self
                .current_match
                .is_some_and(|mi| self.match_rows.get(mi) == Some(&global_idx));
            return highlight_line(line, regex, is_current);
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
