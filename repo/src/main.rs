mod app;
mod archive;
pub mod config;
mod error;
pub mod state;
mod telegram;
mod tui;

use app::{App, AppEvent};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::time::Duration;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match dotenvy::dotenv() {
        Err(e) if !e.not_found() => eprintln!("Error loading .env file: {}", e),
        _ => {}
    }

    let config = config::Config::from_env();

    // Ensure state dir is created
    let state_dir = std::env::var("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").expect("HOME env var not set");
            std::path::PathBuf::from(home).join(".local/state")
        })
        .join("tg-archiver");

    tokio::fs::create_dir_all(&state_dir).await?;

    let state = match state::State::load().await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to load previous state: {}", e);
            eprintln!("To proceed, fix state.json or delete it to reset.");
            std::process::exit(1);
        }
    };

    let app = App::new(config, state);

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app_result = run_app(app, &mut terminal).await;

    // Guaranteed cleanup
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    app_result?;

    Ok(())
}

async fn run_app(
    mut app: App,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::channel(128);
    let tick_rate = Duration::from_millis(250);

    let tx_clone = tx.clone();
    tokio::task::spawn_blocking(move || {
        let mut last_tick = std::time::Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            #[allow(clippy::collapsible_if)]
            if crossterm::event::poll(timeout).expect("failed to poll event") {
                if let Event::Key(key) = event::read().expect("failed to read event") {
                    if tx_clone.blocking_send(AppEvent::Input(key)).is_err() {
                        return;
                    }
                }
            }
            if last_tick.elapsed() >= tick_rate {
                if tx_clone.blocking_send(AppEvent::Tick).is_err() {
                    return;
                }
                last_tick = std::time::Instant::now();
            }
        }
    });

    loop {
        terminal.draw(|f| tui::render(f, &app))?;

        if let Some(event) = rx.recv().await {
            app.handle_event(event);
        }

        if app.should_quit() {
            break;
        }
    }

    Ok(())
}
