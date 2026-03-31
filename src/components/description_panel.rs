//! A collapsible drawer displaying the PR title and description body,
//! rendered as rich markdown. Toggled via `:description`, closes when
//! focus leaves.
//!
//! Uses a line-by-line cursor model matching the diff panel. All keys
//! are dispatched through the keymap via scoped command bindings.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};
use unicode_width::UnicodeWidthStr;

use crate::stack::{PrStatus, StackState};
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CursorRegion {
    Title,
    Body,
}

pub struct DescriptionPanel {
    pub visible: bool,
    pub title: String,
    pub body: String,
    pub branch_info: String,
    pub cursor: usize,
    scroll_offset: usize,
    content_lines: Vec<ContentLine>,
    body_start: usize,
    pub last_width: u16,
}

#[derive(Clone)]
struct ContentLine {
    line: Line<'static>,
    region: CursorRegion,
}

impl DescriptionPanel {
    pub fn new() -> Self {
        Self {
            visible: false,
            title: String::new(),
            body: String::new(),
            branch_info: String::new(),
            cursor: 0,
            scroll_offset: 0,
            content_lines: Vec::new(),
            body_start: 0,
            last_width: 0,
        }
    }

    pub fn load(&mut self, title: &str, body: Option<&str>, branch_info: &str) {
        self.title = title.to_string();
        self.body = body.unwrap_or("").to_string();
        self.branch_info = branch_info.to_string();
        self.rebuild_content(self.last_width.max(60));
    }

    pub fn cursor_down(&mut self) {
        let max = self.content_lines.len().saturating_sub(1);
        self.cursor = (self.cursor + 1).min(max);
    }

    pub fn cursor_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn half_page_down(&mut self, page: usize) {
        let max = self.content_lines.len().saturating_sub(1);
        self.cursor = (self.cursor + page / 2).min(max);
    }

    pub fn half_page_up(&mut self, page: usize) {
        self.cursor = self.cursor.saturating_sub(page / 2);
    }

    pub fn goto_top(&mut self) {
        self.cursor = 0;
    }

    pub fn goto_bottom(&mut self) {
        self.cursor = self.content_lines.len().saturating_sub(1);
    }

    pub fn next_section(&mut self) {
        if self.cursor < self.body_start {
            self.cursor = self.body_start;
        }
    }

    pub fn prev_section(&mut self) {
        if self.cursor >= self.body_start {
            self.cursor = 0;
        }
    }

    pub fn cursor_region(&self) -> CursorRegion {
        self.content_lines
            .get(self.cursor)
            .map(|cl| cl.region)
            .unwrap_or(CursorRegion::Title)
    }

    pub fn draw(&mut self, area: Rect, buf: &mut Buffer, focused: bool, stack: &StackState) {
        let border_style = if focused {
            Theme::border_focused()
        } else {
            Theme::border()
        };

        let outer = Block::default()
            .title(" Description ")
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = outer.inner(area);
        Widget::render(outer, area, buf);

        if inner.height < 2 || inner.width < 4 {
            return;
        }

        if inner.width != self.last_width {
            self.last_width = inner.width;
            self.rebuild_content(inner.width);
        }

        // Reserve space at the bottom for the stack indicator
        let stack_height = if stack.is_empty() {
            0u16
        } else {
            (stack.links.len() as u16 + 3).min(inner.height / 2)
        };
        let content_height = inner.height.saturating_sub(stack_height);

        let visible_height = content_height as usize;
        self.ensure_visible(visible_height);

        let end = (self.scroll_offset + visible_height).min(self.content_lines.len());

        for (screen_y, idx) in (self.scroll_offset..end).enumerate() {
            let y = inner.y + screen_y as u16;
            self.render_line(idx, inner.x, y, inner.width, focused, buf);
        }

        // Render stack indicator at the bottom
        if stack_height > 0 {
            let stack_y = inner.y + content_height;
            Self::render_stack(stack, inner.x, stack_y, inner.width, stack_height, buf);
        }
    }

    fn render_line(
        &self,
        idx: usize,
        x: u16,
        y: u16,
        width: u16,
        focused: bool,
        buf: &mut Buffer,
    ) {
        let cl = &self.content_lines[idx];
        if focused && idx == self.cursor {
            let mut spans = vec![Span::styled("▌", Theme::selected_cursor())];
            spans.extend(
                cl.line
                    .spans
                    .iter()
                    .map(|s| Span::styled(s.content.clone(), s.style.patch(Theme::selected_line()))),
            );
            buf.set_line(x, y, &Line::from(spans), width);
        } else {
            buf.set_line(x, y, &cl.line, width);
        }
    }

    fn render_stack(stack: &StackState, x: u16, y: u16, width: u16, height: u16, buf: &mut Buffer) {
        let mut row = 0u16;

        // Separator line
        if row < height {
            let sep_label = " Stack ";
            let side_len = (width as usize).saturating_sub(sep_label.len()) / 2;
            let sep = format!(
                "{}{}{}",
                "─".repeat(side_len),
                sep_label,
                "─".repeat(side_len),
            );
            buf.set_line(
                x,
                y + row,
                &Line::from(Span::styled(sep, Style::default().fg(Color::DarkGray))),
                width,
            );
            row += 1;
        }

        // PR list (newest first -- already sorted ascending, render in reverse)
        for pr in stack.links.iter().rev() {
            if row >= height {
                break;
            }
            let is_current = pr.pr_number == stack.current_pr;
            let cursor_mark = if is_current { "▸" } else { " " };
            let status = stack.status(pr.pr_number).unwrap_or(PrStatus::Open);
            let status_icon = status.icon();
            let status_color = status.color();
            let name = stack
                .title(pr.pr_number)
                .unwrap_or("")
                .chars()
                .take((width as usize).saturating_sub(12))
                .collect::<String>();
            let pr_label = if name.is_empty() {
                format!("#{}", pr.pr_number)
            } else {
                format!("#{} {name}", pr.pr_number)
            };
            let text_style = if is_current {
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let line = Line::from(vec![
                Span::styled(format!("{cursor_mark} "), text_style),
                Span::styled(format!("{status_icon} "), Style::default().fg(status_color)),
                Span::styled(pr_label, text_style),
            ]);
            buf.set_line(x, y + row, &line, width);
            row += 1;
        }

        // "main" at the bottom
        if row < height {
            buf.set_line(
                x,
                y + row,
                &Line::from(Span::styled("  main", Style::default().fg(Color::DarkGray))),
                width,
            );
        }
    }

    fn ensure_visible(&mut self, visible_height: usize) {
        if self.cursor < self.scroll_offset {
            self.scroll_offset = self.cursor;
        } else if self.cursor >= self.scroll_offset + visible_height {
            self.scroll_offset = self.cursor - visible_height + 1;
        }
    }

    pub fn rebuild_content(&mut self, width: u16) {
        let max_w = width.saturating_sub(2) as usize;
        let mut lines = build_title_lines(&self.title, &self.branch_info, max_w);
        let body_start = lines.len();
        lines.extend(build_body_lines(&self.body, max_w));

        self.body_start = body_start;
        self.content_lines = lines;
        if self.cursor >= self.content_lines.len() {
            self.cursor = self.content_lines.len().saturating_sub(1);
        }
    }
}

fn build_title_lines(title: &str, branch_info: &str, max_w: usize) -> Vec<ContentLine> {
    let mut lines = Vec::new();
    for wl in wrap_text(title, max_w) {
        lines.push(ContentLine {
            line: Line::from(Span::styled(
                wl,
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            )),
            region: CursorRegion::Title,
        });
    }
    if !branch_info.is_empty() {
        lines.push(ContentLine {
            line: Line::from(Span::styled(
                branch_info.to_string(),
                Style::default().fg(Color::DarkGray),
            )),
            region: CursorRegion::Title,
        });
    }
    lines.push(ContentLine {
        line: Line::from(Span::styled(
            "─".repeat(max_w.min(40)),
            Style::default().fg(Color::DarkGray),
        )),
        region: CursorRegion::Title,
    });
    lines.push(ContentLine {
        line: Line::default(),
        region: CursorRegion::Title,
    });
    lines
}

fn build_body_lines(body: &str, max_w: usize) -> Vec<ContentLine> {
    if body.trim().is_empty() {
        return vec![ContentLine {
            line: Line::from(Span::styled(
                "No description provided.",
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            )),
            region: CursorRegion::Body,
        }];
    }
    let mut lines = Vec::new();
    let md_rendered = tui_markdown::from_str(body);
    for md_line in md_rendered.lines {
        let line_style = md_line.style;
        let text: String = md_line.spans.iter().map(|s| s.content.as_ref()).collect();
        let owned_spans: Vec<Span> = md_line
            .spans
            .into_iter()
            .map(|s| Span::styled(s.content.to_string(), s.style))
            .collect();
        if text.width() <= max_w {
            lines.push(ContentLine {
                line: Line::from(owned_spans).style(line_style),
                region: CursorRegion::Body,
            });
        } else {
            for wl in crate::diff::wrap::wrap_spans(&owned_spans, max_w) {
                lines.push(ContentLine {
                    line: wl.style(line_style),
                    region: CursorRegion::Body,
                });
            }
        }
    }
    lines
}

fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 || text.width() <= max_width {
        return vec![text.to_string()];
    }
    let mut result = Vec::new();
    let mut current = String::new();
    let mut current_w = 0;
    for word in text.split_inclusive(' ') {
        let ww = word.width();
        if current_w + ww > max_width && !current.is_empty() {
            result.push(current);
            current = String::new();
            current_w = 0;
        }
        current.push_str(word);
        current_w += ww;
    }
    if !current.is_empty() {
        result.push(current);
    }
    if result.is_empty() {
        result.push(String::new());
    }
    result
}
