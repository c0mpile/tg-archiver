mod app;
mod archive;
pub mod config;
mod error;
pub mod state;
mod telegram;
mod tui;

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

    println!("Loaded config: api_id={}", config.tg_api_id);
    println!(
        "Loaded state: {} pending downloads",
        state.download_status.len()
    );

    Ok(())
}
