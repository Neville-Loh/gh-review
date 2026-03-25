use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};
use tokio::sync::mpsc;
use tui_textarea::Input;

use crate::components::comment_input::{CommentAction, CommentInput};
use crate::components::diff_view::DiffView;
use crate::components::file_picker::FilePicker;
use crate::components::help::HelpOverlay;
use crate::components::review_bar::ReviewBar;
use crate::components::review_confirm::ReviewConfirm;
use crate::components::search_bar::SearchBar;
use crate::search::SearchDirection;
use crate::event::AppEvent;
use crate::types::{DiffFile, ExistingComment, PrMetadata, ReviewComment, ReviewEvent};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Focus {
    FilePicker,
    DiffView,
}

pub struct App {
    pub repo: String,
    pub pr_number: u64,

    pr_meta: Option<PrMetadata>,
    files: Vec<DiffFile>,
    existing_comments: Vec<ExistingComment>,
    pending_comments: Vec<ReviewComment>,

    file_picker: FilePicker,
    diff_view: DiffView,
    comment_input: CommentInput,
    review_confirm: ReviewConfirm,
    search_bar: SearchBar,

    focus: Focus,
    show_help: bool,
    status_msg: String,
    status_is_error: bool,
    loading: bool,
    should_quit: bool,
    pending_key: Option<char>,
    visible_height: usize,

    tx: mpsc::UnboundedSender<AppEvent>,
}

impl App {
    pub fn new(repo: String, pr_number: u64, tx: mpsc::UnboundedSender<AppEvent>) -> Self {
        Self {
            repo,
            pr_number,
            pr_meta: None,
            files: Vec::new(),
            existing_comments: Vec::new(),
            pending_comments: Vec::new(),
            file_picker: FilePicker::new(),
            diff_view: DiffView::new(),
            comment_input: CommentInput::new(),
            review_confirm: ReviewConfirm::new(),
            search_bar: SearchBar::new(),
            focus: Focus::DiffView,
            show_help: false,
            status_msg: String::new(),
            status_is_error: false,
            loading: true,
            should_quit: false,
            pending_key: None,
            visible_height: 40,
            tx,
        }
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn start_loading(&self) {
        let tx = self.tx.clone();
        let repo = self.repo.clone();
        let pr = self.pr_number;

        tokio::spawn(async move {
            match crate::gh::fetch_pr_metadata(&repo, pr).await {
                Ok(meta) => {
                    let _ = tx.send(AppEvent::PrLoaded(Box::new(meta)));
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::Error(format!("Failed to load PR: {e}")));
                }
            }
        });

        let tx2 = self.tx.clone();
        let repo2 = self.repo.clone();
        tokio::spawn(async move {
            match crate::gh::fetch_pr_files(&repo2, pr).await {
                Ok(files) => {
                    let _ = tx2.send(AppEvent::FilesLoaded(files));
                }
                Err(e) => {
                    let _ = tx2.send(AppEvent::Error(format!("Failed to load files: {e}")));
                }
            }
        });

        let tx3 = self.tx.clone();
        let repo3 = self.repo.clone();
        tokio::spawn(async move {
            match crate::gh::fetch_review_comments(&repo3, pr).await {
                Ok(comments) => {
                    let _ = tx3.send(AppEvent::CommentsLoaded(comments));
                }
                Err(e) => {
                    let _ = tx3.send(AppEvent::Error(format!("Failed to load comments: {e}")));
                }
            }
        });
    }

    pub fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Key(key) => self.handle_key(key),
            AppEvent::Resize(_, _) => {}
            AppEvent::Tick => {}
            AppEvent::PrLoaded(meta) => {
                self.pr_meta = Some(*meta);
                self.loading = false;
            }
            AppEvent::FilesLoaded(files) => {
                self.file_picker.set_files(&files);
                self.files = files;
                self.rebuild_display();
                self.loading = false;
            }
            AppEvent::CommentsLoaded(comments) => {
                self.existing_comments = comments;
                self.rebuild_display();
            }
            AppEvent::FileContentLoaded {
                path,
                base_content,
                head_content,
            } => {
                self.expand_context(&path, &base_content, &head_content);
            }
            AppEvent::ReviewSubmitted => {
                self.status_msg = "✓ Review submitted!".to_string();
                self.status_is_error = false;
                self.pending_comments.clear();
                self.rebuild_display();
                // Re-fetch comments so submitted ones appear as existing
                let tx = self.tx.clone();
                let repo = self.repo.clone();
                let pr = self.pr_number;
                tokio::spawn(async move {
                    match crate::gh::fetch_review_comments(&repo, pr).await {
                        Ok(comments) => {
                            let _ = tx.send(AppEvent::CommentsLoaded(comments));
                        }
                        Err(_) => {}
                    }
                });
            }
            AppEvent::Error(msg) => {
                self.status_msg = msg;
                self.status_is_error = true;
                self.loading = false;
            }
        }
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        // Review confirmation popup takes priority
        if self.review_confirm.visible {
            match key.code {
                KeyCode::Enter => {
                    let event = self.review_confirm.event;
                    self.review_confirm.hide();
                    self.submit_review(event);
                }
                KeyCode::Esc => {
                    self.review_confirm.hide();
                }
                _ => {}
            }
            return;
        }

        // Comment input takes priority when visible
        if self.comment_input.visible {
            let input: Input = key.into();
            match self.comment_input.handle_input(input) {
                CommentAction::Submit(body) => {
                    if let Some(comment_id) = self.comment_input.reply_to_id {
                        self.submit_reply(comment_id, body);
                    } else {
                        self.pending_comments.push(ReviewComment {
                            path: self.comment_input.file_path.clone(),
                            line: self.comment_input.line,
                            side: self.comment_input.side,
                            body,
                        });
                        self.rebuild_display();
                    }
                }
                CommentAction::Cancel => {}
                CommentAction::None => {}
            }
            return;
        }

        // Search bar input
        if self.search_bar.active {
            match key.code {
                KeyCode::Enter => {
                    self.search_bar.close();
                    let (curr, total) = self.diff_view.search.match_info();
                    if total > 0 {
                        self.status_msg = format!(
                            "/{} [{}/{}]",
                            self.search_bar.input,
                            curr + 1,
                            total
                        );
                        self.status_is_error = false;
                    } else if !self.search_bar.input.is_empty() {
                        self.status_msg =
                            format!("Pattern not found: {}", self.search_bar.input);
                        self.status_is_error = true;
                    }
                }
                KeyCode::Esc => {
                    if let Some(anchor) = self.diff_view.search.anchor() {
                        self.diff_view.cursor = anchor;
                    }
                    self.diff_view.search.clear();
                    self.search_bar.close();
                    self.status_msg.clear();
                }
                KeyCode::Backspace => {
                    self.search_bar.pop_char();
                    let dir = self.search_bar.direction;
                    self.diff_view.apply_search(&self.search_bar.input, dir);
                }
                KeyCode::Char(c) => {
                    self.search_bar.push_char(c);
                    let dir = self.search_bar.direction;
                    self.diff_view.apply_search(&self.search_bar.input, dir);
                }
                _ => {}
            }
            return;
        }

        // File picker filter input
        if self.file_picker.is_filter_active() {
            match key.code {
                KeyCode::Enter => {
                    self.file_picker.confirm_filter();
                    self.diff_view.goto_file(self.file_picker.selected);
                }
                KeyCode::Esc => {
                    self.file_picker.cancel_filter();
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.file_picker.filter_next();
                    self.diff_view.goto_file(self.file_picker.selected);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.file_picker.filter_prev();
                    self.diff_view.goto_file(self.file_picker.selected);
                }
                KeyCode::Backspace => {
                    self.file_picker.filter_pop();
                    self.diff_view.goto_file(self.file_picker.selected);
                }
                KeyCode::Char(c) => {
                    self.file_picker.filter_push(c);
                    if !self.file_picker.files.is_empty() {
                        self.diff_view.goto_file(self.file_picker.selected);
                    }
                }
                _ => {}
            }
            return;
        }

        // Help overlay
        if self.show_help {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::F(1) => {
                    self.show_help = false;
                }
                _ => {}
            }
            return;
        }

        // Handle two-key sequences (gg, zz, zt, zb)
        if let Some(pending) = self.pending_key.take() {
            match (pending, key.code) {
                ('g', KeyCode::Char('g')) => self.diff_view.goto_first(),
                ('z', KeyCode::Char('z')) => self.diff_view.center_cursor(self.visible_height),
                ('z', KeyCode::Char('t')) => {
                    self.diff_view.scroll_offset = self.diff_view.cursor;
                }
                ('z', KeyCode::Char('b')) => {
                    self.diff_view.scroll_offset =
                        self.diff_view.cursor.saturating_sub(self.visible_height.saturating_sub(1));
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Esc => {
                if self.diff_view.search.is_active() {
                    self.diff_view.search.clear();
                    self.status_msg.clear();
                } else {
                    self.should_quit = true;
                }
            }

            // Help
            KeyCode::F(1) => self.show_help = true,

            // Search
            KeyCode::Char('/') => match self.focus {
                Focus::DiffView => {
                    self.diff_view.search.set_anchor(self.diff_view.cursor);
                    self.search_bar.open(SearchDirection::Forward);
                }
                Focus::FilePicker => {
                    self.file_picker.start_filter();
                }
            },
            KeyCode::Char('?') => {
                if self.focus == Focus::DiffView {
                    self.diff_view.search.set_anchor(self.diff_view.cursor);
                    self.search_bar.open(SearchDirection::Backward);
                } else {
                    self.show_help = true;
                }
            }

            // Focus switching
            KeyCode::Tab => {
                self.focus = match self.focus {
                    Focus::FilePicker => Focus::DiffView,
                    Focus::DiffView => Focus::FilePicker,
                };
            }

            // Navigation — basic
            KeyCode::Char('j') | KeyCode::Down => match self.focus {
                Focus::DiffView => self.diff_view.scroll_down(1),
                Focus::FilePicker => {
                    self.file_picker.next();
                    self.diff_view.goto_file(self.file_picker.selected);
                }
            },
            KeyCode::Char('k') | KeyCode::Up => match self.focus {
                Focus::DiffView => self.diff_view.scroll_up(1),
                Focus::FilePicker => {
                    self.file_picker.prev();
                    self.diff_view.goto_file(self.file_picker.selected);
                }
            },

            // Navigation — page scroll
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.diff_view.page_down(self.visible_height / 2);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.diff_view.page_up(self.visible_height / 2);
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.diff_view.page_down(self.visible_height);
            }
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.diff_view.page_up(self.visible_height);
            }

            // Navigation — start pending key sequences
            KeyCode::Char('g') => self.pending_key = Some('g'),
            KeyCode::Char('z') => self.pending_key = Some('z'),

            // Navigation — top/bottom
            KeyCode::Char('G') => self.diff_view.goto_last(),

            // Navigation — screen-relative (H/M/L)
            KeyCode::Char('H') => self.diff_view.screen_top(),
            KeyCode::Char('M') => self.diff_view.screen_middle(self.visible_height),
            KeyCode::Char('L') => self.diff_view.screen_bottom(self.visible_height),

            // Navigation — hunk jumping
            KeyCode::Char(']') | KeyCode::Char('}') => self.diff_view.next_hunk(),
            KeyCode::Char('[') | KeyCode::Char('{') => self.diff_view.prev_hunk(),

            // Navigation — jump to next/prev change
            KeyCode::Char(')') => self.diff_view.next_change(),
            KeyCode::Char('(') => self.diff_view.prev_change(),

            // Navigation — file/search jumping (n/N dual-purpose)
            KeyCode::Char('n') => {
                if self.diff_view.search.is_active() {
                    let new_cursor = match self.search_bar.direction {
                        SearchDirection::Forward => self.diff_view.search.next_match(),
                        SearchDirection::Backward => self.diff_view.search.prev_match(),
                    };
                    if let Some(c) = new_cursor {
                        self.diff_view.cursor = c;
                    }
                } else {
                    self.diff_view.next_file();
                }
                if let Some(fi) = self.diff_view.current_file_idx() {
                    self.file_picker.selected = fi;
                }
            }
            KeyCode::Char('N') => {
                if self.diff_view.search.is_active() {
                    let new_cursor = match self.search_bar.direction {
                        SearchDirection::Forward => self.diff_view.search.prev_match(),
                        SearchDirection::Backward => self.diff_view.search.next_match(),
                    };
                    if let Some(c) = new_cursor {
                        self.diff_view.cursor = c;
                    }
                } else {
                    self.diff_view.prev_file();
                }
                if let Some(fi) = self.diff_view.current_file_idx() {
                    self.file_picker.selected = fi;
                }
            }

            // Expand/collapse comment
            KeyCode::Enter => {
                if self.diff_view.toggle_comment_expand() {
                    self.rebuild_display();
                }
            }

            // View toggle
            KeyCode::Char('t') => self.diff_view.toggle_mode(),

            // Comment: reply on comment row, new comment on diff line
            KeyCode::Char('c') => {
                if let Some(target) = self.diff_view.comment_reply_target() {
                    self.comment_input
                        .open_reply(target.github_id, target.author);
                } else {
                    self.start_comment();
                }
            }

            // Expand context or toggle comment
            KeyCode::Char('e') => {
                if self.diff_view.toggle_comment_expand() {
                    self.rebuild_display();
                } else {
                    self.request_expand();
                }
            }

            // Review actions — show confirmation popup
            KeyCode::Char('a') => self.review_confirm.show(ReviewEvent::Approve, self.pending_comments.len()),
            KeyCode::Char('r') => self.review_confirm.show(ReviewEvent::RequestChanges, self.pending_comments.len()),
            KeyCode::Char('s') => self.review_confirm.show(ReviewEvent::Comment, self.pending_comments.len()),

            // Open in browser
            KeyCode::Char('o') => self.open_in_browser(),

            _ => {}
        }
    }

    fn start_comment(&mut self) {
        if let Some(target) = self.diff_view.current_line_info() {
            if let Some(file) = self.files.get(target.file_idx) {
                self.comment_input
                    .open(file.path.clone(), target.line, target.side);
            }
        }
    }

    fn request_expand(&mut self) {
        if let Some((file_idx, _hunk_idx)) = self.diff_view.current_hunk_idx() {
            if let Some(file) = self.files.get(file_idx) {
                if let Some(ref meta) = self.pr_meta {
                    let tx = self.tx.clone();
                    let repo = self.repo.clone();
                    let path = file.path.clone();
                    let base_ref = meta.base.sha.clone();
                    let head_ref = meta.head.sha.clone();

                    tokio::spawn(async move {
                        let base =
                            crate::gh::fetch_file_content(&repo, &path, &base_ref).await;
                        let head =
                            crate::gh::fetch_file_content(&repo, &path, &head_ref).await;

                        match (base, head) {
                            (Ok(b), Ok(h)) => {
                                let _ = tx.send(AppEvent::FileContentLoaded {
                                    path,
                                    base_content: b,
                                    head_content: h,
                                });
                            }
                            (Err(e), _) | (_, Err(e)) => {
                                let _ = tx.send(AppEvent::Error(format!("Expand failed: {e}")));
                            }
                        }
                    });
                }
            }
        }
    }

    fn expand_context(&mut self, path: &str, base_content: &str, head_content: &str) {
        let base_lines: Vec<&str> = base_content.lines().collect();
        let head_lines: Vec<&str> = head_content.lines().collect();

        if let Some((file_idx, hunk_idx)) = self.diff_view.current_hunk_idx() {
            if let Some(file) = self.files.get_mut(file_idx) {
                if file.path == path {
                    if let Some(hunk) = file.hunks.get_mut(hunk_idx) {
                        crate::diff::expand::expand_hunk_context(
                            hunk,
                            &base_lines,
                            &head_lines,
                            10,
                        );
                    }
                }
            }
        }

        self.rebuild_display();
    }

    fn submit_review(&mut self, event: ReviewEvent) {
        let tx = self.tx.clone();
        let repo = self.repo.clone();
        let pr = self.pr_number;
        let comments = self.pending_comments.clone();
        let body = String::new();

        self.status_msg = format!("Submitting {}...", event.label());

        tokio::spawn(async move {
            match crate::gh::submit_review(&repo, pr, event, &body, &comments).await {
                Ok(()) => {
                    let _ = tx.send(AppEvent::ReviewSubmitted);
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::Error(format!("Submit failed: {e}")));
                }
            }
        });
    }

    fn submit_reply(&mut self, comment_id: u64, body: String) {
        let tx = self.tx.clone();
        let repo = self.repo.clone();
        let pr = self.pr_number;

        self.status_msg = "Posting reply...".to_string();
        self.status_is_error = false;

        tokio::spawn(async move {
            match crate::gh::reply_to_comment(&repo, pr, comment_id, &body).await {
                Ok(()) => {
                    let _ = tx.send(AppEvent::ReviewSubmitted);
                    match crate::gh::fetch_review_comments(&repo, pr).await {
                        Ok(comments) => {
                            let _ = tx.send(AppEvent::CommentsLoaded(comments));
                        }
                        Err(_) => {}
                    }
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::Error(format!("Reply failed: {e}")));
                }
            }
        });
    }

    fn open_in_browser(&self) {
        let url = format!("https://github.com/{}/pull/{}", self.repo, self.pr_number);
        let _ = std::process::Command::new("open").arg(&url).spawn();
    }

    fn rebuild_display(&mut self) {
        self.diff_view.rebuild_rows(
            &self.files,
            &self.existing_comments,
            &self.pending_comments,
        );
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let size = frame.area();

        // Main layout: content + review bar
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title
                Constraint::Min(0),   // Content
                Constraint::Length(1), // Review bar
            ])
            .split(size);

        // Title bar
        self.draw_title(frame, main_layout[0]);

        // Content: file picker (left) + diff view (right)
        let content_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(30), // File picker
                Constraint::Min(0),    // Diff
            ])
            .split(main_layout[1]);

        // Update scroll visibility before drawing
        let diff_height = content_layout[1].height.saturating_sub(2) as usize;
        self.visible_height = diff_height;
        self.diff_view.ensure_visible(diff_height);

        self.file_picker.draw(
            content_layout[0],
            frame.buffer_mut(),
            self.focus == Focus::FilePicker,
        );

        self.diff_view.draw(
            content_layout[1],
            frame.buffer_mut(),
            self.focus == Focus::DiffView,
        );

        // Review bar / search bar
        if self.search_bar.active {
            let (curr, total) = self.diff_view.search.match_info();
            self.search_bar
                .draw(main_layout[2], frame.buffer_mut(), curr, total);
        } else {
            ReviewBar::draw(
                main_layout[2],
                frame.buffer_mut(),
                self.pending_comments.len(),
                &self.status_msg,
                self.status_is_error,
            );
        }

        // Overlays
        if self.comment_input.visible {
            self.comment_input.draw(content_layout[1], frame.buffer_mut());
        }

        if self.review_confirm.visible {
            self.review_confirm.draw(size, frame.buffer_mut());
        }

        if self.show_help {
            HelpOverlay::draw(size, frame.buffer_mut());
        }
    }

    fn draw_title(&self, frame: &mut Frame, area: Rect) {
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Paragraph, Widget};

        let title = if let Some(ref meta) = self.pr_meta {
            format!(
                " {} #{} — {} ({}→{})",
                self.repo, meta.number, meta.title, meta.base.ref_name, meta.head.ref_name
            )
        } else if self.loading {
            format!(" {} #{} — Loading...", self.repo, self.pr_number)
        } else {
            format!(" {} #{}", self.repo, self.pr_number)
        };

        let line = Line::from(vec![Span::styled(title, crate::theme::Theme::title())]);
        let bar = Paragraph::new(line).style(crate::theme::Theme::review_bar());
        Widget::render(bar, area, frame.buffer_mut());
    }
}
