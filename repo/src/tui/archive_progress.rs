use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
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

    // Active Status
    let total = app.source_message_count.unwrap_or(0);

    // We can't really track detailed progress anymore easily without traversing msg ids,
    // so we'll just show the last forwarded msg id.
    let current_id = app.state.channel_pairs[app.active_pair_index]
        .last_forwarded_message_id
        .unwrap_or(0);

    let info = vec![
        Line::from(vec![
            Span::styled(
                "Highest Source Message ID: ",
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(total.to_string()),
        ]),
        Line::from(vec![
            Span::styled(
                "Last Forwarded Message ID: ",
                Style::default().fg(Color::Green),
            ),
            Span::raw(current_id.to_string()),
        ]),
    ];

    let info_widget =
        Paragraph::new(info).block(Block::default().title("Job Status").borders(Borders::ALL));
    f.render_widget(info_widget, chunks[1]);

    // Summary Footer
    let summary = format!("Forwarded ID: {} / {}", current_id, total);
    let footer = Paragraph::new(summary)
        .style(Style::default().fg(Color::Green))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);
}
