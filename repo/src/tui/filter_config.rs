use crate::app::{App, FilterConfigField};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub fn render_filter_config(f: &mut Frame, app: &App) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Min(1), Constraint::Length(2)].as_ref())
        .split(size);

    let block = Block::default()
        .title("Filter Configuration")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let state = &app.filter_config_state;

    let mut items = Vec::new();

    let fields = vec![
        (
            FilterConfigField::Video,
            format!("Video: {}", if state.filter_video { "[x]" } else { "[ ]" }),
        ),
        (
            FilterConfigField::Audio,
            format!("Audio: {}", if state.filter_audio { "[x]" } else { "[ ]" }),
        ),
        (
            FilterConfigField::Image,
            format!("Image: {}", if state.filter_image { "[x]" } else { "[ ]" }),
        ),
        (
            FilterConfigField::Archive,
            format!(
                "Archive: {}",
                if state.filter_archive { "[x]" } else { "[ ]" }
            ),
        ),
        (
            FilterConfigField::IncludeText,
            format!(
                "Text Descriptions: {}",
                if state.include_text_descriptions {
                    "[x]"
                } else {
                    "[ ]"
                }
            ),
        ),
        (
            FilterConfigField::MinSize,
            format!(
                "Min Size (MB): {}{}",
                state.min_size_mb,
                if state.editing && state.selected_field == FilterConfigField::MinSize {
                    "█"
                } else {
                    ""
                }
            ),
        ),
        (
            FilterConfigField::PostCount,
            format!(
                "Post Count Threshold: {}{}",
                state.post_count_threshold,
                if state.editing && state.selected_field == FilterConfigField::PostCount {
                    "█"
                } else {
                    ""
                }
            ),
        ),
        (
            FilterConfigField::DownloadPath,
            format!(
                "Download Path: {}{}",
                state.local_download_path,
                if state.editing && state.selected_field == FilterConfigField::DownloadPath {
                    "█"
                } else {
                    ""
                }
            ),
        ),
        (FilterConfigField::Save, "Save & Exit".to_string()),
    ];

    for (field, text) in fields {
        let mut style = Style::default();
        if field == state.selected_field {
            if state.editing {
                style = style
                    .bg(Color::Red)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD);
            } else {
                style = style
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD);
            }
        }
        items.push(ListItem::new(Line::from(Span::styled(text, style))));
    }

    if let Some(err) = &state.error_message {
        items.push(ListItem::new(Line::from(""))); // Blank padding
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                "Error: ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(err, Style::default().fg(Color::Red)),
        ])));
    }

    let list = List::new(items).block(block);
    f.render_widget(list, chunks[0]);

    let help_msg = if state.editing {
        "Typing... Press Enter to finish."
    } else {
        "Up/Down to select, Enter to toggle or edit, Esc to cancel."
    };
    let help_text = Paragraph::new(help_msg).style(Style::default().fg(Color::DarkGray));
    f.render_widget(help_text, chunks[1]);
}
