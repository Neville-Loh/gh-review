use crossterm::event::{KeyCode, KeyModifiers};

use crate::search::SearchDirection;
use crate::types::ReviewEvent;

use super::Focus;

pub enum Action {
    Quit,
    ClearSearchOrQuit,
    ToggleHelp,
    OpenSearchForward,
    OpenSearchBackward,
    StartFileFilter,
    SwitchFocus,
    ScrollDown(usize),
    ScrollUp(usize),
    PageDown(usize),
    PageUp(usize),
    GotoFirst,
    GotoLast,
    ScreenTop,
    ScreenMiddle,
    ScreenBottom,
    CenterCursor,
    ScrollCursorToTop,
    ScrollCursorToBottom,
    NextHunk,
    PrevHunk,
    NextChange,
    PrevChange,
    NextFileOrSearchHit(SearchDirection),
    PrevFileOrSearchHit(SearchDirection),
    ToggleCommentExpand,
    ToggleDiffMode,
    StartComment,
    ExpandContextOrToggleComment,
    ShowReviewConfirm(ReviewEvent),
    OpenInBrowser,
    PendingKey(char),
    FilePickerDown,
    FilePickerUp,
}

/// Map a key event to an Action when in normal mode (no modal active, no pending key).
pub fn key_to_action(
    key: crossterm::event::KeyEvent,
    focus: Focus,
    visible_height: usize,
) -> Option<Action> {
    match key.code {
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Esc => Some(Action::ClearSearchOrQuit),
        KeyCode::Char('!') | KeyCode::F(1) => Some(Action::ToggleHelp),

        KeyCode::Char('/') => match focus {
            Focus::DiffView => Some(Action::OpenSearchForward),
            Focus::FilePicker => Some(Action::StartFileFilter),
        },
        KeyCode::Char('?') => match focus {
            Focus::DiffView => Some(Action::OpenSearchBackward),
            Focus::FilePicker => Some(Action::ToggleHelp),
        },

        KeyCode::Tab => Some(Action::SwitchFocus),

        KeyCode::Char('j') | KeyCode::Down => match focus {
            Focus::DiffView => Some(Action::ScrollDown(1)),
            Focus::FilePicker => Some(Action::FilePickerDown),
        },
        KeyCode::Char('k') | KeyCode::Up => match focus {
            Focus::DiffView => Some(Action::ScrollUp(1)),
            Focus::FilePicker => Some(Action::FilePickerUp),
        },

        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::PageDown(visible_height / 2))
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::PageUp(visible_height / 2))
        }
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::PageDown(visible_height))
        }
        KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::PageUp(visible_height))
        }

        KeyCode::Char('g') => Some(Action::PendingKey('g')),
        KeyCode::Char('z') => Some(Action::PendingKey('z')),

        KeyCode::Char('G') => Some(Action::GotoLast),
        KeyCode::Char('H') => Some(Action::ScreenTop),
        KeyCode::Char('M') => Some(Action::ScreenMiddle),
        KeyCode::Char('L') => Some(Action::ScreenBottom),

        KeyCode::Char(']') | KeyCode::Char('}') => Some(Action::NextHunk),
        KeyCode::Char('[') | KeyCode::Char('{') => Some(Action::PrevHunk),

        KeyCode::Char(')') => Some(Action::NextChange),
        KeyCode::Char('(') => Some(Action::PrevChange),

        KeyCode::Char('n') => Some(Action::NextFileOrSearchHit(SearchDirection::Forward)),
        KeyCode::Char('N') => Some(Action::PrevFileOrSearchHit(SearchDirection::Forward)),

        KeyCode::Enter => Some(Action::ToggleCommentExpand),
        KeyCode::Char('t') => Some(Action::ToggleDiffMode),
        KeyCode::Char('c') => Some(Action::StartComment),
        KeyCode::Char('e') => Some(Action::ExpandContextOrToggleComment),

        KeyCode::Char('a') => Some(Action::ShowReviewConfirm(ReviewEvent::Approve)),
        KeyCode::Char('r') => Some(Action::ShowReviewConfirm(ReviewEvent::RequestChanges)),
        KeyCode::Char('s') => Some(Action::ShowReviewConfirm(ReviewEvent::Comment)),

        KeyCode::Char('o') => Some(Action::OpenInBrowser),

        _ => None,
    }
}

/// Map the second key in a two-key sequence to an Action.
pub fn pending_key_to_action(pending: char, code: KeyCode) -> Option<Action> {
    match (pending, code) {
        ('g', KeyCode::Char('g')) => Some(Action::GotoFirst),
        ('z', KeyCode::Char('z')) => Some(Action::CenterCursor),
        ('z', KeyCode::Char('t')) => Some(Action::ScrollCursorToTop),
        ('z', KeyCode::Char('b')) => Some(Action::ScrollCursorToBottom),
        _ => None,
    }
}
