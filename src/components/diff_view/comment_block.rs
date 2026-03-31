//! Detects expanded comment thread ranges in display rows and renders them
//! as ratatui [`Block`] widgets with rounded borders.
//!
//! This module owns the visual representation of expanded comment threads.
//! Collapsed (single-header) comments are rendered by `diff::renderer`.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph, Widget, Wrap},
};

use crate::diff::layout::{COMMENT_BLOCK_MAX_WIDTH, COMMENT_BLOCK_RIGHT_MARGIN, GUTTER_WIDTH};
use crate::diff::renderer::DisplayRow;
use crate::theme::Theme;

// ── Data ───────────────────────────────────────────────────────────────

/// A contiguous expanded comment thread detected in the visible display rows.
///
/// Holds the row range (header through footer) and pre-extracted visual
/// data needed to render the thread as a ratatui [`Block`] widget.
pub(super) struct CommentBlock {
    pub header_idx: usize,
    pub footer_idx: usize,
    bg: Color,
    border_color: Color,
    fg: Color,
    header_title: Line<'static>,
    body_lines: Vec<Line<'static>>,
}

// ── Rendering ──────────────────────────────────────────────────────────

impl CommentBlock {
    /// Render in unified mode. Screen position is derived from the global
    /// row index relative to `scroll`.
    pub fn render_unified(
        &self,
        area: Rect,
        buf: &mut Buffer,
        scroll: usize,
        end: usize,
        cursor: usize,
    ) {
        let vis_start = self.header_idx.max(scroll) - scroll;
        let vis_end = self.footer_idx.min(end - 1) - scroll;
        let has_top = self.header_idx >= scroll;
        let has_bottom = self.footer_idx < end;

        let available = area
            .width
            .saturating_sub(GUTTER_WIDTH as u16 + COMMENT_BLOCK_RIGHT_MARGIN);
        let width = available.min(COMMENT_BLOCK_MAX_WIDTH);

        let block_rect = Rect::new(
            area.x + GUTTER_WIDTH as u16,
            area.y + vis_start as u16,
            width,
            (vis_end - vis_start + 1) as u16,
        );

        self.paint(block_rect, buf, has_top, has_bottom);

        for y in vis_start..=vis_end {
            if scroll + y == cursor {
                paint_cursor(
                    area.x,
                    area.y + y as u16,
                    block_rect.width + GUTTER_WIDTH as u16,
                    buf,
                );
            }
        }
    }

    /// Render in side-by-side mode at an explicit screen position within
    /// one column.
    pub fn render_sbs(
        &self,
        col_area: Rect,
        buf: &mut Buffer,
        screen_y_start: usize,
        num_rows: usize,
        cursor_screen_y: Option<usize>,
    ) {
        let available = col_area.width.saturating_sub(COMMENT_BLOCK_RIGHT_MARGIN);
        let width = available.min(COMMENT_BLOCK_MAX_WIDTH);

        let block_rect = Rect::new(
            col_area.x,
            col_area.y + screen_y_start as u16,
            width,
            num_rows as u16,
        );

        self.paint(block_rect, buf, true, true);

        if let Some(cy) = cursor_screen_y {
            paint_cursor(col_area.x, col_area.y + cy as u16, col_area.width, buf);
        }
    }

    fn paint(&self, rect: Rect, buf: &mut Buffer, has_top: bool, has_bottom: bool) {
        let mut borders = Borders::LEFT | Borders::RIGHT;
        if has_top {
            borders |= Borders::TOP;
        }
        if has_bottom {
            borders |= Borders::BOTTOM;
        }

        let block = Block::new()
            .borders(borders)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(self.border_color))
            .style(Style::default().bg(self.bg).fg(self.fg))
            .padding(Padding::horizontal(1))
            .title_top(self.header_title.clone());

        let inner = block.inner(rect);
        Widget::render(block, rect, buf);

        if inner.height > 0 && inner.width > 0 {
            let body = Paragraph::new(self.body_lines.clone()).wrap(Wrap { trim: true });
            Widget::render(body, inner, buf);
        }
    }
}

fn paint_cursor(x: u16, y: u16, width: u16, buf: &mut Buffer) {
    buf.set_style(Rect::new(x, y, width, 1), Theme::selected_line());
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_char('▌');
        cell.set_style(Theme::selected_cursor());
    }
}

// ── Detection ──────────────────────────────────────────────────────────

/// Scan display rows and collect expanded comment blocks whose row range
/// overlaps the visible window `[scroll, end)`.
pub(super) fn find_comment_blocks(
    rows: &[DisplayRow],
    scroll: usize,
    end: usize,
) -> Vec<CommentBlock> {
    let mut blocks = Vec::new();
    let scan_start = find_block_start_before(rows, scroll);
    let scan_end = end.min(rows.len());
    let mut i = scan_start;

    while i < scan_end {
        if let DisplayRow::CommentHeader {
            expanded: true,
            is_reply: false,
            is_pending,
            is_resolved,
            author,
            reply_count,
            ..
        } = &rows[i]
        {
            if let Some(block) = collect_block(
                rows,
                i,
                scroll,
                *is_pending,
                *is_resolved,
                author,
                *reply_count,
            ) {
                i = block.footer_idx + 1;
                blocks.push(block);
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    blocks
}

/// Walk forward from a root header to its footer, collecting body lines.
/// Returns `None` if no footer is found or the block ends before `scroll`.
fn collect_block(
    rows: &[DisplayRow],
    header_idx: usize,
    scroll: usize,
    is_pending: bool,
    is_resolved: bool,
    author: &str,
    reply_count: usize,
) -> Option<CommentBlock> {
    let (bg, border_color, fg) = thread_colors(is_pending, is_resolved);
    let header_title = build_title(is_pending, is_resolved, author, reply_count, fg);

    let mut body_lines = Vec::new();
    let mut j = header_idx + 1;

    while j < rows.len() {
        match &rows[j] {
            DisplayRow::CommentFooter { .. } => {
                if j < scroll {
                    return None;
                }
                return Some(CommentBlock {
                    header_idx,
                    footer_idx: j,
                    bg,
                    border_color,
                    fg,
                    header_title,
                    body_lines,
                });
            }
            DisplayRow::CommentBodyLine { line, .. } => {
                body_lines.push(line.clone());
                j += 1;
            }
            DisplayRow::CommentHeader {
                is_reply: true,
                author,
                ..
            } => {
                body_lines.push(Line::from(Span::styled(
                    format!("↩ {author}"),
                    Style::default().fg(fg).add_modifier(Modifier::DIM),
                )));
                j += 1;
            }
            _ => return None,
        }
    }
    None
}

/// Walk backwards from `scroll` to find a block header whose body extends
/// into the visible window.
fn find_block_start_before(rows: &[DisplayRow], scroll: usize) -> usize {
    if scroll == 0 {
        return 0;
    }
    for i in (0..scroll).rev() {
        match &rows[i] {
            DisplayRow::CommentHeader {
                expanded: true,
                is_reply: false,
                ..
            } => return i,
            DisplayRow::CommentBodyLine { .. } | DisplayRow::CommentFooter { .. } => continue,
            _ => return scroll,
        }
    }
    scroll
}

// ── Helpers ────────────────────────────────────────────────────────────

fn thread_colors(is_pending: bool, is_resolved: bool) -> (Color, Color, Color) {
    if is_pending {
        (
            Theme::pending_bg(),
            Theme::pending_accent(),
            Theme::pending_fg(),
        )
    } else if is_resolved {
        (
            Theme::resolved_bg(),
            Theme::resolved_accent(),
            Theme::resolved_fg(),
        )
    } else {
        (
            Theme::comment_bg(),
            Theme::comment_accent(),
            Theme::comment_fg(),
        )
    }
}

fn build_title(
    is_pending: bool,
    is_resolved: bool,
    author: &str,
    reply_count: usize,
    fg: Color,
) -> Line<'static> {
    let mut spans = Vec::new();

    if is_pending {
        spans.push(Span::styled(
            " 📝 pending ",
            Style::default().fg(fg).add_modifier(Modifier::BOLD),
        ));
    } else {
        spans.push(Span::styled(
            format!(" 💬 {author} "),
            Style::default().fg(fg).add_modifier(Modifier::BOLD),
        ));
        if is_resolved {
            spans.push(Span::styled("✓ Resolved ", Style::default().fg(fg)));
        }
        if reply_count > 0 {
            spans.push(Span::styled(
                format!("({reply_count} replies) "),
                Style::default().fg(fg).add_modifier(Modifier::DIM),
            ));
        }
    }

    Line::from(spans)
}
