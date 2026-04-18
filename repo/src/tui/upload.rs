use crate::app::{App, UploadEntry};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
};

pub fn render_upload_mode_select(f: &mut Frame, _app: &mut App) {
    let size = f.area();
    let block = Block::default()
        .title("Select Upload Mode")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let text = "Choose an upload mode:\n\n\
        [S]elect files: Upload chosen files immediately without tracking.\n\
        [Y]nc (incremental sync): Track uploads in a state file to skip already-uploaded files.\n\n\
        Press 's' or 'y' to choose, or Esc to return Home.";

    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, size);
}

pub fn render_upload_sync_resume(f: &mut Frame, app: &mut App) {
    let size = f.area();
    let block = Block::default()
        .title("Resume Upload Sync?")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let state = app.upload_sync_state.as_ref().unwrap();
    let dest_group = &state.dest_group_title;
    let dest_topic = state.dest_topic_title.as_deref().unwrap_or("None");
    let count = state.uploaded_files.len();

    let text = format!(
        "Existing sync state found for this directory.\n\n\
        Destination: {} (Topic: {})\n\
        Already uploaded: {} files\n\n\
        Load this state?\n\
        Press 'y' or Enter to load state and skip destination picker.\n\
        Press 'n' to ignore state (start fresh without deleting existing state).\n\
        Press Esc to return Home.",
        dest_group, dest_topic, count
    );

    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, size);
}

pub fn render_upload_file_select(f: &mut Frame, app: &mut App) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Min(1), Constraint::Length(2)].as_ref())
        .split(size);

    let block = Block::default()
        .title(format!(
            "Select Files to Upload (Sort: {:?})",
            app.upload_sort
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let mut items = Vec::new();
    for (i, entry) in app.upload_entries.iter().enumerate() {
        let selected = app.upload_selected[i];
        let check = if selected { "[x]" } else { "[ ]" };
        let name = match entry {
            UploadEntry::File { name, .. } => name.clone(),
            UploadEntry::Dir { name, .. } => format!("{}/", name),
        };
        items.push(ListItem::new(Line::from(Span::raw(format!(
            "{} {}",
            check, name
        )))));
    }

    if items.is_empty() {
        let p = Paragraph::new("No files found.").block(block);
        f.render_widget(p, chunks[0]);
    } else {
        let list = List::new(items).block(block).highlight_style(
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        );
        f.render_stateful_widget(list, chunks[0], &mut app.upload_list_state);
    }

    let help_text = Paragraph::new(
        "Up/Down: navigate | Space: toggle | 'a': select all | 't': sort | Enter: confirm | Esc: back"
    ).style(Style::default().fg(Color::DarkGray));
    f.render_widget(help_text, chunks[1]);
}

pub fn render_upload_group_select(f: &mut Frame, app: &mut App) {
    // Reuse group selection logic from regular group select
    crate::tui::render_group_select(f, app);
}

pub fn render_upload_topic_select(f: &mut Frame, app: &mut App) {
    // Reuse topic selection logic from regular topic select, but with extra item
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
            "+ Enter topic name manually",
            Style::default().fg(Color::Yellow),
        ))),
    );

    if app.is_loading_groups {
        // reuse flag
        let p = Paragraph::new("Loading topics...").block(block);
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

pub fn render_upload_topic_name_entry(f: &mut Frame, app: &mut App) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
        .split(size);

    let block = Block::default()
        .title("Enter new topic name manually")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let mut display_text = app.upload_topic_name_input.clone();
    display_text.push('█'); // Cursor

    let p = Paragraph::new(display_text).block(block);
    f.render_widget(p, chunks[0]);

    if let Some(err) = &app.resolution_error {
        let p = Paragraph::new(format!("Error: {}", err)).style(Style::default().fg(Color::Red));
        f.render_widget(p, chunks[1]);
    } else {
        let help_text = Paragraph::new("Type name, Enter to confirm, Esc to cancel.")
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(help_text, chunks[1]);
    }
}

pub fn render_upload_progress(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Build constraint list dynamically: transcode panel only when active
    let mut constraints: Vec<Constraint> = Vec::new();
    if app.upload_is_transcoding {
        constraints.push(Constraint::Length(6));
    }
    constraints.push(Constraint::Length(3)); // file info
    constraints.push(Constraint::Length(3)); // upload gauge
    constraints.push(Constraint::Min(1)); // warnings / help

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(constraints)
        .split(size);

    // Chunk index offset: transcode panel occupies chunk[0] when present
    let offset = usize::from(app.upload_is_transcoding);

    // ── Transcode panel ───────────────────────────────────────────────────
    if app.upload_is_transcoding {
        let tc_block = Block::default()
            .title("Transcoding")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let stats_line = format!(
            "File: {}  |  FPS: {:.1}  Speed: {:.1}x  Encoded: {}",
            app.upload_transcode_filename,
            app.upload_transcode_fps,
            app.upload_transcode_speed,
            app.upload_transcode_time_encoded,
        );

        // Split inner area of the transcode block: stats row + gauge row
        let tc_inner = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)].as_ref())
            .split(tc_block.inner(chunks[0]));

        f.render_widget(tc_block, chunks[0]);
        f.render_widget(Paragraph::new(stats_line), tc_inner[0]);

        let tc_gauge = Gauge::default()
            .gauge_style(Style::default().fg(Color::Yellow).bg(Color::Black))
            .percent((app.upload_transcode_percent as u16).min(100))
            .label(format!("{:.1}%", app.upload_transcode_percent));
        f.render_widget(tc_gauge, tc_inner[1]);
    }

    // ── File info ─────────────────────────────────────────────────────────
    let block = Block::default()
        .title("Uploading Files")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let file_info = Paragraph::new(format!(
        "Current file: {}",
        app.upload_progress_current_file
    ))
    .block(block);
    f.render_widget(file_info, chunks[offset]);

    // ── Upload progress gauge ─────────────────────────────────────────────
    let percent = if app.upload_progress_total > 0 {
        ((app.upload_progress_current as f64 / app.upload_progress_total as f64) * 100.0) as u16
    } else {
        0
    };

    let gauge = Gauge::default()
        .block(Block::default().title("Progress").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
        .percent(percent.min(100))
        .label(format!(
            "{}/{}",
            app.upload_progress_current, app.upload_progress_total
        ));

    f.render_widget(gauge, chunks[offset + 1]);

    // ── Help / warnings ───────────────────────────────────────────────────
    let mut lines = vec![Line::from(
        "Press 'p' to pause/resume, 'q' or Esc to cancel.",
    )];

    if app.is_paused.load(std::sync::atomic::Ordering::SeqCst) {
        lines.push(Line::from(Span::styled(
            "UPLOAD PAUSED",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
    }

    if let Some(err) = &app.home_error {
        lines.push(Line::from(Span::styled(
            format!("ERROR: {}", err),
            Style::default().fg(Color::Red),
        )));
    }

    if !app.upload_warnings.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Warnings (Skipped files):",
            Style::default().fg(Color::Yellow),
        )));
        for w in &app.upload_warnings {
            lines.push(Line::from(w.as_str()));
        }
    }

    let p = Paragraph::new(lines).block(Block::default().borders(Borders::ALL));
    f.render_widget(p, chunks[offset + 2]);
}
