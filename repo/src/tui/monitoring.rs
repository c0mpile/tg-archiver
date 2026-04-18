use crate::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table},
};

pub fn render_monitoring(f: &mut Frame, app: &mut App) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3), // Header/Countdown
                Constraint::Min(5),    // Table
                Constraint::Length(2), // Help
            ]
            .as_ref(),
        )
        .split(size);

    // Header: Interval & Countdown
    let mut header_text = format!("Poll Interval: {}s", app.state.poll_interval_secs);
    if let Some(next_tick) = app.next_tick_at {
        let now = std::time::Instant::now();
        let remaining = if next_tick > now {
            (next_tick - now).as_secs()
        } else {
            0
        };
        header_text = format!("{} | Next poll in: {}s", header_text, remaining);
    }

    let header = Paragraph::new(header_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Monitoring Mode"),
        )
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(header, chunks[0]);

    // Table
    let header_cells = ["Source", "Destination", "Last ID"]
        .iter()
        .map(|h| ratatui::widgets::Cell::from(*h).style(Style::default().fg(Color::Cyan)));
    let header_row = Row::new(header_cells)
        .style(Style::default().bg(Color::DarkGray))
        .height(1)
        .bottom_margin(1);

    let rows = app.state.channel_pairs.iter().enumerate().map(|(i, pair)| {
        let source = if pair.source_channel_title.is_empty() {
            "None".to_string()
        } else {
            pair.source_channel_title.clone()
        };
        let dest = pair.dest_topic_title.clone().unwrap_or_else(|| {
            if pair.dest_group_title.is_empty() {
                "None".to_string()
            } else {
                pair.dest_group_title.clone()
            }
        });
        let last_id = pair
            .last_forwarded_message_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "0".to_string());

        let style = if i == app.active_pair_index {
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        Row::new(vec![source, dest, last_id]).style(style)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(40),
            Constraint::Percentage(40),
            Constraint::Percentage(20),
        ],
    )
    .header(header_row)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Channel Pairs"),
    );

    f.render_widget(table, chunks[1]);

    // Help
    let help_text = Paragraph::new("Up/Down: Select Pair | a: Add | d: Delete | s: Force Sync | i: Set Interval | q: Exit Monitoring")
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help_text, chunks[2]);
}

pub fn render_delete_prompt(f: &mut Frame, _app: &mut App) {
    let size = f.area();
    let block = Block::default()
        .title("Delete Pair?")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));

    let content = "Are you sure you want to delete this channel pair?\n\n\
        Press 'y' or Enter to Delete.\n\
        Press 'n' or Esc to Cancel.";

    let paragraph = Paragraph::new(content)
        .block(block)
        .style(Style::default().fg(Color::White));
    f.render_widget(paragraph, size);
}

pub fn render_interval_config(f: &mut Frame, app: &mut App) {
    let size = f.area();
    let block = Block::default()
        .title("Set Poll Interval")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let mut text = vec![
        Line::from("Enter new poll interval in seconds (minimum 60):"),
        Line::from(""),
        Line::from(format!("> {}_", app.interval_config_state.interval_secs)),
    ];

    if let Some(err) = &app.interval_config_state.error_message {
        text.push(Line::from(""));
        text.push(Line::from(vec![Span::styled(
            format!("Error: {}", err),
            Style::default().fg(Color::Red),
        )]));
    }

    let p = Paragraph::new(text).block(block);
    f.render_widget(p, size);
}
