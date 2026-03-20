use crate::types::{DiffLine, Hunk, LineKind};

/// Expand context around a hunk by splicing in lines from the full file content.
/// `count` is the number of extra lines to show in each direction.
pub fn expand_hunk_context(
    hunk: &mut Hunk,
    base_lines: &[&str],
    head_lines: &[&str],
    count: usize,
) {
    expand_before(hunk, base_lines, head_lines, count);
    expand_after(hunk, base_lines, head_lines, count);
}

fn expand_before(hunk: &mut Hunk, base_lines: &[&str], _head_lines: &[&str], count: usize) {
    if hunk.old_start <= 1 && hunk.new_start <= 1 {
        return;
    }

    let first_old = hunk.old_start;
    let first_new = hunk.new_start;

    let expand_start_old = first_old.saturating_sub(count);
    let expand_start_new = first_new.saturating_sub(count);
    let expand_start = expand_start_old.max(1);

    let mut new_lines = Vec::new();
    for i in expand_start..first_old {
        let old_idx = i - 1;
        let content = base_lines
            .get(old_idx)
            .copied()
            .unwrap_or("")
            .to_string();
        new_lines.push(DiffLine {
            kind: LineKind::Context,
            old_lineno: Some(i),
            new_lineno: Some(expand_start_new + (i - expand_start)),
            content,
        });
    }

    new_lines.append(&mut hunk.lines);
    hunk.lines = new_lines;
    hunk.old_start = expand_start;
    hunk.new_start = expand_start_new.max(1);
}

fn expand_after(hunk: &mut Hunk, base_lines: &[&str], _head_lines: &[&str], count: usize) {
    let last_line = hunk.lines.last();
    let (last_old, last_new) = match last_line {
        Some(l) => (
            l.old_lineno.unwrap_or(hunk.old_start + hunk.old_count),
            l.new_lineno.unwrap_or(hunk.new_start + hunk.new_count),
        ),
        None => return,
    };

    let max_old = base_lines.len();
    let end = (last_old + count).min(max_old);

    for i in (last_old + 1)..=end {
        let old_idx = i - 1;
        let new_lineno = last_new + (i - last_old);
        let content = base_lines
            .get(old_idx)
            .copied()
            .unwrap_or("")
            .to_string();
        hunk.lines.push(DiffLine {
            kind: LineKind::Context,
            old_lineno: Some(i),
            new_lineno: Some(new_lineno),
            content,
        });
    }
}
