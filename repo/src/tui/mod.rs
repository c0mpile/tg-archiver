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

pub fn render(f: &mut Frame, app: &mut App) {
    match app.active_view {
        ActiveView::Home => render_home(f, app),
        ActiveView::ChannelSelect => render_channel_select(f, app),
        ActiveView::GroupSelect => render_group_select(f, app),
        ActiveView::TopicSelect => render_topic_select(f, app),
        ActiveView::FilterConfig => filter_config::render_filter_config(f, app),
        ActiveView::ConfirmDownloadPath => render_confirm_download_path(f, app),
        ActiveView::ArchiveProgress => archive_progress::draw(f, app),
        ActiveView::ResumePrompt => render_resume_prompt(f, app),
    }
}

fn render_home(f: &mut Frame, app: &mut App) {
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
        Press '3' to configure filters & download path.\n\
        Press 'q' or Ctrl-C to quit.",
        channel_text, group_text, topic_text
    );

    let paragraph = Paragraph::new(content).block(block);
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

    let items: Vec<ListItem> = app
        .available_topics
        .iter()
        .map(|(id, title)| ListItem::new(Line::from(Span::raw(format!("{}  {}", id, title)))))
        .collect();

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

fn render_confirm_download_path(f: &mut Frame, app: &mut App) {
    let size = f.area();
    let block = Block::default()
        .title("Warning: Temporary Download Path")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));

    let content = format!(
        "Your download path is currently set to '{}'.\n\
        Files downloaded to /tmp are temporary and WILL BE DELETED by your OS on reboot.\n\n\
        Are you sure you want to continue archiving to this location?\n\n\
        Press 'y' or Enter to continue.\n\
        Press 'n' or Esc to cancel and return.",
        app.state().local_download_path
    );

    let paragraph = Paragraph::new(content)
        .block(block)
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(paragraph, size);
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
