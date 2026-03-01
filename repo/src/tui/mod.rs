use crate::app::{ActiveView, App};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub fn render(f: &mut Frame, app: &App) {
    match app.active_view {
        ActiveView::Home => render_home(f, app),
        ActiveView::ChannelSelect => render_input(f, app, "Select Source Channel"),
        ActiveView::GroupSelect => render_input(f, app, "Select Destination Group"),
        ActiveView::TopicSelect => render_topic_select(f, app),
    }
}

fn render_home(f: &mut Frame, app: &App) {
    let size = f.area();
    let block = Block::default()
        .title("tg-archiver")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let channel_text = app
        .state()
        .source_channel_title
        .as_deref()
        .unwrap_or("None");
    let group_text = app.state().dest_group_title.as_deref().unwrap_or("None");
    let topic_text = app.state().dest_topic_title.as_deref().unwrap_or("None");

    let content = format!(
        "Welcome to tg-archiver shell.\n\n\
        1. Source Channel: {}\n\
        2. Destination Group: {} (Topic: {})\n\n\
        Press '1' to set source channel.\n\
        Press '2' to set destination group & topic.\n\
        Press 'q' or Ctrl-C to quit.",
        channel_text, group_text, topic_text
    );

    let paragraph = Paragraph::new(content).block(block);
    f.render_widget(paragraph, size);
}

fn render_input(f: &mut Frame, app: &App, title: &str) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Length(4), Constraint::Min(1)].as_ref())
        .split(size);

    let input_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let mut lines = vec![Line::from(format!("> {}", app.input_buffer))];

    // Display error message in bold red if present
    if let Some(err) = &app.resolution_error {
        lines.push(Line::from(vec![
            Span::styled(
                "Error: ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(err, Style::default().fg(Color::Red)),
        ]));
    }

    let paragraph = Paragraph::new(lines).block(input_block);
    f.render_widget(paragraph, chunks[0]);

    let help_text = Paragraph::new("Press Enter to submit, Esc to cancel.")
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help_text, chunks[1]);
}

fn render_topic_select(f: &mut Frame, app: &App) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Min(1), Constraint::Length(2)].as_ref())
        .split(size);

    let block = Block::default()
        .title("Select Topic")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let items: Vec<ListItem> = app
        .available_topics
        .iter()
        .enumerate()
        .map(|(i, (_id, title))| {
            let style = if i == app.selected_topic_index {
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(Span::styled(title.clone(), style)))
        })
        .collect();

    // If there are no topics, show a message instead
    if items.is_empty() {
        let mut lines = vec![Line::from("No topics available or still loading...")];
        if let Some(err) = &app.resolution_error {
            lines.push(Line::from(vec![
                Span::styled(
                    "Error: ",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::styled(err, Style::default().fg(Color::Red)),
            ]));
        }
        let p = Paragraph::new(lines).block(block.clone());
        f.render_widget(p, chunks[0]);
    } else {
        let list = List::new(items).block(block);
        f.render_widget(list, chunks[0]);
    }

    let help_text =
        Paragraph::new("Use Up/Down arrows to select, Enter to confirm, Esc to cancel.")
            .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help_text, chunks[1]);
}
