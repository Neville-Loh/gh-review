mod actions;
mod handlers;
mod ui;

use tokio::sync::mpsc;

use crate::components::comment_input::CommentInput;
use crate::components::diff_view::DiffView;
use crate::components::file_picker::FilePicker;
use crate::components::review_confirm::ReviewConfirm;
use crate::components::search_bar::SearchBar;
use crate::event::AppEvent;
use std::collections::HashMap;

use crate::types::{DiffFile, ExistingComment, PrMetadata, ReviewComment, ThreadInfo};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Focus {
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
    thread_map: HashMap<u64, ThreadInfo>,

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
            thread_map: HashMap::new(),
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

        let tx4 = self.tx.clone();
        let repo4 = self.repo.clone();
        tokio::spawn(async move {
            match crate::gh::fetch_review_threads(&repo4, pr).await {
                Ok(threads) => {
                    let _ = tx4.send(AppEvent::ThreadsLoaded(threads));
                }
                Err(e) => {
                    let _ = tx4.send(AppEvent::Error(format!("Failed to load threads: {e}")));
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
            AppEvent::ThreadsLoaded(threads) => {
                self.thread_map = threads;
                self.rebuild_display();
            }
            AppEvent::ThreadResolveToggled => {
                self.status_msg = "✓ Thread updated".to_string();
                self.status_is_error = false;
                self.reload_threads();
            }
            AppEvent::ReviewDismissed => {
                self.status_msg = "✓ Review dismissed".to_string();
                self.status_is_error = false;
            }
            AppEvent::SuggestionAccepted => {
                self.status_msg = "✓ Suggestion applied".to_string();
                self.status_is_error = false;
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
                self.reload_comments_and_threads();
            }
            AppEvent::Error(msg) => {
                self.status_msg = msg;
                self.status_is_error = true;
                self.loading = false;
            }
        }
    }

    fn rebuild_display(&mut self) {
        self.diff_view.rebuild_rows(
            &self.files,
            &self.existing_comments,
            &self.pending_comments,
            &self.thread_map,
        );
    }

    fn reload_threads(&self) {
        let tx = self.tx.clone();
        let repo = self.repo.clone();
        let pr = self.pr_number;
        tokio::spawn(async move {
            if let Ok(threads) = crate::gh::fetch_review_threads(&repo, pr).await {
                let _ = tx.send(AppEvent::ThreadsLoaded(threads));
            }
        });
    }

    fn reload_comments_and_threads(&self) {
        let tx = self.tx.clone();
        let repo = self.repo.clone();
        let pr = self.pr_number;
        tokio::spawn(async move {
            if let Ok(comments) = crate::gh::fetch_review_comments(&repo, pr).await {
                let _ = tx.send(AppEvent::CommentsLoaded(comments));
            }
        });
        self.reload_threads();
    }
}
