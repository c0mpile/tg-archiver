use crate::app::{ActiveView, App};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub mod archive_progress;
pub mod filter_config;
pub mod monitoring;
pub mod upload;

pub fn render(f: &mut Frame, app: &mut App) {
    match app.active_view {
        ActiveView::Home => render_home(f, app),
        ActiveView::ChannelSelect => render_channel_select(f, app),
        ActiveView::GroupSelect => render_group_select(f, app),
        ActiveView::TopicSelect => render_topic_select(f, app),
        ActiveView::FilterConfig => filter_config::render_filter_config(f, app),
        ActiveView::ArchiveProgress => archive_progress::draw(f, app),
        ActiveView::ResumePrompt => render_resume_prompt(f, app),
        ActiveView::Monitoring => monitoring::render_monitoring(f, app),
        ActiveView::DeletePairPrompt => monitoring::render_delete_prompt(f, app),
        ActiveView::IntervalConfig => monitoring::render_interval_config(f, app),
        ActiveView::UploadModeSelect => upload::render_upload_mode_select(f, app),
        ActiveView::UploadSyncResume => upload::render_upload_sync_resume(f, app),
        ActiveView::UploadFileSelect => upload::render_upload_file_select(f, app),
        ActiveView::UploadGroupSelect => upload::render_upload_group_select(f, app),
        ActiveView::UploadTopicSelect => upload::render_upload_topic_select(f, app),
        ActiveView::UploadTopicNameEntry => upload::render_upload_topic_name_entry(f, app),
        ActiveView::UploadProgress => upload::render_upload_progress(f, app),
    }
}

fn render_home(f: &mut Frame, app: &mut App) {
    let size = f.area();
    let block = Block::default()
        .title("tg-archiver")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let channel_title = &app.state().channel_pairs[app.active_pair_index].source_channel_title;
    let channel_text = if channel_title.is_empty() {
        "None"
    } else {
        channel_title
    };
    let group_title = &app.state().channel_pairs[app.active_pair_index].dest_group_title;
    let group_text = if group_title.is_empty() {
        "None"
    } else {
        group_title
    };
    let topic_text = app.state().channel_pairs[app.active_pair_index]
        .dest_topic_title
        .as_deref()
        .unwrap_or("None");

    let mut lines = vec![
        Line::from("Welcome to tg-archiver shell."),
        Line::from(""),
        Line::from(format!("1. Source Channel: {}", channel_text)),
        Line::from(format!(
            "2. Destination Group: {} (Topic: {})",
            group_text, topic_text
        )),
        Line::from(""),
        Line::from("Press '1' to set source channel."),
        Line::from("Press '3' to configure threshold."),
        Line::from("Press 's' to start archive."),
        Line::from("Press 'm' to enter monitoring mode."),
        Line::from("Press 'q' or Ctrl-C to quit."),
    ];

    if let Some(err) = &app.home_error {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                "Error: ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(err, Style::default().fg(Color::Red)),
        ]));
    }

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, size);
}

fn render_channel_select(f: &mut Frame, app: &mut App) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Min(1), Constraint::Length(2)].as_ref())
        .split(size);

    let block = Block::default()
        .title("Select Source Channel")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let items: Vec<ListItem> = app
        .available_channels
        .iter()
        .map(|(id, title)| ListItem::new(Line::from(Span::raw(format!("{}  {}", id, title)))))
        .collect();

    if app.is_loading_channels {
        let p = Paragraph::new("Loading channels...").block(block);
        f.render_widget(p, chunks[0]);
    } else if items.is_empty() {
        let mut lines = vec![Line::from("No channels available.")];
        if let Some(err) = &app.resolution_error {
            lines.push(Line::from(vec![
                Span::styled(
                    "Error: ",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::styled(err, Style::default().fg(Color::Red)),
            ]));
        }
        let p = Paragraph::new(lines).block(block);
        f.render_widget(p, chunks[0]);
    } else {
        let list = List::new(items).block(block).highlight_style(
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        );
        f.render_stateful_widget(list, chunks[0], &mut app.channel_list_state);
    }

    let help_text =
        Paragraph::new("Use Up/Down arrows to select, Enter to confirm, Esc to cancel.")
            .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help_text, chunks[1]);
}

fn render_group_select(f: &mut Frame, app: &mut App) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Min(1), Constraint::Length(2)].as_ref())
        .split(size);

    let block = Block::default()
        .title("Select Destination Group")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let items: Vec<ListItem> = app
        .available_groups
        .iter()
        .map(|(id, title)| ListItem::new(Line::from(Span::raw(format!("{}  {}", id, title)))))
        .collect();

    if app.is_loading_groups {
        let p = Paragraph::new("Loading groups...").block(block);
        f.render_widget(p, chunks[0]);
    } else if items.is_empty() {
        let mut lines = vec![Line::from("No groups available.")];
        if let Some(err) = &app.resolution_error {
            lines.push(Line::from(vec![
                Span::styled(
                    "Error: ",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::styled(err, Style::default().fg(Color::Red)),
            ]));
        }
        let p = Paragraph::new(lines).block(block);
        f.render_widget(p, chunks[0]);
    } else {
        let list = List::new(items).block(block).highlight_style(
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        );
        f.render_stateful_widget(list, chunks[0], &mut app.group_list_state);
    }

    let help_text =
        Paragraph::new("Use Up/Down arrows to select, Enter to confirm, Esc to cancel.")
            .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help_text, chunks[1]);
}

fn render_topic_select(f: &mut Frame, app: &mut App) {
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

    let mut items: Vec<ListItem> = app
        .available_topics
        .iter()
        .map(|(id, title)| ListItem::new(Line::from(Span::raw(format!("{}  {}", id, title)))))
        .collect();

    items.insert(
        0,
        ListItem::new(Line::from(Span::styled(
            "<Create new topic named automatically>",
            Style::default().fg(Color::Yellow),
        ))),
    );

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
        let p = Paragraph::new(lines).block(block);
        f.render_widget(p, chunks[0]);
    } else {
        let list = List::new(items).block(block).highlight_style(
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        );
        f.render_stateful_widget(list, chunks[0], &mut app.topic_list_state);
    }

    let help_text =
        Paragraph::new("Use Up/Down arrows to select, Enter to confirm, Esc to cancel.")
            .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help_text, chunks[1]);
}

fn render_resume_prompt(f: &mut Frame, _app: &App) {
    let size = f.area();
    let block = Block::default()
        .title("Resume Previous Archive?")
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    let content = "An unfinished archive session was detected.\n\n\
        Would you like to resume from where it left off?\n\
        Files that are partially downloaded will be restarted from scratch, but completed files will be skipped.\n\n\
        Press 'y' or Enter to Resume.\n\
        Press 'n' to Start Fresh (clears previous progress).";

    let paragraph = Paragraph::new(content)
        .block(block)
        .style(Style::default().fg(Color::White));
    f.render_widget(paragraph, size);
}
