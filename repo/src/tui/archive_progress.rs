use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;

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

    let title_text = if app.is_paused.load(std::sync::atomic::Ordering::Relaxed) {
        "Archive Progress [PAUSED]"
    } else {
        "Archive Progress"
    };

    let title = Paragraph::new(title_text)
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(title, chunks[0]);

    // Console Log Block
    let log_lines: Vec<Line> = app
        .archive_progress_state
        .logs
        .iter()
        .map(|log| Line::from(format!("[{}] {}", log.timestamp, log.msg)))
        .collect();

    let logs_len = log_lines.len();
    let visible_height = chunks[1].height.saturating_sub(2) as usize;
    let max_offset = logs_len.saturating_sub(visible_height);
    let scroll_offset = app.archive_progress_state.scroll_offset.min(max_offset);

    let log_widget = Paragraph::new(log_lines)
        .block(Block::default().title("Activity Log").borders(Borders::ALL))
        .scroll((scroll_offset as u16, 0));

    f.render_widget(log_widget, chunks[1]);

    // Summary Footer
    let current_id = app.state.last_forwarded_message_id.unwrap_or(0);
    let highest_id = app.archive_progress_state.highest_msg_id;

    let footer_text = if app.archive_progress_state.completed {
        "Complete — all messages forwarded. Press 'q' to return to main menu or 'r' to run again."
            .to_string()
    } else {
        let percent = if highest_id > 0 {
            (current_id as f64 / highest_id as f64 * 100.0).clamp(0.0, 100.0) as u32
        } else {
            0
        };
        format!(
            "Forwarded up to ID: {} / {}  [{}%]",
            current_id, highest_id, percent
        )
    };

    let footer_color = if app.archive_progress_state.completed {
        Color::Cyan
    } else {
        Color::Green
    };

    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(footer_color))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);
}
