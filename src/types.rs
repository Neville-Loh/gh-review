use serde::Deserialize;

#[derive(Debug, Clone, PartialEq)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
}

impl FileStatus {
    pub fn from_str(s: &str) -> Self {
        match s {
            "added" => Self::Added,
            "removed" => Self::Deleted,
            "renamed" => Self::Renamed,
            "copied" => Self::Copied,
            _ => Self::Modified,
        }
    }

    pub fn symbol(&self) -> &str {
        match self {
            Self::Added => "A",
            Self::Modified => "M",
            Self::Deleted => "D",
            Self::Renamed => "R",
            Self::Copied => "C",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiffFile {
    pub path: String,
    pub status: FileStatus,
    pub additions: usize,
    pub deletions: usize,
    pub hunks: Vec<Hunk>,
}

#[derive(Debug, Clone)]
pub struct Hunk {
    pub header: String,
    pub old_start: usize,
    pub old_count: usize,
    pub new_start: usize,
    pub new_count: usize,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LineKind {
    Context,
    Added,
    Removed,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub kind: LineKind,
    pub old_lineno: Option<usize>,
    pub new_lineno: Option<usize>,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Side {
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct ReviewComment {
    pub path: String,
    pub line: usize,
    pub side: Side,
    pub body: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReviewEvent {
    Approve,
    RequestChanges,
    Comment,
}

impl ReviewEvent {
    pub fn as_api_str(&self) -> &str {
        match self {
            Self::Approve => "APPROVE",
            Self::RequestChanges => "REQUEST_CHANGES",
            Self::Comment => "COMMENT",
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Approve => "Approve",
            Self::RequestChanges => "Request Changes",
            Self::Comment => "Comment",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiffMode {
    Unified,
    SideBySide,
}

#[derive(Debug, Deserialize)]
pub struct PrMetadata {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub head: PrRef,
    pub base: PrRef,
    pub user: PrUser,
    pub additions: Option<usize>,
    pub deletions: Option<usize>,
    pub changed_files: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct PrRef {
    pub sha: String,
    #[serde(rename = "ref")]
    pub ref_name: String,
}

#[derive(Debug, Deserialize)]
pub struct PrUser {
    pub login: String,
}

#[derive(Debug, Deserialize)]
pub struct GhFile {
    pub filename: String,
    pub status: String,
    pub additions: usize,
    pub deletions: usize,
    pub patch: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExistingComment {
    pub id: u64,
    pub path: String,
    pub line: Option<usize>,
    pub side: Option<String>,
    pub body: String,
    pub user: PrUser,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(default)]
    pub in_reply_to_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct FileContent {
    pub content: String,
    pub encoding: String,
}
