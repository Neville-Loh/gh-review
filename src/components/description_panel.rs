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
            visible: true,
            title: String::new(),
            body: String::new(),
            cursor: 0,
            scroll_offset: 0,
            content_lines: Vec::new(),
            body_start: 0,
            last_width: 0,
        }
    }

    pub fn load(&mut self, title: &str, body: Option<&str>) {
        self.title = title.to_string();
        self.body = body.unwrap_or("").to_string();
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

    pub fn draw(&mut self, area: Rect, buf: &mut Buffer, focused: bool) {
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

        let visible_height = inner.height as usize;
        self.ensure_visible(visible_height);

        let end = (self.scroll_offset + visible_height).min(self.content_lines.len());

        for (screen_y, idx) in (self.scroll_offset..end).enumerate() {
            let y = inner.y + screen_y as u16;
            self.render_line(idx, inner.x, y, inner.width, focused, buf);
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

    fn ensure_visible(&mut self, visible_height: usize) {
        if self.cursor < self.scroll_offset {
            self.scroll_offset = self.cursor;
        } else if self.cursor >= self.scroll_offset + visible_height {
            self.scroll_offset = self.cursor - visible_height + 1;
        }
    }

    pub fn rebuild_content(&mut self, width: u16) {
        let max_w = width.saturating_sub(2) as usize;
        let mut lines = Vec::new();

        // Title: bold white, wrapped to fit
        for wl in wrap_text(&self.title, max_w) {
            lines.push(ContentLine {
                line: Line::from(Span::styled(
                    wl,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )),
                region: CursorRegion::Title,
            });
        }

        // Thin separator under the title
        lines.push(ContentLine {
            line: Line::from(Span::styled(
                "─".repeat(max_w.min(40)),
                Style::default().fg(Color::DarkGray),
            )),
            region: CursorRegion::Title,
        });

        // Blank line before body
        lines.push(ContentLine {
            line: Line::default(),
            region: CursorRegion::Title,
        });

        let body_start = lines.len();

        // Body (markdown, wrapped)
        if self.body.trim().is_empty() {
            lines.push(ContentLine {
                line: Line::from(Span::styled(
                    "No description provided.",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                )),
                region: CursorRegion::Body,
            });
        } else {
            let md_rendered = tui_markdown::from_str(&self.body);
            for md_line in md_rendered.lines {
                let line_style = md_line.style;
                let text: String = md_line.spans.iter().map(|s| s.content.as_ref()).collect();
                if text.width() <= max_w {
                    let spans: Vec<Span> = md_line
                        .spans
                        .into_iter()
                        .map(|s| Span::styled(s.content.to_string(), s.style))
                        .collect();
                    lines.push(ContentLine {
                        line: Line::from(spans).style(line_style),
                        region: CursorRegion::Body,
                    });
                } else {
                    for wl in crate::diff::wrap::wrap_spans(
                        &md_line
                            .spans
                            .into_iter()
                            .map(|s| Span::styled(s.content.to_string(), s.style))
                            .collect::<Vec<_>>(),
                        max_w,
                    ) {
                        lines.push(ContentLine {
                            line: wl.style(line_style),
                            region: CursorRegion::Body,
                        });
                    }
                }
            }
        }

        self.body_start = body_start;
        self.content_lines = lines;
        if self.cursor >= self.content_lines.len() {
            self.cursor = self.content_lines.len().saturating_sub(1);
        }
    }
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
