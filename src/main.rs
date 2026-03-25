mod app;
mod components;
mod diff;
mod event;
mod gh;
mod search;
mod theme;
mod types;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

#[derive(Parser)]
#[command(
    name = "gh-review",
    about = "Terminal UI for reviewing GitHub pull requests"
)]
struct Cli {
    /// Repository in owner/repo format
    repo: String,

    /// Pull request number
    pr_number: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Panic hook to restore terminal even on crash
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        original_hook(info);
    }));

    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen).context("Failed to enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("Failed to create terminal")?;

    let result = run_app(&mut terminal, cli.repo, cli.pr_number).await;

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

        if let Some(event) = events.next().await {
            app.handle_event(event);
        }

        if app.should_quit() {
            break;
        }
    }

    events.stop();
    Ok(())
}
