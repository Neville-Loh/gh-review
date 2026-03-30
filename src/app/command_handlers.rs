//! Command handler functions — one per [`Command`] in the registry.
//!
//! Each function takes `&mut App` and performs a single action. Handlers are
//! panel-specific: diff handlers touch `app.diff_view`, description handlers
//! touch `app.description_panel`, etc. The keymap routes the right handler
//! to the right panel via [`ScopeBinding`].

use crate::search::SearchDirection;
use crate::types::ReviewEvent;

use super::App;

pub fn quit(app: &mut App) {
    app.should_quit = true;
}

pub fn escape(app: &mut App) {
    if app.diff_view.is_visual_mode() {
        app.diff_view.cancel_visual();
    } else if app.diff_view.search.is_active() {
        app.diff_view.search.clear();
        app.status.clear();
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
    let smooth = app.config.smooth_scroll;
    app.diff_view.page_down(h / 2, smooth);
}

pub fn half_page_up(app: &mut App) {
    let h = app.visible_height;
    let smooth = app.config.smooth_scroll;
    app.diff_view.page_up(h / 2, smooth);
}

pub fn full_page_down(app: &mut App) {
    let h = app.visible_height;
    let smooth = app.config.smooth_scroll;
    app.diff_view.page_down(h, smooth);
}

pub fn full_page_up(app: &mut App) {
    let h = app.visible_height;
    let smooth = app.config.smooth_scroll;
    app.diff_view.page_up(h, smooth);
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

pub fn next_comment(app: &mut App) {
    app.diff_view.next_comment();
}

pub fn prev_comment(app: &mut App) {
    app.diff_view.prev_comment();
}

pub fn next_paragraph(app: &mut App) {
    app.diff_view.next_paragraph();
}

pub fn prev_paragraph(app: &mut App) {
    app.diff_view.prev_paragraph();
}

pub fn next_match_or_file(app: &mut App) {
    step_match_or_file(app, true);
}

pub fn prev_match_or_file(app: &mut App) {
    step_match_or_file(app, false);
}

fn step_match_or_file(app: &mut App, forward: bool) {
    if app.diff_view.search.is_active() {
        let same_dir = matches!(
            (forward, app.search_bar.direction),
            (true, SearchDirection::Forward) | (false, SearchDirection::Backward)
        );
        let c = if same_dir {
            app.diff_view.search.next_match()
        } else {
            app.diff_view.search.prev_match()
        };
        if let Some(c) = c {
            app.diff_view.cursor = c;
        }
        app.update_search_status();
    } else if forward {
        app.diff_view.next_file();
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
    next_panel(app);
}

pub fn next_panel(app: &mut App) {
    let desc_open = app.description_panel.visible;
    app.focus = app.focus.next(desc_open);
}

pub fn prev_panel(app: &mut App) {
    let desc_open = app.description_panel.visible;
    app.focus = app.focus.prev(desc_open);
}

pub fn toggle_view(app: &mut App) {
    app.diff_view.toggle_mode();
    app.rebuild_display();
}

pub fn fold_toggle(app: &mut App) {
    if app.diff_view.fold_toggle() {
        app.rebuild_display();
    }
}

pub fn fold_open(app: &mut App) {
    if app.diff_view.fold_open() {
        app.rebuild_display();
    }
}

pub fn fold_close(app: &mut App) {
    if app.diff_view.fold_close() {
        app.rebuild_display();
    }
}

pub fn toggle_comment(app: &mut App) {
    let on_comment = matches!(
        app.diff_view.current_context(),
        crate::types::RowContext::Comment(_) | crate::types::RowContext::Suggestion(_)
    );
    if app.diff_view.toggle_comment_expand() || (!on_comment && app.diff_view.fold_toggle()) {
        app.rebuild_display();
    }
}

pub fn expand_all_comments(app: &mut App) {
    set_all_comments_expanded(app, true);
}

pub fn collapse_all_comments(app: &mut App) {
    set_all_comments_expanded(app, false);
}

fn set_all_comments_expanded(app: &mut App, expand: bool) {
    use crate::diff::renderer::DisplayRow;
    app.diff_view.expanded_threads.clear();
    app.diff_view.expanded_pending.clear();
    for row in &app.diff_view.display_rows {
        match row {
            // Threads toggle via expanded_threads set: resolved threads
            // default to collapsed, unresolved to expanded. Adding an id
            // to the set inverts the default. So to expand all, add resolved
            // ids; to collapse all, add unresolved ids.
            DisplayRow::CommentHeader {
                thread_root_id: Some(root_id),
                is_resolved,
                ..
            } if *is_resolved == expand => {
                app.diff_view.expanded_threads.insert(*root_id);
            }
            DisplayRow::CommentHeader {
                is_pending: true,
                pending_idx: Some(idx),
                ..
            } if expand => {
                app.diff_view.expanded_pending.insert(*idx);
            }
            _ => {}
        }
    }
    app.rebuild_display();
}

pub fn file_filter(app: &mut App) {
    app.file_picker.start_filter();
}

pub fn open_command_mode(app: &mut App) {
    app.command_bar.open();
}

pub fn config_path(app: &mut App) {
    let path = crate::dirs::config_dir().join("config.toml");
    app.status.info(format!("Config: {}", path.display()));
}

pub fn comment(app: &mut App) {
    app.review_confirm
        .show_with_body(ReviewEvent::Comment, app.pending_comments.len(), true);
}

pub fn suggest(app: &mut App) {
    let use_external = crate::editor::has_external_editor();

    if app.diff_view.is_visual_mode() {
        if let Some(content) = app.diff_view.visual_selection_content()
            && let Some((start, end)) = app.diff_view.visual_selection_targets()
            && let Some(file) = app.files.get(start.file_idx)
        {
            if use_external {
                let file_ext = file.path.rsplit('.').next().unwrap_or("txt").to_string();
                app.pending_action = Some(crate::app::Action::OpenEditor {
                    file_path: file.path.clone(),
                    line: end.line,
                    side: end.side,
                    start_line: Some(start.line),
                    start_side: Some(start.side),
                    content,
                    file_ext,
                });
            } else {
                app.comment_input.open_suggestion_range(
                    file.path.clone(),
                    start.line,
                    start.side,
                    end.line,
                    end.side,
                    &content,
                );
            }
        }
        app.diff_view.cancel_visual();
    } else if let Some(content) = app.diff_view.current_line_content()
        && let Some(target) = app.diff_view.current_line_info()
        && let Some(file) = app.files.get(target.file_idx)
    {
        if use_external {
            let clean = content.strip_prefix(' ').unwrap_or(&content).to_string();
            let file_ext = file.path.rsplit('.').next().unwrap_or("txt").to_string();
            app.pending_action = Some(crate::app::Action::OpenEditor {
                file_path: file.path.clone(),
                line: target.line,
                side: target.side,
                start_line: None,
                start_side: None,
                content: clean,
                file_ext,
            });
        } else {
            app.comment_input.open_suggestion(
                file.path.clone(),
                target.line,
                target.side,
                &content,
            );
        }
    }
}

pub fn expand(app: &mut App) {
    app.request_expand();
}

pub fn approve(app: &mut App) {
    app.review_confirm
        .show(ReviewEvent::Approve, app.pending_comments.len());
}

pub fn approve_with_comment(app: &mut App) {
    app.review_confirm
        .show_with_body(ReviewEvent::Approve, app.pending_comments.len(), true);
}

pub fn request_changes(app: &mut App) {
    app.review_confirm
        .show(ReviewEvent::RequestChanges, app.pending_comments.len());
}

pub fn request_changes_with_comment(app: &mut App) {
    app.review_confirm
        .show_with_body(ReviewEvent::RequestChanges, app.pending_comments.len(), true);
}

pub fn submit(app: &mut App) {
    app.review_confirm
        .show(ReviewEvent::Comment, app.pending_comments.len());
}

pub fn unapprove(app: &mut App) {
    app.review_confirm
        .show_with_body(ReviewEvent::Unapprove, 0, true);
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

pub fn comment_on_line(app: &mut App) {
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

pub fn description(app: &mut App) {
    if app.description_panel.visible {
        app.description_panel.visible = false;
        if app.focus == super::Focus::Description {
            app.focus = super::Focus::DiffView;
        }
    } else {
        app.description_panel.visible = true;
        app.focus = super::Focus::Description;
    }
}

pub fn edit_description(app: &mut App) {
    let region = app.description_panel.cursor_region();
    let content = match region {
        crate::components::description_panel::CursorRegion::Title => {
            app.description_panel.title.clone()
        }
        crate::components::description_panel::CursorRegion::Body => {
            app.description_panel.body.clone()
        }
    };
    app.pending_action = Some(super::Action::EditDescription { region, content });
}

// ── Description panel navigation ─────────────────────────────────────

pub fn desc_scroll_down(app: &mut App) {
    app.description_panel.cursor_down();
}

pub fn desc_scroll_up(app: &mut App) {
    app.description_panel.cursor_up();
}

pub fn desc_page_down(app: &mut App) {
    app.description_panel.half_page_down(app.visible_height);
}

pub fn desc_page_up(app: &mut App) {
    app.description_panel.half_page_up(app.visible_height);
}

pub fn desc_goto_first(app: &mut App) {
    app.description_panel.goto_top();
}

pub fn desc_goto_last(app: &mut App) {
    app.description_panel.goto_bottom();
}

pub fn desc_next_section(app: &mut App) {
    app.description_panel.next_section();
}

pub fn desc_prev_section(app: &mut App) {
    app.description_panel.prev_section();
}

pub fn desc_close(app: &mut App) {
    app.focus = super::Focus::DiffView;
}

pub fn stack_up(app: &mut App) {
    if let Some(pr) = app.stack.pr_above() {
        app.navigate_to_pr(pr);
    } else {
        app.status.error("Already at the top of the stack");
    }
}

pub fn stack_down(app: &mut App) {
    if let Some(pr) = app.stack.pr_below() {
        app.navigate_to_pr(pr);
    } else {
        app.status.error("Already at the bottom of the stack");
    }
}

pub fn lgtm(app: &mut App) {
    app.submit_review(ReviewEvent::Approve, "LGTM, ship it".to_string());
}
