//! Stack PR management.
//!
//! A "stack" is an ordered list of related PRs. The concept is generic --
//! Graphite is one provider, but others (spr, ghstack) could be added.
//!
//! [`StackState`] holds the current stack and navigates between PRs.
//! [`PrCache`] stores fetched PR data so navigation is instant.

pub mod graphite;

use std::collections::HashMap;

use crate::types::{DiffFile, ExistingComment, PrMetadata, ReviewComment, ThreadInfo};

/// Visual status of a PR in a stack.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrStatus {
    Open,
    Approved,
    ChangesRequested,
    Draft,
    Merged,
    Closed,
}

impl PrStatus {
    pub fn from_metadata(state: &str, draft: bool, review_decision: Option<&str>) -> Self {
        match state.to_uppercase().as_str() {
            "MERGED" => Self::Merged,
            "CLOSED" => Self::Closed,
            _ if draft => Self::Draft,
            _ => match review_decision {
                Some("APPROVED") => Self::Approved,
                Some("CHANGES_REQUESTED") => Self::ChangesRequested,
                _ => Self::Open,
            },
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Open => "●",
            Self::Approved => "✓",
            Self::ChangesRequested => "✗",
            Self::Draft => "◌",
            Self::Merged => "✓",
            Self::Closed => "✗",
        }
    }

    pub fn color(self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            Self::Open => Color::Green,
            Self::Approved => Color::Green,
            Self::ChangesRequested => Color::Red,
            Self::Draft => Color::DarkGray,
            Self::Merged => Color::Magenta,
            Self::Closed => Color::Red,
        }
    }
}

/// A PR in a stack.
#[derive(Debug, Clone, PartialEq)]
pub struct PrLink {
    pub owner: String,
    pub repo: String,
    pub pr_number: u64,
    pub url: String,
}

/// Holds the stack PR list and a title cache.
///
/// The cache persists across comment reloads so titles are only fetched
/// once per session.
pub struct StackState {
    pub links: Vec<PrLink>,
    pub current_pr: u64,
    title_cache: HashMap<u64, String>,
    status_cache: HashMap<u64, PrStatus>,
}

impl StackState {
    pub fn empty() -> Self {
        Self {
            links: Vec::new(),
            current_pr: 0,
            title_cache: HashMap::new(),
            status_cache: HashMap::new(),
        }
    }

    /// Load stack from comments using all known providers (currently Graphite).
    pub fn load_from_comments(
        &mut self,
        comments: &[crate::types::ExistingComment],
        current_pr: u64,
    ) {
        self.current_pr = current_pr;
        self.links = graphite::extract_stack(comments);
    }

    /// Store fetched titles into the cache.
    pub fn insert_titles(&mut self, titles: &[(u64, String)]) {
        for (pr_number, title) in titles {
            self.title_cache.insert(*pr_number, title.clone());
        }
    }

    /// Store a PR's status.
    pub fn insert_status(&mut self, pr_number: u64, status: PrStatus) {
        self.status_cache.insert(pr_number, status);
    }

    /// Look up a cached title.
    pub fn title(&self, pr_number: u64) -> Option<&str> {
        self.title_cache.get(&pr_number).map(String::as_str)
    }

    /// Look up a cached status.
    pub fn status(&self, pr_number: u64) -> Option<PrStatus> {
        self.status_cache.get(&pr_number).copied()
    }

    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }

    /// PR number above the current one in the stack (toward newest).
    pub fn pr_above(&self) -> Option<u64> {
        let idx = self
            .links
            .iter()
            .position(|l| l.pr_number == self.current_pr)?;
        self.links.get(idx + 1).map(|l| l.pr_number)
    }

    /// PR number below the current one in the stack (toward base/main).
    pub fn pr_below(&self) -> Option<u64> {
        let idx = self
            .links
            .iter()
            .position(|l| l.pr_number == self.current_pr)?;
        if idx > 0 {
            Some(self.links[idx - 1].pr_number)
        } else {
            None
        }
    }
}

// ── PrCache ──────────────────────────────────────────────────────────

/// Snapshot of all data needed to display a PR.
#[derive(Debug)]
pub struct PrSnapshot {
    pub meta: PrMetadata,
    pub files: Vec<DiffFile>,
    pub comments: Vec<ExistingComment>,
    pub review_body_comments: Vec<ExistingComment>,
    pub pending_comments: Vec<ReviewComment>,
    pub threads: HashMap<u64, ThreadInfo>,
}

/// Caches full PR data so stack navigation is instant.
///
/// Keyed by PR number. The current PR's data is saved here before
/// navigating away, and restored when navigating back.
pub struct PrCache {
    entries: HashMap<u64, PrSnapshot>,
}

impl Default for PrCache {
    fn default() -> Self {
        Self::new()
    }
}

impl PrCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn insert(&mut self, pr_number: u64, snapshot: PrSnapshot) {
        self.entries.insert(pr_number, snapshot);
    }

    pub fn take(&mut self, pr_number: u64) -> Option<PrSnapshot> {
        self.entries.remove(&pr_number)
    }

    pub fn contains(&self, pr_number: u64) -> bool {
        self.entries.contains_key(&pr_number)
    }
}
