use crossterm::event::KeyCode;
use tui_textarea::Input;

use crate::components::comment_input::CommentAction;
use crate::event::AppEvent;
use crate::search::SearchDirection;
use crate::types::{ReviewComment, ReviewEvent};

use super::App;
use super::Focus;
use super::actions::{Action, key_to_action, pending_key_to_action};

impl App {
    pub(super) fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        if self.review_confirm.visible {
            self.handle_review_confirm_key(key.code);
            return;
        }

        if self.comment_input.visible {
            self.handle_comment_input_key(key);
            return;
        }

        if self.search_bar.active {
            self.handle_search_bar_key(key.code);
            return;
        }

        if self.file_picker.is_filter_active() {
            self.handle_filter_key(key.code);
            return;
        }

        if self.show_help {
            self.handle_help_key(key.code);
            return;
        }

        if let Some(pending) = self.pending_key.take() {
            if let Some(action) = pending_key_to_action(pending, key.code) {
                self.execute_action(action);
            }
            return;
        }

        if let Some(action) = key_to_action(key, self.focus, self.visible_height) {
            self.execute_action(action);
        }
    }

    fn execute_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::ClearSearchOrQuit => {
                if self.diff_view.search.is_active() {
                    self.diff_view.search.clear();
                    self.status_msg.clear();
                } else {
                    self.should_quit = true;
                }
            }
            Action::ToggleHelp => self.show_help = !self.show_help,
            Action::OpenSearchForward => {
                self.diff_view.search.set_anchor(self.diff_view.cursor);
                self.search_bar.open(SearchDirection::Forward);
            }
            Action::OpenSearchBackward => {
                self.diff_view.search.set_anchor(self.diff_view.cursor);
                self.search_bar.open(SearchDirection::Backward);
            }
            Action::StartFileFilter => self.file_picker.start_filter(),
            Action::SwitchFocus => {
                self.focus = match self.focus {
                    Focus::FilePicker => Focus::DiffView,
                    Focus::DiffView => Focus::FilePicker,
                };
            }
            Action::ScrollDown(n) => self.diff_view.scroll_down(n),
            Action::ScrollUp(n) => self.diff_view.scroll_up(n),
            Action::PageDown(n) => self.diff_view.page_down(n),
            Action::PageUp(n) => self.diff_view.page_up(n),
            Action::GotoFirst => self.diff_view.goto_first(),
            Action::GotoLast => self.diff_view.goto_last(),
            Action::ScreenTop => self.diff_view.screen_top(),
            Action::ScreenMiddle => self.diff_view.screen_middle(self.visible_height),
            Action::ScreenBottom => self.diff_view.screen_bottom(self.visible_height),
            Action::CenterCursor => self.diff_view.center_cursor(self.visible_height),
            Action::ScrollCursorToTop => {
                self.diff_view.scroll_offset = self.diff_view.cursor;
            }
            Action::ScrollCursorToBottom => {
                self.diff_view.scroll_offset = self
                    .diff_view
                    .cursor
                    .saturating_sub(self.visible_height.saturating_sub(1));
            }
            Action::NextHunk => self.diff_view.next_hunk(),
            Action::PrevHunk => self.diff_view.prev_hunk(),
            Action::NextChange => self.diff_view.next_change(),
            Action::PrevChange => self.diff_view.prev_change(),
            Action::NextFileOrSearchHit(search_dir) => {
                if self.diff_view.search.is_active() {
                    let new_cursor = match search_dir {
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
            Action::PrevFileOrSearchHit(search_dir) => {
                if self.diff_view.search.is_active() {
                    let new_cursor = match search_dir {
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
            Action::ToggleCommentExpand => {
                if self.diff_view.toggle_comment_expand() {
                    self.rebuild_display();
                }
            }
            Action::ToggleDiffMode => self.diff_view.toggle_mode(),
            Action::StartComment => {
                if let Some(target) = self.diff_view.comment_reply_target() {
                    self.comment_input
                        .open_reply(target.github_id, target.author);
                } else {
                    self.start_comment();
                }
            }
            Action::ExpandContextOrToggleComment => {
                if self.diff_view.toggle_comment_expand() {
                    self.rebuild_display();
                } else {
                    self.request_expand();
                }
            }
            Action::ShowReviewConfirm(event) => {
                self.review_confirm
                    .show(event, self.pending_comments.len());
            }
            Action::OpenInBrowser => self.open_in_browser(),
            Action::PendingKey(c) => self.pending_key = Some(c),
            Action::FilePickerDown => {
                self.file_picker.next();
                self.diff_view.goto_file(self.file_picker.selected);
            }
            Action::FilePickerUp => {
                self.file_picker.prev();
                self.diff_view.goto_file(self.file_picker.selected);
            }
        }
    }

    // --- Modal key handlers ---

    fn handle_review_confirm_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Enter => {
                let event = self.review_confirm.event;
                self.review_confirm.hide();
                self.submit_review(event);
            }
            KeyCode::Esc => self.review_confirm.hide(),
            _ => {}
        }
    }

    fn handle_comment_input_key(&mut self, key: crossterm::event::KeyEvent) {
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
    }

    fn handle_search_bar_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Enter => {
                self.search_bar.close();
                let (curr, total) = self.diff_view.search.match_info();
                if total > 0 {
                    self.status_msg =
                        format!("/{} [{}/{}]", self.search_bar.input, curr + 1, total);
                    self.status_is_error = false;
                } else if !self.search_bar.input.is_empty() {
                    self.status_msg = format!("Pattern not found: {}", self.search_bar.input);
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
    }

    fn handle_filter_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Enter => {
                self.file_picker.confirm_filter();
                self.diff_view.goto_file(self.file_picker.selected);
            }
            KeyCode::Esc => self.file_picker.cancel_filter(),
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
    }

    fn handle_help_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('!') | KeyCode::F(1) => {
                self.show_help = false;
            }
            _ => {}
        }
    }

    // --- Async command handlers ---

    pub(super) fn start_comment(&mut self) {
        if let Some(target) = self.diff_view.current_line_info()
            && let Some(file) = self.files.get(target.file_idx)
        {
            self.comment_input
                .open(file.path.clone(), target.line, target.side);
        }
    }

    pub(super) fn request_expand(&mut self) {
        if let Some((file_idx, _hunk_idx)) = self.diff_view.current_hunk_idx()
            && let Some(file) = self.files.get(file_idx)
            && let Some(ref meta) = self.pr_meta
        {
            let tx = self.tx.clone();
            let repo = self.repo.clone();
            let path = file.path.clone();
            let base_ref = meta.base.sha.clone();
            let head_ref = meta.head.sha.clone();

            tokio::spawn(async move {
                let base = crate::gh::fetch_file_content(&repo, &path, &base_ref).await;
                let head = crate::gh::fetch_file_content(&repo, &path, &head_ref).await;

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

    pub(super) fn expand_context(&mut self, path: &str, base_content: &str, head_content: &str) {
        let base_lines: Vec<&str> = base_content.lines().collect();
        let head_lines: Vec<&str> = head_content.lines().collect();

        if let Some((file_idx, hunk_idx)) = self.diff_view.current_hunk_idx()
            && let Some(file) = self.files.get_mut(file_idx)
            && file.path == path
            && let Some(hunk) = file.hunks.get_mut(hunk_idx)
        {
            crate::diff::expand::expand_hunk_context(hunk, &base_lines, &head_lines, 10);

            let highlighted = crate::highlight::highlight_content(path, base_content);
            for line in &mut hunk.lines {
                if line.highlighted_content.is_none()
                    && let Some(lineno) = line.old_lineno
                {
                    line.highlighted_content = highlighted.get(lineno - 1).cloned();
                }
            }
        }

        self.rebuild_display();
    }

    pub(super) fn submit_review(&mut self, event: ReviewEvent) {
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

    pub(super) fn submit_reply(&mut self, comment_id: u64, body: String) {
        let tx = self.tx.clone();
        let repo = self.repo.clone();
        let pr = self.pr_number;

        self.status_msg = "Posting reply...".to_string();
        self.status_is_error = false;

        tokio::spawn(async move {
            match crate::gh::reply_to_comment(&repo, pr, comment_id, &body).await {
                Ok(()) => {
                    let _ = tx.send(AppEvent::ReviewSubmitted);
                    if let Ok(comments) = crate::gh::fetch_review_comments(&repo, pr).await {
                        let _ = tx.send(AppEvent::CommentsLoaded(comments));
                    }
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::Error(format!("Reply failed: {e}")));
                }
            }
        });
    }

    pub(super) fn open_in_browser(&mut self) {
        let url = format!("https://github.com/{}/pull/{}", self.repo, self.pr_number);
        if let Err(e) = open::that(&url) {
            self.status_msg = format!("Failed to open browser: {e}");
            self.status_is_error = true;
        }
    }
}
