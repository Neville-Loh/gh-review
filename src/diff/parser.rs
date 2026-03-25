use crate::types::{DiffLine, Hunk, LineKind};

/// Parse a GitHub patch string into structured hunks.
/// GitHub's API returns patches without the --- / +++ file headers,
/// starting directly with @@ hunk headers.
pub fn parse_patch(patch: &str) -> Vec<Hunk> {
    let mut hunks = Vec::new();
    let mut current_hunk: Option<HunkBuilder> = None;

    for line in patch.lines() {
        if line.starts_with("@@") {
            if let Some(builder) = current_hunk.take() {
                hunks.push(builder.build());
            }
            if let Some(builder) = parse_hunk_header(line) {
                current_hunk = Some(builder);
            }
        } else if let Some(ref mut builder) = current_hunk {
            builder.add_line(line);
        }
    }

    if let Some(builder) = current_hunk {
        hunks.push(builder.build());
    }

    hunks
}

struct HunkBuilder {
    header: String,
    old_start: usize,
    old_count: usize,
    new_start: usize,
    new_count: usize,
    lines: Vec<DiffLine>,
    old_lineno: usize,
    new_lineno: usize,
}

impl HunkBuilder {
    fn add_line(&mut self, raw: &str) {
        if let Some(stripped) = raw.strip_prefix('+') {
            self.lines.push(DiffLine {
                kind: LineKind::Added,
                old_lineno: None,
                new_lineno: Some(self.new_lineno),
                content: stripped.to_string(),
                highlighted_content: None,
            });
            self.new_lineno += 1;
        } else if let Some(stripped) = raw.strip_prefix('-') {
            self.lines.push(DiffLine {
                kind: LineKind::Removed,
                old_lineno: Some(self.old_lineno),
                new_lineno: None,
                content: stripped.to_string(),
                highlighted_content: None,
            });
            self.old_lineno += 1;
        } else if raw.starts_with('\\') {
            // "\ No newline at end of file" — skip
        } else {
            let content = raw.strip_prefix(' ').unwrap_or(raw).to_string();
            self.lines.push(DiffLine {
                kind: LineKind::Context,
                old_lineno: Some(self.old_lineno),
                new_lineno: Some(self.new_lineno),
                content,
                highlighted_content: None,
            });
            self.old_lineno += 1;
            self.new_lineno += 1;
        }
    }

    fn build(self) -> Hunk {
        Hunk {
            header: self.header,
            old_start: self.old_start,
            old_count: self.old_count,
            new_start: self.new_start,
            new_count: self.new_count,
            lines: self.lines,
        }
    }
}

/// Parse "@@ -10,5 +12,7 @@ optional context" into a HunkBuilder
fn parse_hunk_header(line: &str) -> Option<HunkBuilder> {
    let trimmed = line.trim_start_matches("@@ ");
    let parts: Vec<&str> = trimmed.splitn(2, " @@").collect();
    if parts.is_empty() {
        return None;
    }

    let ranges = parts[0];
    let range_parts: Vec<&str> = ranges.split_whitespace().collect();
    if range_parts.len() < 2 {
        return None;
    }

    let (old_start, old_count) = parse_range(range_parts[0].trim_start_matches('-'))?;
    let (new_start, new_count) = parse_range(range_parts[1].trim_start_matches('+'))?;

    Some(HunkBuilder {
        header: line.to_string(),
        old_start,
        old_count,
        new_start,
        new_count,
        lines: Vec::new(),
        old_lineno: old_start,
        new_lineno: new_start,
    })
}

fn parse_range(s: &str) -> Option<(usize, usize)> {
    let parts: Vec<&str> = s.split(',').collect();
    let start = parts.first()?.parse::<usize>().ok()?;
    let count = if parts.len() > 1 {
        parts[1].parse::<usize>().ok()?
    } else {
        1
    };
    Some((start, count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_patch() {
        let patch = "@@ -1,3 +1,4 @@\n context\n-removed\n+added\n+new line\n context";
        let hunks = parse_patch(patch);
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].old_start, 1);
        assert_eq!(hunks[0].new_start, 1);
        assert_eq!(hunks[0].lines.len(), 5);

        assert_eq!(hunks[0].lines[0].kind, LineKind::Context);
        assert_eq!(hunks[0].lines[1].kind, LineKind::Removed);
        assert_eq!(hunks[0].lines[2].kind, LineKind::Added);
        assert_eq!(hunks[0].lines[3].kind, LineKind::Added);
        assert_eq!(hunks[0].lines[4].kind, LineKind::Context);
    }

    #[test]
    fn test_parse_multiple_hunks() {
        let patch = "@@ -1,3 +1,3 @@\n context\n-old\n+new\n@@ -10,2 +10,3 @@\n ctx\n+added";
        let hunks = parse_patch(patch);
        assert_eq!(hunks.len(), 2);
        assert_eq!(hunks[0].old_start, 1);
        assert_eq!(hunks[1].old_start, 10);
    }

    #[test]
    fn test_line_numbers() {
        let patch = "@@ -5,3 +5,4 @@\n context\n-removed\n+added1\n+added2\n context";
        let hunks = parse_patch(patch);
        let lines = &hunks[0].lines;

        assert_eq!(lines[0].old_lineno, Some(5));
        assert_eq!(lines[0].new_lineno, Some(5));
        assert_eq!(lines[1].old_lineno, Some(6));
        assert_eq!(lines[1].new_lineno, None);
        assert_eq!(lines[2].old_lineno, None);
        assert_eq!(lines[2].new_lineno, Some(6));
        assert_eq!(lines[3].old_lineno, None);
        assert_eq!(lines[3].new_lineno, Some(7));
    }
}
