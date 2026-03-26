use crate::search::SearchDirection;
use crate::types::ReviewEvent;

use super::App;
use super::Focus;

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
                execute: cmd::$name,
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
    file_filter,          "Filter file list",              false;
    open_command_mode,    "Open command prompt",            false;
    pending_g,            "Start gg sequence",             false;
    pending_z,            "Start zz/zt/zb sequence",       false;

    // Review
    comment,              "Comment on current line",       true;
    suggest,              "Suggest change on current line", true;
    expand,               "Expand context",                true;
    approve,              "Submit review: approve",        true;
    request_changes,      "Submit review: request changes", true;
    submit,               "Submit review: comment only",   true;
    unapprove,            "Dismiss own approval",          true;
    discard,              "Discard pending comment",       true;
    resolve,              "Resolve / unresolve thread",    true;
    accept_suggestion,    "Accept suggestion",             true;
    visual,               "Visual select mode",            false;

    // File picker
    picker_down,          "File picker: next",             false;
    picker_up,            "File picker: previous",         false;
}

// ── Handler functions ───────────────────────────────────────────────

mod cmd {
    use super::*;

    pub fn quit(app: &mut App) {
        app.should_quit = true;
    }

    pub fn escape(app: &mut App) {
        if app.diff_view.is_visual_mode() {
            app.diff_view.cancel_visual();
        } else if app.diff_view.search.is_active() {
            app.diff_view.search.clear();
            app.status_msg.clear();
        } else {
            app.should_quit = true;
        }
    }

    pub fn open_browser(app: &mut App) {
        app.open_in_browser();
    }

    pub fn scroll_down(app: &mut App) {
        app.diff_view.scroll_down(1);
    }

    pub fn scroll_up(app: &mut App) {
        app.diff_view.scroll_up(1);
    }

    pub fn half_page_down(app: &mut App) {
        let h = app.visible_height;
        app.diff_view.page_down(h / 2);
    }

    pub fn half_page_up(app: &mut App) {
        let h = app.visible_height;
        app.diff_view.page_up(h / 2);
    }

    pub fn full_page_down(app: &mut App) {
        let h = app.visible_height;
        app.diff_view.page_down(h);
    }

    pub fn full_page_up(app: &mut App) {
        let h = app.visible_height;
        app.diff_view.page_up(h);
    }

    pub fn goto_first(app: &mut App) {
        app.diff_view.goto_first();
    }

    pub fn goto_last(app: &mut App) {
        app.diff_view.goto_last();
    }

    pub fn screen_top(app: &mut App) {
        app.diff_view.screen_top();
    }

    pub fn screen_middle(app: &mut App) {
        let h = app.visible_height;
        app.diff_view.screen_middle(h);
    }

    pub fn screen_bottom(app: &mut App) {
        let h = app.visible_height;
        app.diff_view.screen_bottom(h);
    }

    pub fn center_cursor(app: &mut App) {
        let h = app.visible_height;
        app.diff_view.center_cursor(h);
    }

    pub fn scroll_cursor_top(app: &mut App) {
        app.diff_view.scroll_offset = app.diff_view.cursor;
    }

    pub fn scroll_cursor_bottom(app: &mut App) {
        app.diff_view.scroll_offset = app
            .diff_view
            .cursor
            .saturating_sub(app.visible_height.saturating_sub(1));
    }

    pub fn next_hunk(app: &mut App) {
        app.diff_view.next_hunk();
    }

    pub fn prev_hunk(app: &mut App) {
        app.diff_view.prev_hunk();
    }

    pub fn next_change(app: &mut App) {
        app.diff_view.next_change();
    }

    pub fn prev_change(app: &mut App) {
        app.diff_view.prev_change();
    }

    pub fn next_match_or_file(app: &mut App) {
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

    pub fn prev_match_or_file(app: &mut App) {
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

    pub fn search_forward(app: &mut App) {
        app.diff_view.search.set_anchor(app.diff_view.cursor);
        app.search_bar.open(SearchDirection::Forward);
    }

    pub fn search_backward(app: &mut App) {
        app.diff_view.search.set_anchor(app.diff_view.cursor);
        app.search_bar.open(SearchDirection::Backward);
    }

    pub fn help(app: &mut App) {
        app.show_help = !app.show_help;
    }

    pub fn switch_focus(app: &mut App) {
        app.focus = match app.focus {
            Focus::FilePicker => Focus::DiffView,
            Focus::DiffView => Focus::FilePicker,
        };
    }

    pub fn toggle_view(app: &mut App) {
        app.diff_view.toggle_mode();
    }

    pub fn toggle_comment(app: &mut App) {
        if app.diff_view.toggle_comment_expand() {
            app.rebuild_display();
        }
    }

    pub fn file_filter(app: &mut App) {
        app.file_picker.start_filter();
    }

    pub fn open_command_mode(app: &mut App) {
        app.command_bar.open();
    }

    pub fn pending_g(app: &mut App) {
        app.pending_key = Some('g');
    }

    pub fn pending_z(app: &mut App) {
        app.pending_key = Some('z');
    }

    pub fn comment(app: &mut App) {
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

    pub fn suggest(app: &mut App) {
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

    pub fn expand(app: &mut App) {
        app.request_expand();
    }

    pub fn approve(app: &mut App) {
        app.review_confirm
            .show(ReviewEvent::Approve, app.pending_comments.len());
    }

    pub fn request_changes(app: &mut App) {
        app.review_confirm
            .show(ReviewEvent::RequestChanges, app.pending_comments.len());
    }

    pub fn submit(app: &mut App) {
        app.review_confirm
            .show(ReviewEvent::Comment, app.pending_comments.len());
    }

    pub fn unapprove(app: &mut App) {
        app.review_confirm.show(ReviewEvent::Unapprove, 0);
    }

    pub fn discard(app: &mut App) {
        if let Some(pt) = app.diff_view.pending_comment_at_cursor()
            && pt.pending_idx < app.pending_comments.len()
        {
            app.pending_comments.remove(pt.pending_idx);
            app.rebuild_display();
        }
    }

    pub fn resolve(app: &mut App) {
        if let Some(target) = app.diff_view.thread_resolve_target() {
            app.toggle_resolve_thread(target.thread_node_id, target.is_resolved);
        }
    }

    pub fn accept_suggestion(app: &mut App) {
        if let Some(target) = app.diff_view.suggestion_at_cursor() {
            app.accept_suggestion(target);
        }
    }

    pub fn visual(app: &mut App) {
        if app.diff_view.is_visual_mode() {
            app.diff_view.cancel_visual();
        } else {
            app.diff_view.start_visual();
        }
    }

    pub fn picker_down(app: &mut App) {
        app.file_picker.next();
        app.diff_view.goto_file(app.file_picker.selected);
    }

    pub fn picker_up(app: &mut App) {
        app.file_picker.prev();
        app.diff_view.goto_file(app.file_picker.selected);
    }
}
