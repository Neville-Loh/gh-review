use super::App;

#[derive(Clone, Copy)]
pub struct Command {
    pub name: &'static str,
    #[allow(dead_code)]
    pub doc: &'static str,
    pub typable: bool,
    pub execute: fn(&mut App),
}

impl Command {
    pub fn by_name(name: &str) -> Option<&'static Command> {
        COMMAND_LIST.iter().find(|c| c.name == name).copied()
    }

    pub fn typable_commands() -> impl Iterator<Item = &'static Command> {
        COMMAND_LIST.iter().copied().filter(|c| c.typable)
    }
}

impl std::fmt::Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Command")
            .field("name", &self.name)
            .finish()
    }
}

impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

macro_rules! define_commands {
    ($( $name:ident, $doc:literal, $typable:literal; )*) => {
        $(
            #[allow(non_upper_case_globals)]
            pub const $name: Command = Command {
                name: stringify!($name),
                doc: $doc,
                typable: $typable,
                execute: super::command_handlers::$name,
            };
        )*

        pub const COMMAND_LIST: &[&Command] = &[$( &$name, )*];
    };
}

// ── Command registry ────────────────────────────────────────────────

define_commands! {
    // System
    quit,                 "Quit",                          true;
    escape,               "Clear search / cancel / quit",  false;
    open_browser,         "Open PR in browser",            true;

    // Navigation
    scroll_down,          "Scroll down one line",          false;
    scroll_up,            "Scroll up one line",            false;
    half_page_down,       "Scroll down half page",         false;
    half_page_up,         "Scroll up half page",           false;
    full_page_down,       "Scroll down full page",         false;
    full_page_up,         "Scroll up full page",           false;
    goto_first,           "Go to first line",              false;
    goto_last,            "Go to last line",               false;
    screen_top,           "Cursor to screen top",          false;
    screen_middle,        "Cursor to screen middle",       false;
    screen_bottom,        "Cursor to screen bottom",       false;
    center_cursor,        "Center cursor in viewport",     false;
    scroll_cursor_top,    "Scroll cursor to top",          false;
    scroll_cursor_bottom, "Scroll cursor to bottom",       false;
    next_hunk,            "Jump to next hunk",             false;
    prev_hunk,            "Jump to previous hunk",         false;
    next_change,          "Jump to next change",           false;
    prev_change,          "Jump to previous change",       false;
    next_match_or_file,   "Next search match or file",     false;
    prev_match_or_file,   "Previous search match or file", false;

    // Search
    search_forward,       "Search forward",                false;
    search_backward,      "Search backward",               false;

    // UI
    help,                 "Toggle help overlay",           true;
    switch_focus,         "Switch focus: files / diff",    false;
    toggle_view,          "Toggle unified / side-by-side", true;
    toggle_comment,       "Toggle comment expand",         false;
    expand_all_comments,  "Expand all comment threads",   true;
    collapse_all_comments,"Collapse all comment threads", true;
    file_filter,          "Filter file list",              false;
    open_command_mode,    "Open command prompt",            false;
    fold_toggle,          "Toggle file fold",               false;
    fold_open,            "Expand file fold",               false;
    fold_close,           "Collapse file fold",             false;

    // Review -- typable
    comment,              "Review comment with body",      true;
    suggest,              "Suggest change on current line", true;
    expand,               "Expand context",                true;
    approve,              "Approve (quick confirm)",       true;
    approve_with_comment, "Approve with review body",     true;
    request_changes,      "Request changes (quick)",      true;
    request_changes_with_comment, "Request changes with body", true;
    submit,               "Submit comment-only (quick)",  true;
    unapprove,            "Dismiss own approval",          true;
    discard,              "Discard pending comment",       true;
    resolve,              "Resolve / unresolve thread",    true;
    accept_suggestion,    "Accept suggestion",             true;

    // Review -- keybinding only
    comment_on_line,      "Comment on current line",       false;
    visual,               "Visual select mode",            false;

    // Config
    config_path,          "Show config file path",         true;

    // File picker
    picker_down,          "File picker: next",             false;
    picker_up,            "File picker: previous",         false;
}
