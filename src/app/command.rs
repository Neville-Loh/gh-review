use crate::search::SearchDirection;
use crate::types::ReviewEvent;

use super::App;
use super::Focus;

#[derive(Clone, Copy)]
pub struct Command {
    pub name: &'static str,
    #[allow(dead_code)]
    pub doc: &'static str,
    pub execute: fn(&mut App),
}

impl Command {
    #[allow(dead_code)]
    pub fn by_name(name: &str) -> Option<&'static Command> {
        COMMAND_LIST.iter().find(|c| c.name == name).copied()
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
    ($( $const_name:ident, $str_name:literal, $doc:literal, $handler:expr; )*) => {
        $(
            #[allow(non_upper_case_globals)]
            pub const $const_name: Command = Command {
                name: $str_name,
                doc: $doc,
                execute: $handler,
            };
        )*

        #[allow(dead_code)]
        pub const COMMAND_LIST: &[&Command] = &[$( &$const_name, )*];
    };
}

define_commands! {
    // --- System ---
    quit,                "quit",                "Quit",                                   |app| app.should_quit = true;
    escape,              "escape",              "Clear search / cancel visual / quit",    cmd_escape;
    open_browser,        "open_browser",        "Open PR in browser",                     |app| app.open_in_browser();

    // --- Navigation ---
    scroll_down,         "scroll_down",         "Scroll down one line",                   |app| app.diff_view.scroll_down(1);
    scroll_up,           "scroll_up",           "Scroll up one line",                     |app| app.diff_view.scroll_up(1);
    half_page_down,      "half_page_down",      "Scroll down half page",                  |app| { let h = app.visible_height; app.diff_view.page_down(h / 2); };
    half_page_up,        "half_page_up",        "Scroll up half page",                    |app| { let h = app.visible_height; app.diff_view.page_up(h / 2); };
    full_page_down,      "full_page_down",      "Scroll down full page",                  |app| { let h = app.visible_height; app.diff_view.page_down(h); };
    full_page_up,        "full_page_up",        "Scroll up full page",                    |app| { let h = app.visible_height; app.diff_view.page_up(h); };
    goto_first,          "goto_first",          "Go to first line",                       |app| app.diff_view.goto_first();
    goto_last,           "goto_last",           "Go to last line",                        |app| app.diff_view.goto_last();
    screen_top,          "screen_top",          "Cursor to screen top",                   |app| app.diff_view.screen_top();
    screen_middle,       "screen_middle",       "Cursor to screen middle",                |app| { let h = app.visible_height; app.diff_view.screen_middle(h); };
    screen_bottom,       "screen_bottom",       "Cursor to screen bottom",                |app| { let h = app.visible_height; app.diff_view.screen_bottom(h); };
    center_cursor,       "center_cursor",       "Center cursor in viewport",              |app| { let h = app.visible_height; app.diff_view.center_cursor(h); };
    scroll_cursor_top,   "scroll_cursor_top",   "Scroll cursor to top",                   |app| app.diff_view.scroll_offset = app.diff_view.cursor;
    scroll_cursor_bottom,"scroll_cursor_bottom","Scroll cursor to bottom",                |app| {
        app.diff_view.scroll_offset = app.diff_view.cursor.saturating_sub(app.visible_height.saturating_sub(1));
    };
    next_hunk,           "next_hunk",           "Jump to next hunk",                      |app| app.diff_view.next_hunk();
    prev_hunk,           "prev_hunk",           "Jump to previous hunk",                  |app| app.diff_view.prev_hunk();
    next_change,         "next_change",         "Jump to next change",                    |app| app.diff_view.next_change();
    prev_change,         "prev_change",         "Jump to previous change",                |app| app.diff_view.prev_change();
    next_match_or_file,  "next_match_or_file",  "Next search match or file",              cmd_next_match_or_file;
    prev_match_or_file,  "prev_match_or_file",  "Previous search match or file",          cmd_prev_match_or_file;

    // --- Search ---
    search_forward,      "search_forward",      "Search forward",                         |app| {
        app.diff_view.search.set_anchor(app.diff_view.cursor);
        app.search_bar.open(SearchDirection::Forward);
    };
    search_backward,     "search_backward",     "Search backward",                        |app| {
        app.diff_view.search.set_anchor(app.diff_view.cursor);
        app.search_bar.open(SearchDirection::Backward);
    };

    // --- UI ---
    help,                "help",                "Toggle help overlay",                    |app| app.show_help = !app.show_help;
    switch_focus,        "switch_focus",        "Switch focus between file list and diff", |app| {
        app.focus = match app.focus { Focus::FilePicker => Focus::DiffView, Focus::DiffView => Focus::FilePicker };
    };
    toggle_view,         "toggle_view",         "Toggle unified / side-by-side",          |app| app.diff_view.toggle_mode();
    toggle_comment,      "toggle_comment",      "Toggle comment expand/collapse",         |app| {
        if app.diff_view.toggle_comment_expand() { app.rebuild_display(); }
    };
    file_filter,         "file_filter",         "Filter file list",                       |app| app.file_picker.start_filter();
    pending_g,           "pending_g",           "Start gg sequence",                      |app| app.pending_key = Some('g');
    pending_z,           "pending_z",           "Start zz/zt/zb sequence",                |app| app.pending_key = Some('z');

    // --- Review ---
    comment,             "comment",             "Comment on current line",                cmd_comment;
    suggest,             "suggest",             "Suggest change on current line",         cmd_suggest;
    expand,              "expand",              "Expand context",                         |app| app.request_expand();
    approve,             "approve",             "Submit review: approve",                 |app| app.review_confirm.show(ReviewEvent::Approve, app.pending_comments.len());
    request_changes,     "request_changes",     "Submit review: request changes",         |app| app.review_confirm.show(ReviewEvent::RequestChanges, app.pending_comments.len());
    submit,              "submit",              "Submit review: comment only",            |app| app.review_confirm.show(ReviewEvent::Comment, app.pending_comments.len());
    unapprove,           "unapprove",           "Dismiss own approval",                  |app| app.review_confirm.show(ReviewEvent::Unapprove, 0);
    discard,             "discard",             "Discard pending comment",                cmd_discard;
    resolve,             "resolve",             "Resolve / unresolve thread",             cmd_resolve;
    accept_suggestion,   "accept_suggestion",   "Accept suggestion",                      cmd_accept_suggestion;
    visual,              "visual",              "Visual select mode",                     |app| {
        if app.diff_view.is_visual_mode() { app.diff_view.cancel_visual(); }
        else { app.diff_view.start_visual(); }
    };

    // --- File picker ---
    picker_down,         "picker_down",         "File picker: next",                      |app| { app.file_picker.next(); app.diff_view.goto_file(app.file_picker.selected); };
    picker_up,           "picker_up",           "File picker: previous",                  |app| { app.file_picker.prev(); app.diff_view.goto_file(app.file_picker.selected); };
}

fn cmd_escape(app: &mut App) {
    if app.diff_view.is_visual_mode() {
        app.diff_view.cancel_visual();
    } else if app.diff_view.search.is_active() {
        app.diff_view.search.clear();
        app.status_msg.clear();
    } else {
        app.should_quit = true;
    }
}

fn cmd_next_match_or_file(app: &mut App) {
    if app.diff_view.search.is_active() {
        if let Some(c) = app.diff_view.search.next_match() {
            app.diff_view.cursor = c;
        }
        app.update_search_status();
    } else {
        app.diff_view.next_file();
    }
    if let Some(fi) = app.diff_view.current_file_idx() {
        app.file_picker.selected = fi;
    }
}

fn cmd_prev_match_or_file(app: &mut App) {
    if app.diff_view.search.is_active() {
        if let Some(c) = app.diff_view.search.prev_match() {
            app.diff_view.cursor = c;
        }
        app.update_search_status();
    } else {
        app.diff_view.prev_file();
    }
    if let Some(fi) = app.diff_view.current_file_idx() {
        app.file_picker.selected = fi;
    }
}

fn cmd_comment(app: &mut App) {
    if app.diff_view.is_visual_mode() {
        app.start_visual_comment();
    } else if let Some(pt) = app.diff_view.pending_comment_at_cursor() {
        if let Some(pc) = app.pending_comments.get(pt.pending_idx) {
            app.comment_input.open_edit(
                pt.pending_idx,
                pc.path.clone(),
                pc.line,
                pc.side,
                &pc.body,
            );
        }
    } else if let Some(target) = app.diff_view.comment_reply_target() {
        app.comment_input
            .open_reply(target.github_id, target.author);
    } else {
        app.start_comment();
    }
}

fn cmd_suggest(app: &mut App) {
    if app.diff_view.is_visual_mode() {
        if let Some(content) = app.diff_view.visual_selection_content()
            && let Some((start, end)) = app.diff_view.visual_selection_targets()
            && let Some(file) = app.files.get(start.file_idx)
        {
            app.comment_input.open_suggestion_range(
                file.path.clone(),
                start.line,
                start.side,
                end.line,
                end.side,
                &content,
            );
        }
        app.diff_view.cancel_visual();
    } else if let Some(content) = app.diff_view.current_line_content()
        && let Some(target) = app.diff_view.current_line_info()
        && let Some(file) = app.files.get(target.file_idx)
    {
        app.comment_input.open_suggestion(
            file.path.clone(),
            target.line,
            target.side,
            &content,
        );
    }
}

fn cmd_discard(app: &mut App) {
    if let Some(pt) = app.diff_view.pending_comment_at_cursor()
        && pt.pending_idx < app.pending_comments.len()
    {
        app.pending_comments.remove(pt.pending_idx);
        app.rebuild_display();
    }
}

fn cmd_resolve(app: &mut App) {
    if let Some(target) = app.diff_view.thread_resolve_target() {
        app.toggle_resolve_thread(target.thread_node_id, target.is_resolved);
    }
}

fn cmd_accept_suggestion(app: &mut App) {
    if let Some(target) = app.diff_view.suggestion_at_cursor() {
        app.accept_suggestion(target);
    }
}
