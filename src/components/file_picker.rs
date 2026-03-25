use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Widget},
};

use crate::theme::Theme;
use crate::types::{DiffFile, FileStatus};

pub struct FilePicker {
    pub selected: usize,
    pub files: Vec<FileEntry>,
    filter_active: bool,
    filter_input: String,
    filtered_indices: Vec<usize>,
    filter_cursor: usize,
}

pub struct FileEntry {
    pub path: String,
    pub status: FileStatus,
    pub additions: usize,
    pub deletions: usize,
}

impl FilePicker {
    pub fn new() -> Self {
        Self {
            selected: 0,
            files: Vec::new(),
            filter_active: false,
            filter_input: String::new(),
            filtered_indices: Vec::new(),
            filter_cursor: 0,
        }
    }

    pub fn set_files(&mut self, files: &[DiffFile]) {
        self.files = files
            .iter()
            .map(|f| FileEntry {
                path: f.path.clone(),
                status: f.status.clone(),
                additions: f.additions,
                deletions: f.deletions,
            })
            .collect();
        if self.selected >= self.files.len() && !self.files.is_empty() {
            self.selected = self.files.len() - 1;
        }
    }

    pub fn next(&mut self) {
        if !self.files.is_empty() {
            self.selected = (self.selected + 1) % self.files.len();
        }
    }

    pub fn prev(&mut self) {
        if !self.files.is_empty() {
            self.selected = self.selected.checked_sub(1).unwrap_or(self.files.len() - 1);
        }
    }

    #[allow(dead_code)]
    pub fn selected_path(&self) -> Option<&str> {
        self.files.get(self.selected).map(|f| f.path.as_str())
    }

    // --- Filter mode ---

    pub fn is_filter_active(&self) -> bool {
        self.filter_active
    }

    pub fn start_filter(&mut self) {
        self.filter_active = true;
        self.filter_input.clear();
        self.recompute_filter();
    }

    pub fn cancel_filter(&mut self) {
        self.filter_active = false;
        self.filter_input.clear();
    }

    pub fn confirm_filter(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected = self.filtered_indices[self.filter_cursor];
        }
        self.filter_active = false;
        self.filter_input.clear();
    }

    pub fn filter_push(&mut self, c: char) {
        self.filter_input.push(c);
        self.recompute_filter();
    }

    pub fn filter_pop(&mut self) {
        self.filter_input.pop();
        self.recompute_filter();
    }

    pub fn filter_next(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.filter_cursor = (self.filter_cursor + 1) % self.filtered_indices.len();
            self.selected = self.filtered_indices[self.filter_cursor];
        }
    }

    pub fn filter_prev(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.filter_cursor = self
                .filter_cursor
                .checked_sub(1)
                .unwrap_or(self.filtered_indices.len() - 1);
            self.selected = self.filtered_indices[self.filter_cursor];
        }
    }

    fn recompute_filter(&mut self) {
        if self.filter_input.is_empty() {
            self.filtered_indices = (0..self.files.len()).collect();
        } else {
            self.filtered_indices = self
                .files
                .iter()
                .enumerate()
                .filter(|(_, f)| fuzzy_match(&self.filter_input, &f.path))
                .map(|(i, _)| i)
                .collect();
        }
        self.filter_cursor = 0;
        if !self.filtered_indices.is_empty() {
            self.selected = self.filtered_indices[0];
        }
    }

    pub fn draw(&self, area: Rect, buf: &mut Buffer, focused: bool) {
        let border_style = if focused {
            Theme::border_focused()
        } else {
            Theme::border()
        };

        let title = if self.filter_active {
            format!(" Files ({} matches) ", self.filtered_indices.len())
        } else {
            format!(" Files ({}/{}) ", self.selected + 1, self.files.len())
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(area);
        Widget::render(block, area, buf);

        let list_height = if self.filter_active {
            inner.height.saturating_sub(1) as usize
        } else {
            inner.height as usize
        };

        let display_files: Vec<(usize, &FileEntry)> = if self.filter_active {
            self.filtered_indices
                .iter()
                .map(|&i| (i, &self.files[i]))
                .collect()
        } else {
            self.files.iter().enumerate().collect()
        };

        let display_selected = if self.filter_active {
            self.filter_cursor
        } else {
            self.selected
        };

        let scroll = if display_selected >= list_height {
            display_selected - list_height + 1
        } else {
            0
        };
        let visible_end = (scroll + list_height).min(display_files.len());
        let visible_files = &display_files[scroll..visible_end];

        let items: Vec<ListItem> = visible_files
            .iter()
            .enumerate()
            .map(|(vi, (_, f))| {
                let display_idx = scroll + vi;
                let status_style = match f.status {
                    FileStatus::Added => Theme::status_added(),
                    FileStatus::Deleted => Theme::status_deleted(),
                    _ => Theme::status_modified(),
                };

                let short_path = shorten_path(&f.path, inner.width.saturating_sub(12) as usize);
                let style = if display_idx == display_selected {
                    Theme::file_list_selected()
                } else {
                    Theme::file_list_normal()
                };

                ListItem::new(Line::from(vec![
                    Span::styled(format!("{} ", f.status.symbol()), status_style),
                    Span::styled(short_path, style),
                    Span::styled(format!(" +{}", f.additions), Theme::status_added()),
                    Span::styled(format!(" -{}", f.deletions), Theme::status_deleted()),
                ]))
            })
            .collect();

        let list_area = if self.filter_active {
            Rect::new(inner.x, inner.y, inner.width, list_height as u16)
        } else {
            inner
        };

        let mut state = ListState::default();
        let visible_selected = display_selected.saturating_sub(scroll);
        state.select(Some(visible_selected));

        let list = List::new(items);
        Widget::render(list, list_area, buf);

        if self.filter_active {
            let filter_y = inner.y + inner.height.saturating_sub(1);
            let filter_line = Line::from(vec![
                Span::styled("/", Theme::search_prompt()),
                Span::styled(self.filter_input.clone(), Theme::search_prompt()),
                Span::styled("█", Theme::search_prompt()),
            ]);
            buf.set_line(inner.x, filter_y, &filter_line, inner.width);
        }
    }
}

fn shorten_path(path: &str, max_width: usize) -> String {
    if path.len() <= max_width {
        return path.to_string();
    }
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() <= 1 {
        return path[..max_width].to_string();
    }
    let filename = parts.last().unwrap();
    let remaining = max_width.saturating_sub(filename.len() + 4);
    if remaining == 0 {
        return format!(".../{filename}");
    }
    let prefix: String = parts[..parts.len() - 1].join("/");
    if prefix.len() <= remaining {
        path.to_string()
    } else {
        format!(".../{filename}")
    }
}

/// Subsequence fuzzy match: all characters in pattern appear in order within text.
fn fuzzy_match(pattern: &str, text: &str) -> bool {
    let pattern_lower = pattern.to_lowercase();
    let text_lower = text.to_lowercase();
    let mut pattern_chars = pattern_lower.chars().peekable();
    for ch in text_lower.chars() {
        if pattern_chars.peek() == Some(&ch) {
            pattern_chars.next();
        }
    }
    pattern_chars.peek().is_none()
}
