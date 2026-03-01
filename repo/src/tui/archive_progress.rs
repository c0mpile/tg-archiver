use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::app::App;
use crate::state::DownloadStatus;

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(f.area());

    let title = Paragraph::new("Archive Progress")
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Active Downloads
    let mut items = Vec::new();
    let statuses = &app.state.download_status;

    // We only show a limited number. The user plan said:
    // Render overall progress and list of active downloads with statuses

    let mut in_progress = 0;
    let mut completed = 0;
    let mut failed = 0;
    let mut skipped = 0;

    for (msg_id, status) in statuses.iter() {
        match status {
            DownloadStatus::InProgress { bytes_received } => {
                in_progress += 1;
                items.push(ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("Msg {}: ", msg_id),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::raw(format!("Downloading... {} bytes", bytes_received)),
                ])));
            }
            DownloadStatus::Complete { .. } => completed += 1,
            DownloadStatus::Uploaded => completed += 1,
            DownloadStatus::Failed { reason } => {
                failed += 1;
                items.push(ListItem::new(Line::from(vec![
                    Span::styled(format!("Msg {}: ", msg_id), Style::default().fg(Color::Red)),
                    Span::raw(format!("Failed: {}", reason)),
                ])));
            }
            DownloadStatus::Skipped => skipped += 1,
            DownloadStatus::Pending => {}
        }
    }

    let active_list = List::new(items).block(
        Block::default()
            .title("Active Downloads & Errors")
            .borders(Borders::ALL),
    );
    f.render_widget(active_list, chunks[1]);

    // Summary Footer
    let summary = format!(
        "Processed messages: {} | Completed: {} | Skipped: {} | Failed: {} | Active: {}",
        app.state.message_cursor.unwrap_or(0),
        completed,
        skipped,
        failed,
        in_progress
    );
    let footer = Paragraph::new(summary)
        .style(Style::default().fg(Color::Green))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);
}
