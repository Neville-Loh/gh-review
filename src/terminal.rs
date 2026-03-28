use anyhow::Result;
use crossterm::{
    execute,
    event::{
        KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::{app, components::description_panel::CursorRegion, editor, event, types};

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
                    app.status.success("Suggestion added");
                }
                Ok(_) => {
                    app.status.info("No changes made");
                }
                Err(e) => {
                    app.status.error(format!("Editor failed: {e}"));
                }
            }
        }
        app::Action::EditDescription { region, content } => {
            events.pause();
            suspend(terminal);
            let result = editor::edit_in_external(&content, "md");
            resume(terminal)?;
            events.resume();

            match result {
                Ok(edited) if edited.trim() != content.trim() => {
                    let (field, display) = match region {
                        CursorRegion::Title => ("title", "title"),
                        CursorRegion::Body => ("body", "description"),
                    };
                    let value = edited.trim().to_string();
                    match region {
                        CursorRegion::Title => app.description_panel.title = value.clone(),
                        CursorRegion::Body => app.description_panel.body = value.clone(),
                    }
                    app.description_panel.rebuild_content(app.description_panel.last_width.max(60));

                    let tx = app.tx.clone();
                    let repo = app.repo.clone();
                    let pr = app.pr_number;
                    let field = field.to_string();
                    app.status.info(format!("Updating {display}..."));
                    tokio::spawn(async move {
                        match crate::gh::update_pr(&repo, pr, &field, &value).await {
                            Ok(()) => {
                                let _ = tx.send(crate::event::AppEvent::CustomActionComplete {
                                    description: format!("PR {display} updated"),
                                    result: Ok(()),
                                });
                            }
                            Err(e) => {
                                let _ = tx.send(crate::event::AppEvent::Error(
                                    format!("Failed to update {display}: {e}"),
                                ));
                            }
                        }
                    });
                }
                Ok(_) => {
                    app.status.info("No changes made");
                }
                Err(e) => {
                    app.status.error(format!("Editor failed: {e}"));
                }
            }
        }
    }
    Ok(())
}
