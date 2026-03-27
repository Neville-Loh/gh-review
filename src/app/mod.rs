pub(crate) mod command;
mod command_handlers;
pub(crate) mod custom_action;
mod handlers;
pub(crate) mod keymap;
mod ui;

use tokio::sync::mpsc;

use crate::components::command_bar::CommandBar;
use crate::components::comment_input::CommentInput;
use crate::components::diff_view::DiffView;
use crate::components::file_picker::FilePicker;
use crate::components::review_confirm::ReviewConfirm;
use crate::components::search_bar::SearchBar;
use crate::components::status_line::StatusLine;
use crate::event::AppEvent;
use std::collections::HashMap;

use crate::config::{Config, load_user_config};
use custom_action::resolve_custom_actions;
use crate::types::{DiffFile, ExistingComment, PrMetadata, ReviewComment, ThreadInfo};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Focus {
    FilePicker,
    DiffView,
}

impl Focus {
    pub fn next(self) -> Self {
        match self {
            Focus::FilePicker => Focus::DiffView,
            Focus::DiffView => Focus::FilePicker,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Focus::FilePicker => Focus::DiffView,
            Focus::DiffView => Focus::FilePicker,
        }
    }
}

pub enum Action {
    None,
    OpenEditor {
        file_path: String,
        line: usize,
        side: crate::types::Side,
        start_line: Option<usize>,
        start_side: Option<crate::types::Side>,
        content: String,
        file_ext: String,
    },
}

pub struct App {
    pub repo: String,
    pub pr_number: u64,

    pub(crate) pr_meta: Option<PrMetadata>,
    pub(crate) files: Vec<DiffFile>,
    pub(crate) existing_comments: Vec<ExistingComment>,
    pub(crate) pending_comments: Vec<ReviewComment>,
    pub(crate) thread_map: HashMap<u64, ThreadInfo>,

    pub(crate) file_picker: FilePicker,
    pub(crate) diff_view: DiffView,
    pub(crate) comment_input: CommentInput,
    pub(crate) review_confirm: ReviewConfirm,
    pub(crate) search_bar: SearchBar,
    pub(crate) command_bar: CommandBar,

    pub(crate) focus: Focus,
    pub(crate) show_help: bool,
    pub(crate) status: StatusLine,
    pub(crate) loading: bool,
    pub(crate) should_quit: bool,
    pub(crate) pending_key: Option<char>,
    pub(crate) visible_height: usize,

    pub(crate) pending_action: Option<Action>,
    pub(crate) config: Config,
    pub(crate) keymap: keymap::Keymap,
    pub(crate) tx: mpsc::UnboundedSender<AppEvent>,
}

impl App {
    pub fn new(repo: String, pr_number: u64, tx: mpsc::UnboundedSender<AppEvent>) -> Self {
        let user_config = load_user_config();
        let config = Config::from_user_config(&user_config);
        let resolved_actions = resolve_custom_actions(&user_config.actions);
        let keymap = keymap::Keymap::from_config(&user_config, resolved_actions);

        let mut status = StatusLine::new();
        if !keymap.warnings.is_empty() {
            status.error(format!("Config: {}", keymap.warnings.join("; ")));
        }

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
            command_bar: CommandBar::new(),
            focus: Focus::DiffView,
            show_help: false,
            status,
            loading: true,
            should_quit: false,
            pending_key: None,
            visible_height: 40,
            pending_action: None,
            config,
            keymap,
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

    pub fn handle_event(&mut self, event: AppEvent) -> Action {
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
                self.status.success("Thread updated");
                self.reload_threads();
            }
            AppEvent::ReviewDismissed => {
                self.status.success("Review dismissed");
            }
            AppEvent::SuggestionAccepted => {
                self.status.success("Suggestion applied");
            }
            AppEvent::FileContentLoaded {
                path,
                base_content,
                head_content,
            } => {
                self.expand_context(&path, &base_content, &head_content);
            }
            AppEvent::ReviewSubmitted => {
                self.status.success("Review submitted!");
                self.pending_comments.clear();
                self.rebuild_display();
                self.reload_comments_and_threads();
            }
            AppEvent::CustomActionComplete {
                description,
                result,
            } => match result {
                Ok(()) => self.status.success(description),
                Err(msg) => self.status.error(format!("✗ {description}: {msg}")),
            },
            AppEvent::Error(msg) => {
                self.status.error(msg);
                self.loading = false;
            }
        }
        self.pending_action.take().unwrap_or(Action::None)
    }

    pub(crate) fn rebuild_display(&mut self) {
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
