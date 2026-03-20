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

    pub fn selected_path(&self) -> Option<&str> {
        self.files.get(self.selected).map(|f| f.path.as_str())
    }

    pub fn draw(&self, area: Rect, buf: &mut Buffer, focused: bool) {
        let border_style = if focused {
            Theme::border_focused()
        } else {
            Theme::border()
        };

        let title = format!(" Files ({}/{}) ", self.selected + 1, self.files.len());
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style);

        let items: Vec<ListItem> = self
            .files
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let status_style = match f.status {
                    FileStatus::Added => Theme::status_added(),
                    FileStatus::Deleted => Theme::status_deleted(),
                    _ => Theme::status_modified(),
                };

                let short_path = shorten_path(&f.path, area.width.saturating_sub(14) as usize);
                let style = if i == self.selected {
                    Theme::file_list_selected()
                } else {
                    Theme::file_list_normal()
                };

                ListItem::new(Line::from(vec![
                    Span::styled(format!("{} ", f.status.symbol()), status_style),
                    Span::styled(short_path, style),
                    Span::styled(
                        format!(" +{}", f.additions),
                        Theme::status_added(),
                    ),
                    Span::styled(
                        format!(" -{}", f.deletions),
                        Theme::status_deleted(),
                    ),
                ]))
            })
            .collect();

        let mut state = ListState::default();
        state.select(Some(self.selected));

        let list = List::new(items).block(block);
        Widget::render(list, area, buf);
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
