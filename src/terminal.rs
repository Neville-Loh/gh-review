use anyhow::Result;
use crossterm::{
    execute,
    event::{
        KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::{app, editor, event, types};

pub type Term = Terminal<CrosstermBackend<std::io::Stdout>>;

pub fn suspend(terminal: &mut Term) {
    let _ = execute!(terminal.backend_mut(), PopKeyboardEnhancementFlags);
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = disable_raw_mode();
}

pub fn resume(terminal: &mut Term) -> Result<()> {
    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    let _ = execute!(
        terminal.backend_mut(),
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
    );
    terminal.clear()?;
    Ok(())
}

pub fn handle_action(
    terminal: &mut Term,
    events: &event::EventHandler,
    app: &mut app::App,
    action: app::Action,
) -> Result<()> {
    match action {
        app::Action::None => {}
        app::Action::OpenEditor {
            file_path,
            line,
            side,
            start_line,
            start_side,
            content,
            file_ext,
        } => {
            events.pause();
            suspend(terminal);
            let result = editor::edit_in_external(&content, &file_ext);
            resume(terminal)?;
            events.resume();

            match result {
                Ok(edited) if edited.trim() != content.trim() => {
                    let body = format!("```suggestion\n{edited}```");
                    app.pending_comments.push(types::ReviewComment {
                        path: file_path,
                        line,
                        side,
                        body,
                        start_line,
                        start_side,
                    });
                    app.rebuild_display();
                    app.status_msg = "Suggestion added".to_string();
                    app.status_is_error = false;
                }
                Ok(_) => {
                    app.status_msg = "No changes made".to_string();
                    app.status_is_error = false;
                }
                Err(e) => {
                    app.status_msg = format!("Editor failed: {e}");
                    app.status_is_error = true;
                }
            }
        }
    }
    Ok(())
}
