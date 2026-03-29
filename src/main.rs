mod app;
mod cli;
mod components;
mod config;
mod diff;
mod dirs;
mod editor;
mod event;
mod gh;
pub mod stack;
mod highlight;
mod search;
mod terminal;
mod theme;
mod types;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    execute,
    event::{
        KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    let (repo, pr_number) = cli::resolve(cli.args)?;

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(std::io::stdout(), PopKeyboardEnhancementFlags);
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        original_hook(info);
    }));

    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen).context("Failed to enter alternate screen")?;
    let _ = execute!(
        stdout,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
    );
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("Failed to create terminal")?;

    let result = run_app(&mut terminal, repo, pr_number).await;

    let _ = execute!(terminal.backend_mut(), PopKeyboardEnhancementFlags);
    disable_raw_mode().context("Failed to disable raw mode")?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .context("Failed to leave alternate screen")?;
    terminal.show_cursor().context("Failed to show cursor")?;

    if let Err(ref e) = result {
        eprintln!("Error: {e:#}");
    }

    result
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    repo: String,
    pr_number: u64,
) -> Result<()> {
    let mut events = event::EventHandler::new();
    let tx = events.sender();
    let mut app = app::App::new(repo, pr_number, tx);

    app.start_loading();

    loop {
        terminal.draw(|frame| app.draw(frame))?;

        if app.should_quit() {
            break;
        }

        if app.diff_view.is_animating() {
            tokio::select! {
                biased;
                event = events.next() => {
                    app.diff_view.finish_animation();
                    if let Some(event) = event {
                        let action = app.handle_event(event);
                        terminal::handle_action(terminal, &events, &mut app, action)?;
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(16)) => {
                    app.diff_view.step_animation();
                }
            }
        } else if let Some(event) = events.next().await {
            let action = app.handle_event(event);
            terminal::handle_action(terminal, &events, &mut app, action)?;
        }
    }

    events.stop();
    Ok(())
}
