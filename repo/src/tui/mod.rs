use crate::app::App;
use ratatui::{
    Frame,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

pub fn render(f: &mut Frame, app: &App) {
    let size = f.area();

    let block = Block::default()
        .title("tg-archiver")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let content = format!(
        "Welcome to tg-archiver shell.\n\n\
        Configured api_id: {}\n\
        Pending downloads: {}\n\n\
        Press 'q' or Ctrl-C to quit.",
        app.config().tg_api_id,
        app.state().download_status.len()
    );

    let paragraph = Paragraph::new(content).block(block);
    f.render_widget(paragraph, size);
}
