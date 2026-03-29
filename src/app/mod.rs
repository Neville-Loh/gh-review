pub(crate) mod command;
mod command_handlers;
pub(crate) mod custom_action;
mod handlers;
pub(crate) mod keymap;
mod ui;

use tokio::sync::mpsc;

use crate::components::command_bar::CommandBar;
use crate::components::comment_input::CommentInput;
use crate::components::description_panel::DescriptionPanel;
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
    Description,
}

impl Focus {
    pub fn next(self, desc_open: bool) -> Self {
        match self {
            Focus::FilePicker => Focus::DiffView,
            Focus::DiffView if desc_open => Focus::Description,
            Focus::DiffView => Focus::FilePicker,
            Focus::Description => Focus::FilePicker,
        }
    }

    pub fn prev(self, desc_open: bool) -> Self {
        match self {
            Focus::FilePicker if desc_open => Focus::Description,
            Focus::FilePicker => Focus::DiffView,
            Focus::DiffView => Focus::FilePicker,
            Focus::Description => Focus::DiffView,
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
    EditDescription {
        region: crate::components::description_panel::CursorRegion,
        content: String,
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
    pub(crate) description_panel: DescriptionPanel,
    pub(crate) stack: crate::stack::StackState,
    pub(crate) pr_cache: crate::stack::PrCache,
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
    /// Tracks how many of the 4 initial load events have arrived.
    primary_load_count: u8,
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
            description_panel: DescriptionPanel::new(),
            stack: crate::stack::StackState::empty(),
            pr_cache: crate::stack::PrCache::new(),
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
            primary_load_count: 0,
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
                    let _ = tx.send(AppEvent::PrLoaded { pr, data: Box::new(meta) });
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
                    let _ = tx2.send(AppEvent::FilesLoaded { pr, data: files });
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
                    let _ = tx3.send(AppEvent::CommentsLoaded { pr, data: comments });
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
                    let _ = tx4.send(AppEvent::ThreadsLoaded { pr, data: threads });
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
            AppEvent::PrLoaded { pr, data: meta } if pr == self.pr_number => {
                self.description_panel.load(&meta.title, meta.body.as_deref());
                self.stack.insert_titles(&[(self.pr_number, meta.title.clone())]);
                self.pr_meta = Some(*meta);
                self.loading = false;
                self.check_primary_ready();
            }
            AppEvent::FilesLoaded { pr, data: files } if pr == self.pr_number => {
                self.file_picker.set_files(&files);
                self.files = files;
                self.rebuild_display();
                self.loading = false;
                self.check_primary_ready();
            }
            AppEvent::CommentsLoaded { pr, data: comments } if pr == self.pr_number => {
                self.stack.load_from_comments(&comments, self.pr_number);
                if !self.stack.is_empty() {
                    self.prefetch_stack();
                }
                self.existing_comments = comments;
                self.rebuild_display();
                self.check_primary_ready();
            }
            AppEvent::ThreadsLoaded { pr, data: threads } if pr == self.pr_number => {
                self.thread_map = threads;
                self.rebuild_display();
                self.check_primary_ready();
            }
            // Stale events from a previously active PR -- discard
            AppEvent::PrLoaded { .. }
            | AppEvent::FilesLoaded { .. }
            | AppEvent::CommentsLoaded { .. }
            | AppEvent::ThreadsLoaded { .. } => {}
            AppEvent::StackTitlesLoaded(_) => {}
            AppEvent::StackPrefetchLoaded(snapshots) => {
                for (pr_number, snapshot) in snapshots {
                    self.stack.insert_titles(&[(pr_number, snapshot.meta.title.clone())]);
                    self.pr_cache.insert(pr_number, snapshot);
                }
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

    fn check_primary_ready(&mut self) {
        self.primary_load_count += 1;
    }

    fn prefetch_stack(&self) {
        let pr_numbers: Vec<u64> = self
            .stack
            .links
            .iter()
            .filter(|l| l.pr_number != self.pr_number)
            .filter(|l| !self.pr_cache.contains(l.pr_number))
            .map(|l| l.pr_number)
            .collect();
        if pr_numbers.is_empty() {
            return;
        }
        let tx = self.tx.clone();
        let repo = self.repo.clone();
        tokio::spawn(async move {
            match crate::gh::fetch_prs_batch(&repo, &pr_numbers).await {
                Ok(batch) => {
                    let mut snapshots = std::collections::HashMap::new();
                    for (pr_number, (meta, comments, threads)) in batch {
                        // Files need REST (GraphQL doesn't provide patches).
                        // Pre-fetch files per PR concurrently.
                        let files = crate::gh::fetch_pr_files(&repo, pr_number)
                            .await
                            .unwrap_or_default();
                        snapshots.insert(
                            pr_number,
                            crate::stack::PrSnapshot {
                                meta,
                                files,
                                comments,
                                pending_comments: Vec::new(),
                                threads,
                            },
                        );
                    }
                    let _ = tx.send(AppEvent::StackPrefetchLoaded(snapshots));
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::Error(format!("Stack prefetch failed: {e}")));
                }
            }
        });
    }

    // ── Stack navigation ──────────────────────────────────────────────

    /// Save the current PR's state to the cache.
    fn save_current_to_cache(&mut self) {
        if let Some(meta) = self.pr_meta.take() {
            let snapshot = crate::stack::PrSnapshot {
                meta,
                files: std::mem::take(&mut self.files),
                comments: std::mem::take(&mut self.existing_comments),
                pending_comments: std::mem::take(&mut self.pending_comments),
                threads: std::mem::take(&mut self.thread_map),
            };
            self.pr_cache.insert(self.pr_number, snapshot);
        }
    }

    /// Load a PR from cache into the active state. Returns false if not cached.
    fn restore_from_cache(&mut self, pr_number: u64) -> bool {
        if let Some(snapshot) = self.pr_cache.take(pr_number) {
            self.pr_number = pr_number;
            self.description_panel.load(&snapshot.meta.title, snapshot.meta.body.as_deref());
            self.pr_meta = Some(snapshot.meta);
            self.files = snapshot.files;
            self.existing_comments = snapshot.comments;
            self.pending_comments = snapshot.pending_comments;
            self.thread_map = snapshot.threads;
            self.file_picker.set_files(&self.files);
            self.diff_view = crate::components::diff_view::DiffView::new();
            self.rebuild_display();
            self.stack.current_pr = pr_number;
            self.loading = false;
            true
        } else {
            false
        }
    }

    /// Navigate to a different PR in the stack.
    pub(crate) fn navigate_to_pr(&mut self, pr_number: u64) {
        if pr_number == self.pr_number {
            return;
        }
        let mode = self.diff_view.mode;
        self.save_current_to_cache();

        if self.restore_from_cache(pr_number) {
            self.diff_view.mode = mode;
            self.status.success(format!("Switched to PR #{pr_number}"));
        } else {
            // Not cached -- fetch via REST (same as initial load)
            self.pr_number = pr_number;
            self.pr_meta = None;
            self.files.clear();
            self.existing_comments.clear();
            self.pending_comments.clear();
            self.thread_map.clear();
            self.file_picker.set_files(&self.files);
            self.diff_view = crate::components::diff_view::DiffView::new();
            self.diff_view.mode = mode;
            self.stack.current_pr = pr_number;
            self.primary_load_count = 0;
            self.loading = true;
            self.status.info(format!("Loading PR #{pr_number}..."));
            self.start_loading();
        }
    }

    fn reload_threads(&self) {
        let tx = self.tx.clone();
        let repo = self.repo.clone();
        let pr = self.pr_number;
        tokio::spawn(async move {
            if let Ok(threads) = crate::gh::fetch_review_threads(&repo, pr).await {
                let _ = tx.send(AppEvent::ThreadsLoaded { pr, data: threads });
            }
        });
    }

    fn reload_comments_and_threads(&self) {
        let tx = self.tx.clone();
        let repo = self.repo.clone();
        let pr = self.pr_number;
        tokio::spawn(async move {
            if let Ok(comments) = crate::gh::fetch_review_comments(&repo, pr).await {
                let _ = tx.send(AppEvent::CommentsLoaded { pr, data: comments });
            }
        });
        self.reload_threads();
    }
}
