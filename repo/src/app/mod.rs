use crate::config::Config;
use crate::state::State;
use crossterm::event::KeyEvent;
use std::sync::Arc;
use tokio::sync::mpsc;

pub enum AppEvent {
    Input(KeyEvent),
    Tick,
    ChannelsLoaded(Result<Vec<(i64, String)>, String>),
    GroupsLoaded(Result<Vec<(i64, String)>, String>),
    TopicsLoaded(Result<Vec<(i32, String)>, String>),
    FilterConfigNextField,
    FilterConfigPrevField,
    BeginEditField,
    TypeFilterChar(char),
    BackspaceFilterChar,
    EndEditField,
    CancelEditField,
    ExitFilterConfig,
    SaveFilterConfig,
    StartArchiveRun,
    ArchiveComplete,
    ArchiveError(String),
    SaveCursor(i32),
    TogglePause,
    PromptResumeResult(bool),
    TopicCreated(i32, String),
    ArchiveTotalCount(i32),
    MonitoringTick,
    PairSyncStarted {
        pair_index: usize,
    },
    PairSynced {
        pair_index: usize,
        last_forwarded_message_id: i32,
    },
    PairError {
        pair_index: usize,
        error: String,
    },
    EnterUpload,
    UploadFileToggled(usize),
    UploadSelectAll,
    UploadSortToggle,
    UploadModeSelected(UploadMode),
    UploadSyncStateFound(crate::upload::UploadSyncState),
    StartUploadRun,
    UploadFileComplete {
        filename: String,
        index: usize,
        total: usize,
    },
    UploadComplete,
    UploadError(String),
    UploadWarning(String),
    UploadTopicCreated(i32, String),
    TranscodeStarted {
        filename: String,
        index: usize,
        total: usize,
    },
    TranscodeProgress {
        filename: String,
        fps: f32,
        speed: f32,
        time_encoded: String,
        percent: f32,
    },
    TranscodeComplete {
        filename: String,
        mkv_path: std::path::PathBuf,
    },
    TranscodeError {
        filename: String,
        error: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum UploadEntry {
    File {
        name: String,
        path: std::path::PathBuf,
        size_bytes: u64,
        modified: std::time::SystemTime,
    },
    Dir {
        name: String,
        path: std::path::PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum UploadMode {
    #[default]
    Select,
    Sync,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum UploadSort {
    #[default]
    Alphabetical,
    ByModDate,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum PairStatus {
    #[default]
    Idle,
    Syncing,
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum ActiveView {
    #[default]
    Home,
    ChannelSelect,
    GroupSelect,
    TopicSelect,
    FilterConfig,
    ArchiveProgress,
    ResumePrompt,
    Monitoring,
    DeletePairPrompt,
    IntervalConfig,
    UploadFileSelect,
    UploadModeSelect,
    UploadSyncResume,
    UploadGroupSelect,
    UploadTopicSelect,
    UploadTopicNameEntry,
    UploadProgress,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum FilterConfigField {
    #[default]
    PostCount,
    Save, // Button to confirm and exit
}

impl FilterConfigField {
    pub fn next(&self) -> Self {
        match self {
            Self::PostCount => Self::Save,
            Self::Save => Self::Save,
        }
    }
    pub fn prev(&self) -> Self {
        match self {
            Self::PostCount => Self::PostCount,
            Self::Save => Self::PostCount,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FilterConfigState {
    pub selected_field: FilterConfigField,
    pub post_count_threshold: String,
    pub editing: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct IntervalConfigState {
    pub interval_secs: String,
    pub error_message: Option<String>,
}

pub struct App {
    #[allow(dead_code)]
    pub config: Config,
    pub state: State,
    pub should_quit: bool,
    pub active_view: ActiveView,
    pub resolution_error: Option<String>,
    pub home_error: Option<String>,
    pub available_channels: Vec<(i64, String)>,
    pub channel_list_state: ratatui::widgets::ListState,
    pub is_loading_channels: bool,
    pub available_groups: Vec<(i64, String)>,
    pub group_list_state: ratatui::widgets::ListState,
    pub is_loading_groups: bool,
    pub available_topics: Vec<(i32, String)>,
    pub topic_list_state: ratatui::widgets::ListState,
    pub filter_config_state: FilterConfigState,
    pub interval_config_state: IntervalConfigState,
    pub is_paused: Arc<std::sync::atomic::AtomicBool>,
    pub active_pair_index: usize,
    pub pair_statuses: Vec<PairStatus>,
    pub source_message_count: Option<i32>,
    pub monitoring_cancel_tx: Option<tokio::sync::watch::Sender<bool>>,
    pub next_tick_at: Option<std::time::Instant>,
    pub upload_cwd: std::path::PathBuf,
    pub upload_entries: Vec<UploadEntry>,
    pub upload_list_state: ratatui::widgets::ListState,
    pub upload_selected: Vec<bool>,
    pub upload_sort: UploadSort,
    pub upload_mode: Option<UploadMode>,
    pub upload_sync_state: Option<crate::upload::UploadSyncState>,
    pub upload_dest_group_id: Option<i64>,
    pub upload_dest_group_title: String,
    pub upload_dest_topic_id: Option<i32>,
    pub upload_dest_topic_title: Option<String>,
    pub upload_topic_name_input: String,
    pub upload_progress_current: usize,
    pub upload_progress_total: usize,
    pub upload_progress_current_file: String,
    pub upload_warnings: Vec<String>,
    pub upload_pause_tx: Option<tokio::sync::watch::Sender<bool>>,
    pub upload_cancel_tx: Option<tokio::sync::watch::Sender<()>>,
    pub upload_is_transcoding: bool,
    pub upload_transcode_filename: String,
    pub upload_transcode_fps: f32,
    pub upload_transcode_speed: f32,
    pub upload_transcode_time_encoded: String,
    pub upload_transcode_percent: f32,
}

impl App {
    pub fn new(config: Config, mut state: State) -> Self {
        if state.channel_pairs.is_empty() {
            state
                .channel_pairs
                .push(crate::state::ChannelPair::default());
        }
        let pair_statuses = vec![PairStatus::default(); state.channel_pairs.len()];
        let has_partial_state = state.channel_pairs[0].last_forwarded_message_id.is_some();
        Self {
            config,
            state,
            should_quit: false,
            active_view: if has_partial_state {
                ActiveView::ResumePrompt
            } else {
                ActiveView::Home
            },
            resolution_error: None,
            home_error: None,
            available_channels: Vec::new(),
            channel_list_state: ratatui::widgets::ListState::default(),
            is_loading_channels: false,
            available_groups: Vec::new(),
            group_list_state: ratatui::widgets::ListState::default(),
            is_loading_groups: false,
            available_topics: Vec::new(),
            topic_list_state: ratatui::widgets::ListState::default(),
            filter_config_state: FilterConfigState::default(),
            interval_config_state: IntervalConfigState::default(),
            is_paused: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            active_pair_index: 0,
            pair_statuses,
            source_message_count: None,
            monitoring_cancel_tx: None,
            next_tick_at: None,
            upload_cwd: std::path::PathBuf::new(),
            upload_entries: Vec::new(),
            upload_list_state: ratatui::widgets::ListState::default(),
            upload_selected: Vec::new(),
            upload_sort: UploadSort::default(),
            upload_mode: None,
            upload_sync_state: None,
            upload_dest_group_id: None,
            upload_dest_group_title: String::new(),
            upload_dest_topic_id: None,
            upload_dest_topic_title: None,
            upload_topic_name_input: String::new(),
            upload_progress_current: 0,
            upload_progress_total: 0,
            upload_progress_current_file: String::new(),
            upload_warnings: Vec::new(),
            upload_pause_tx: None,
            upload_cancel_tx: None,
            upload_is_transcoding: false,
            upload_transcode_filename: String::new(),
            upload_transcode_fps: 0.0,
            upload_transcode_speed: 0.0,
            upload_transcode_time_encoded: String::new(),
            upload_transcode_percent: 0.0,
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn handle_event(
        &mut self,
        event: AppEvent,
        telegram: &Arc<crate::telegram::TelegramClient>,
        tx: &mpsc::Sender<AppEvent>,
    ) {
        match event {
            AppEvent::Input(key) => {
                match self.active_view {
                    ActiveView::Home => match key.code {
                        crossterm::event::KeyCode::Char('q') => self.should_quit = true,
                        crossterm::event::KeyCode::Char('1') => {
                            self.active_view = ActiveView::ChannelSelect;
                            self.is_loading_channels = true;
                            self.available_channels.clear();
                            self.channel_list_state.select(Some(0));
                            let tg = Arc::clone(telegram);
                            let tx = tx.clone();
                            tokio::spawn(async move {
                                let res = tg.get_joined_channels().await.map_err(|e| e.to_string());
                                let _ = tx.send(AppEvent::ChannelsLoaded(res)).await;
                            });
                        }
                        crossterm::event::KeyCode::Char('2') => {
                            self.active_view = ActiveView::GroupSelect;
                            self.is_loading_groups = true;
                            self.available_groups.clear();
                            self.group_list_state.select(Some(0));
                            let tg = Arc::clone(telegram);
                            let tx = tx.clone();
                            tokio::spawn(async move {
                                let res = tg.get_joined_groups().await.map_err(|e| e.to_string());
                                let _ = tx.send(AppEvent::GroupsLoaded(res)).await;
                            });
                        }
                        crossterm::event::KeyCode::Char('3') => {
                            self.active_view = ActiveView::FilterConfig;
                            self.filter_config_state = FilterConfigState {
                                selected_field: FilterConfigField::PostCount,
                                post_count_threshold: self.state.post_count_threshold.to_string(),
                                editing: false,
                                error_message: None,
                            };
                        }
                        crossterm::event::KeyCode::Char('m') => {
                            self.active_view = ActiveView::Monitoring;
                            let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
                            self.monitoring_cancel_tx = Some(cancel_tx);
                            self.next_tick_at = Some(
                                std::time::Instant::now()
                                    + std::time::Duration::from_secs(
                                        self.state.poll_interval_secs.max(60),
                                    ),
                            );

                            crate::monitor::start_monitoring_loop(
                                self.state.clone(),
                                Arc::clone(telegram),
                                tx.clone(),
                                cancel_rx,
                            );
                        }
                        crossterm::event::KeyCode::Char('u') => {
                            if let Some(tx_cancel) = self.monitoring_cancel_tx.take() {
                                let _ = tx_cancel.send(true);
                            }
                            let tx_clone = tx.clone();
                            let _ = tx_clone.try_send(AppEvent::EnterUpload);
                        }
                        crossterm::event::KeyCode::Char('s') => {
                            let mut missing = Vec::new();
                            if self.state.channel_pairs[self.active_pair_index]
                                .source_channel_id
                                .is_none()
                            {
                                missing.push("Source Channel");
                            }
                            if self.state.channel_pairs[self.active_pair_index]
                                .dest_group_id
                                .is_none()
                            {
                                missing.push("Destination Group");
                            }
                            if self.state.channel_pairs[self.active_pair_index]
                                .dest_topic_id
                                .is_none()
                                && !self.state.auto_create_topic
                            {
                                missing.push("Destination Topic");
                            }

                            if !missing.is_empty() {
                                self.home_error =
                                    Some(format!("Missing configuration: {}", missing.join(", ")));
                                return;
                            }
                            self.home_error = None;

                            let tx = tx.clone();
                            let _ = tx.try_send(AppEvent::StartArchiveRun);
                        }
                        crossterm::event::KeyCode::Char('c')
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            self.should_quit = true;
                        }
                        _ => {}
                    },
                    ActiveView::ChannelSelect => match key.code {
                        crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                            if !self.available_channels.is_empty() {
                                let i = match self.channel_list_state.selected() {
                                    Some(i) => {
                                        if i >= self.available_channels.len() - 1 {
                                            i
                                        } else {
                                            i + 1
                                        }
                                    }
                                    None => 0,
                                };
                                self.channel_list_state.select(Some(i));
                            }
                        }
                        crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                            if !self.available_channels.is_empty() {
                                let i = match self.channel_list_state.selected() {
                                    Some(i) => {
                                        if i == 0 {
                                            0
                                        } else {
                                            i - 1
                                        }
                                    }
                                    None => 0,
                                };
                                self.channel_list_state.select(Some(i));
                            }
                        }
                        crossterm::event::KeyCode::Esc => {
                            self.home_error = None;
                            self.active_view = ActiveView::Home;
                        }
                        crossterm::event::KeyCode::Enter => {
                            if let Some(i) = self.channel_list_state.selected()
                                && let Some((id, title)) = self.available_channels.get(i)
                            {
                                self.state.channel_pairs[self.active_pair_index]
                                    .source_channel_id = Some(*id);
                                self.state.channel_pairs[self.active_pair_index]
                                    .source_channel_title = title.clone();
                                let state_clone = self.state.clone();
                                tokio::spawn(async move {
                                    let _ = state_clone.save().await;
                                });
                                self.active_view = ActiveView::GroupSelect;
                                self.is_loading_groups = true;
                                self.available_groups.clear();
                                self.group_list_state.select(Some(0));
                                let tg = Arc::clone(telegram);
                                let tx = tx.clone();
                                tokio::spawn(async move {
                                    let res =
                                        tg.get_joined_groups().await.map_err(|e| e.to_string());
                                    let _ = tx.send(AppEvent::GroupsLoaded(res)).await;
                                });
                            }
                        }
                        _ => {}
                    },
                    ActiveView::GroupSelect => match key.code {
                        crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                            if !self.available_groups.is_empty() {
                                let i = match self.group_list_state.selected() {
                                    Some(i) => {
                                        if i >= self.available_groups.len() - 1 {
                                            i
                                        } else {
                                            i + 1
                                        }
                                    }
                                    None => 0,
                                };
                                self.group_list_state.select(Some(i));
                            }
                        }
                        crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                            if !self.available_groups.is_empty() {
                                let i = match self.group_list_state.selected() {
                                    Some(i) => {
                                        if i == 0 {
                                            0
                                        } else {
                                            i - 1
                                        }
                                    }
                                    None => 0,
                                };
                                self.group_list_state.select(Some(i));
                            }
                        }
                        crossterm::event::KeyCode::Esc => {
                            self.home_error = None;
                            self.active_view = ActiveView::Home;
                        }
                        crossterm::event::KeyCode::Enter => {
                            if let Some(i) = self.group_list_state.selected()
                                && let Some((id, title)) = self.available_groups.get(i)
                            {
                                self.state.channel_pairs[self.active_pair_index].dest_group_id =
                                    Some(*id);
                                self.state.channel_pairs[self.active_pair_index].dest_group_title =
                                    title.clone();
                                let state_clone = self.state.clone();
                                tokio::spawn(async move {
                                    let _ = state_clone.save().await;
                                });
                                self.active_view = ActiveView::TopicSelect;
                                let tg = Arc::clone(telegram);
                                let tx = tx.clone();
                                let group_id = *id;
                                tokio::spawn(async move {
                                    let res_topics =
                                        tg.list_topics(group_id).await.map_err(|e| e.to_string());
                                    let _ = tx.send(AppEvent::TopicsLoaded(res_topics)).await;
                                });
                            }
                        }
                        _ => {}
                    },
                    ActiveView::TopicSelect => match key.code {
                        crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                            if !self.available_topics.is_empty() {
                                let i = match self.topic_list_state.selected() {
                                    Some(i) => {
                                        if i >= self.available_topics.len() - 1 {
                                            i
                                        } else {
                                            i + 1
                                        }
                                    }
                                    None => 0,
                                };
                                self.topic_list_state.select(Some(i));
                            }
                        }
                        crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                            if !self.available_topics.is_empty() {
                                let i = match self.topic_list_state.selected() {
                                    Some(i) => {
                                        if i == 0 {
                                            0
                                        } else {
                                            i - 1
                                        }
                                    }
                                    None => 0,
                                };
                                self.topic_list_state.select(Some(i));
                            }
                        }
                        crossterm::event::KeyCode::Esc => {
                            self.home_error = None;
                            self.active_view = ActiveView::Home;
                        }
                        crossterm::event::KeyCode::Enter => {
                            if let Some(i) = self.topic_list_state.selected() {
                                if i == 0 {
                                    // "Create new topic automatically"
                                    self.state.channel_pairs[self.active_pair_index]
                                        .dest_topic_id = None;
                                    self.state.channel_pairs[self.active_pair_index]
                                        .dest_topic_title = None;
                                    self.state.auto_create_topic = true;
                                } else if let Some((id, title)) = self.available_topics.get(i - 1) {
                                    self.state.channel_pairs[self.active_pair_index]
                                        .dest_topic_id = Some(*id);
                                    self.state.channel_pairs[self.active_pair_index]
                                        .dest_topic_title = Some(title.clone());
                                    self.state.auto_create_topic = false;
                                }

                                let state_clone = self.state.clone();
                                tokio::spawn(async move {
                                    let _ = state_clone.save().await;
                                });
                                self.home_error = None;
                                self.active_view = ActiveView::Home;
                            }
                        }
                        _ => {}
                    },
                    ActiveView::UploadModeSelect => match key.code {
                        crossterm::event::KeyCode::Char('s')
                        | crossterm::event::KeyCode::Char('S') => {
                            let _ = tx.try_send(AppEvent::UploadModeSelected(UploadMode::Select));
                        }
                        crossterm::event::KeyCode::Char('y')
                        | crossterm::event::KeyCode::Char('Y') => {
                            let _ = tx.try_send(AppEvent::UploadModeSelected(UploadMode::Sync));
                        }
                        crossterm::event::KeyCode::Esc => {
                            self.active_view = ActiveView::Home;
                        }
                        _ => {}
                    },
                    ActiveView::UploadSyncResume => match key.code {
                        crossterm::event::KeyCode::Char('y')
                        | crossterm::event::KeyCode::Char('Y')
                        | crossterm::event::KeyCode::Enter => {
                            self.upload_dest_group_id =
                                Some(self.upload_sync_state.as_ref().unwrap().dest_group_id);
                            self.upload_dest_topic_id =
                                self.upload_sync_state.as_ref().unwrap().dest_topic_id;
                            // Filter out already uploaded files from selection
                            if let Some(state) = &self.upload_sync_state {
                                #[allow(clippy::collapsible_if)]
                                for (i, entry) in self.upload_entries.iter().enumerate() {
                                    if let UploadEntry::File {
                                        name, size_bytes, ..
                                    } = entry
                                    {
                                        if !state.uploaded_files.iter().any(|f| {
                                            f.filename == *name && f.size_bytes >= *size_bytes
                                        }) {
                                            self.upload_selected[i] = true;
                                        }
                                    }
                                }
                            }
                            self.active_view = ActiveView::UploadFileSelect;
                        }
                        crossterm::event::KeyCode::Char('n')
                        | crossterm::event::KeyCode::Char('N') => {
                            self.upload_sync_state = None;
                            self.active_view = ActiveView::UploadFileSelect;
                        }
                        crossterm::event::KeyCode::Esc => {
                            self.active_view = ActiveView::Home;
                        }
                        _ => {}
                    },
                    ActiveView::UploadFileSelect => match key.code {
                        crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                            if !self.upload_entries.is_empty() {
                                let i = match self.upload_list_state.selected() {
                                    Some(i) => {
                                        if i >= self.upload_entries.len() - 1 {
                                            i
                                        } else {
                                            i + 1
                                        }
                                    }
                                    None => 0,
                                };
                                self.upload_list_state.select(Some(i));
                            }
                        }
                        crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                            if !self.upload_entries.is_empty() {
                                let i = match self.upload_list_state.selected() {
                                    Some(i) => {
                                        if i == 0 {
                                            0
                                        } else {
                                            i - 1
                                        }
                                    }
                                    None => 0,
                                };
                                self.upload_list_state.select(Some(i));
                            }
                        }
                        crossterm::event::KeyCode::Char(' ') => {
                            if let Some(i) = self.upload_list_state.selected() {
                                let _ = tx.try_send(AppEvent::UploadFileToggled(i));
                            }
                        }
                        crossterm::event::KeyCode::Char('a') => {
                            let _ = tx.try_send(AppEvent::UploadSelectAll);
                        }
                        crossterm::event::KeyCode::Char('t') => {
                            let _ = tx.try_send(AppEvent::UploadSortToggle);
                        }
                        crossterm::event::KeyCode::Enter => {
                            if self.upload_sync_state.is_some() {
                                let _ = tx.try_send(AppEvent::StartUploadRun);
                            } else {
                                self.active_view = ActiveView::UploadGroupSelect;
                                self.is_loading_groups = true;
                                self.available_groups.clear();
                                self.group_list_state.select(Some(0));
                                let tg = std::sync::Arc::clone(telegram);
                                let tx_clone = tx.clone();
                                tokio::spawn(async move {
                                    let res =
                                        tg.get_joined_groups().await.map_err(|e| e.to_string());
                                    let _ = tx_clone.send(AppEvent::GroupsLoaded(res)).await;
                                });
                            }
                        }
                        crossterm::event::KeyCode::Esc => {
                            self.active_view = ActiveView::UploadModeSelect;
                        }
                        _ => {}
                    },
                    ActiveView::UploadGroupSelect => match key.code {
                        crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                            if !self.available_groups.is_empty() {
                                let i = match self.group_list_state.selected() {
                                    Some(i) => {
                                        if i >= self.available_groups.len() - 1 {
                                            i
                                        } else {
                                            i + 1
                                        }
                                    }
                                    None => 0,
                                };
                                self.group_list_state.select(Some(i));
                            }
                        }
                        crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                            if !self.available_groups.is_empty() {
                                let i = match self.group_list_state.selected() {
                                    Some(i) => {
                                        if i == 0 {
                                            0
                                        } else {
                                            i - 1
                                        }
                                    }
                                    None => 0,
                                };
                                self.group_list_state.select(Some(i));
                            }
                        }
                        crossterm::event::KeyCode::Esc => {
                            self.active_view = ActiveView::UploadFileSelect;
                        }
                        crossterm::event::KeyCode::Enter => {
                            if let Some(i) = self.group_list_state.selected()
                                && let Some((id, title)) = self.available_groups.get(i)
                            {
                                self.upload_dest_group_id = Some(*id);
                                self.upload_dest_group_title = title.clone();
                                self.active_view = ActiveView::UploadTopicSelect;
                                let tg = std::sync::Arc::clone(telegram);
                                let tx_clone = tx.clone();
                                let group_id = *id;
                                tokio::spawn(async move {
                                    let res_topics =
                                        tg.list_topics(group_id).await.map_err(|e| e.to_string());
                                    let _ = tx_clone.send(AppEvent::TopicsLoaded(res_topics)).await;
                                });
                            }
                        }
                        _ => {}
                    },
                    ActiveView::UploadTopicSelect => match key.code {
                        crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                            if !self.available_topics.is_empty() {
                                let i = match self.topic_list_state.selected() {
                                    Some(i) => {
                                        if i >= self.available_topics.len() {
                                            i
                                        } else {
                                            i + 1
                                        }
                                    } // Note: topics length + 1 because of manual entry option
                                    None => 0,
                                };
                                self.topic_list_state.select(Some(i));
                            }
                        }
                        crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                            if !self.available_topics.is_empty() {
                                let i = match self.topic_list_state.selected() {
                                    Some(i) => {
                                        if i == 0 {
                                            0
                                        } else {
                                            i - 1
                                        }
                                    }
                                    None => 0,
                                };
                                self.topic_list_state.select(Some(i));
                            }
                        }
                        crossterm::event::KeyCode::Esc => {
                            self.active_view = ActiveView::UploadGroupSelect;
                        }
                        crossterm::event::KeyCode::Enter => {
                            if let Some(i) = self.topic_list_state.selected() {
                                if i == 0 {
                                    self.upload_topic_name_input.clear();
                                    self.active_view = ActiveView::UploadTopicNameEntry;
                                } else if let Some((id, title)) = self.available_topics.get(i - 1) {
                                    self.upload_dest_topic_id = Some(*id);
                                    self.upload_dest_topic_title = Some(title.clone());
                                    let _ = tx.try_send(AppEvent::StartUploadRun);
                                }
                            }
                        }
                        _ => {}
                    },
                    ActiveView::UploadTopicNameEntry => match key.code {
                        crossterm::event::KeyCode::Char(c) => {
                            self.resolution_error = None;
                            self.upload_topic_name_input.push(c);
                        }
                        crossterm::event::KeyCode::Backspace => {
                            self.resolution_error = None;
                            self.upload_topic_name_input.pop();
                        }
                        crossterm::event::KeyCode::Esc => {
                            self.active_view = ActiveView::UploadTopicSelect;
                        }
                        crossterm::event::KeyCode::Enter => {
                            if !self.upload_topic_name_input.is_empty() {
                                let tg = std::sync::Arc::clone(telegram);
                                let tx_clone = tx.clone();
                                let group_id = self.upload_dest_group_id.unwrap();
                                let title = self.upload_topic_name_input.clone();
                                tokio::spawn(async move {
                                    match tg.create_topic(group_id, &title).await {
                                        Ok(id) => {
                                            let _ = tx_clone
                                                .send(AppEvent::UploadTopicCreated(id, title))
                                                .await;
                                        }
                                        Err(e) => {
                                            // For now just error as topicsloaded
                                            let _ = tx_clone
                                                .send(AppEvent::TopicsLoaded(Err(e.to_string())))
                                                .await;
                                        }
                                    }
                                });
                            }
                        }
                        _ => {}
                    },
                    ActiveView::UploadProgress => match key.code {
                        crossterm::event::KeyCode::Char('p')
                        | crossterm::event::KeyCode::Char(' ') => {
                            if let Some(tx) = &self.upload_pause_tx {
                                let current =
                                    self.is_paused.load(std::sync::atomic::Ordering::SeqCst);
                                let new_val = !current;
                                self.is_paused
                                    .store(new_val, std::sync::atomic::Ordering::SeqCst);
                                let _ = tx.send_replace(new_val);
                            }
                        }
                        crossterm::event::KeyCode::Esc | crossterm::event::KeyCode::Char('q') => {
                            if let Some(tx) = self.upload_cancel_tx.take() {
                                let _ = tx.send(());
                            }
                            self.upload_pause_tx = None;
                            self.is_paused
                                .store(false, std::sync::atomic::Ordering::SeqCst);
                            self.active_view = ActiveView::Home;
                        }
                        crossterm::event::KeyCode::Char('c')
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            self.should_quit = true;
                        }
                        _ => {}
                    },
                    ActiveView::FilterConfig => {
                        let tx = tx.clone();
                        let st = &self.filter_config_state;
                        // Handle input based on editing mode
                        if st.editing {
                            match key.code {
                                crossterm::event::KeyCode::Char(c) => {
                                    let _ = tx.try_send(AppEvent::TypeFilterChar(c));
                                }
                                crossterm::event::KeyCode::Backspace => {
                                    let _ = tx.try_send(AppEvent::BackspaceFilterChar);
                                }
                                crossterm::event::KeyCode::Enter => {
                                    let _ = tx.try_send(AppEvent::EndEditField);
                                }
                                crossterm::event::KeyCode::Esc => {
                                    let _ = tx.try_send(AppEvent::CancelEditField);
                                }
                                _ => {}
                            }
                        } else {
                            match key.code {
                                crossterm::event::KeyCode::Down
                                | crossterm::event::KeyCode::Char('j') => {
                                    let _ = tx.try_send(AppEvent::FilterConfigNextField);
                                }
                                crossterm::event::KeyCode::Up
                                | crossterm::event::KeyCode::Char('k') => {
                                    let _ = tx.try_send(AppEvent::FilterConfigPrevField);
                                }
                                crossterm::event::KeyCode::Esc => {
                                    let _ = tx.try_send(AppEvent::ExitFilterConfig);
                                }
                                crossterm::event::KeyCode::Enter => match st.selected_field {
                                    FilterConfigField::PostCount => {
                                        let _ = tx.try_send(AppEvent::BeginEditField);
                                    }
                                    FilterConfigField::Save => {
                                        let _ = tx.try_send(AppEvent::SaveFilterConfig);
                                    }
                                },
                                _ => {}
                            }
                        }
                    }

                    ActiveView::ArchiveProgress => match key.code {
                        crossterm::event::KeyCode::Char('p')
                        | crossterm::event::KeyCode::Char(' ') => {
                            let tx = tx.clone();
                            let _ = tx.try_send(AppEvent::TogglePause);
                        }
                        crossterm::event::KeyCode::Char('c')
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            self.should_quit = true;
                        }
                        _ => {}
                    },
                    ActiveView::ResumePrompt => match key.code {
                        crossterm::event::KeyCode::Char('y')
                        | crossterm::event::KeyCode::Char('Y')
                        | crossterm::event::KeyCode::Enter => {
                            let tx = tx.clone();
                            let _ = tx.try_send(AppEvent::PromptResumeResult(true));
                        }
                        crossterm::event::KeyCode::Char('n')
                        | crossterm::event::KeyCode::Char('N') => {
                            let tx = tx.clone();
                            let _ = tx.try_send(AppEvent::PromptResumeResult(false));
                        }
                        crossterm::event::KeyCode::Char('c')
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            self.should_quit = true;
                        }
                        _ => {}
                    },
                    ActiveView::Monitoring => match key.code {
                        crossterm::event::KeyCode::Char('a') => {
                            if let Some(tx_cancel) = self.monitoring_cancel_tx.take() {
                                let _ = tx_cancel.send(true);
                            }
                            let new_pair = crate::state::ChannelPair::default();
                            self.state.channel_pairs.push(new_pair);
                            self.pair_statuses.push(PairStatus::default());
                            self.active_pair_index = self.state.channel_pairs.len() - 1;

                            self.active_view = ActiveView::ChannelSelect;
                            self.is_loading_channels = true;
                            self.available_channels.clear();
                            self.channel_list_state.select(Some(0));
                            let tg = Arc::clone(telegram);
                            let tx = tx.clone();
                            tokio::spawn(async move {
                                let res = tg.get_joined_channels().await.map_err(|e| e.to_string());
                                let _ = tx.send(AppEvent::ChannelsLoaded(res)).await;
                            });
                        }
                        crossterm::event::KeyCode::Char('d') => {
                            if self.state.channel_pairs.len() > 1 {
                                self.active_view = ActiveView::DeletePairPrompt;
                            }
                        }
                        crossterm::event::KeyCode::Char('s') => {
                            if let Some(tx_cancel) = self.monitoring_cancel_tx.take() {
                                let _ = tx_cancel.send(true);
                            }
                            let tx = tx.clone();
                            let _ = tx.try_send(AppEvent::StartArchiveRun);
                        }
                        crossterm::event::KeyCode::Char('i') => {
                            self.active_view = ActiveView::IntervalConfig;
                            self.interval_config_state = IntervalConfigState {
                                interval_secs: self.state.poll_interval_secs.to_string(),
                                error_message: None,
                            };
                        }
                        crossterm::event::KeyCode::Char('q') => {
                            if let Some(tx_cancel) = self.monitoring_cancel_tx.take() {
                                let _ = tx_cancel.send(true);
                            }
                            self.active_view = ActiveView::Home;
                        }
                        crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                            if self.active_pair_index < self.state.channel_pairs.len() - 1 {
                                self.active_pair_index += 1;
                            }
                        }
                        crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                            if self.active_pair_index > 0 {
                                self.active_pair_index -= 1;
                            }
                        }
                        _ => {}
                    },
                    ActiveView::DeletePairPrompt => match key.code {
                        crossterm::event::KeyCode::Char('y')
                        | crossterm::event::KeyCode::Char('Y')
                        | crossterm::event::KeyCode::Enter => {
                            if self.state.channel_pairs.len() > 1 {
                                self.state.channel_pairs.remove(self.active_pair_index);
                                self.pair_statuses.remove(self.active_pair_index);
                                if self.active_pair_index >= self.state.channel_pairs.len() {
                                    self.active_pair_index = self.state.channel_pairs.len() - 1;
                                }
                                let state_clone = self.state.clone();
                                tokio::spawn(async move {
                                    let _ = state_clone.save().await;
                                });
                            }
                            self.active_view = ActiveView::Monitoring;
                        }
                        crossterm::event::KeyCode::Char('n')
                        | crossterm::event::KeyCode::Char('N')
                        | crossterm::event::KeyCode::Esc => {
                            self.active_view = ActiveView::Monitoring;
                        }
                        _ => {}
                    },
                    ActiveView::IntervalConfig => match key.code {
                        crossterm::event::KeyCode::Char(c) if c.is_ascii_digit() => {
                            self.interval_config_state.error_message = None;
                            self.interval_config_state.interval_secs.push(c);
                        }
                        crossterm::event::KeyCode::Backspace => {
                            self.interval_config_state.error_message = None;
                            self.interval_config_state.interval_secs.pop();
                        }
                        crossterm::event::KeyCode::Enter => {
                            if let Ok(mut val) =
                                self.interval_config_state.interval_secs.parse::<u64>()
                            {
                                if val < 60 {
                                    val = 60;
                                }
                                self.state.poll_interval_secs = val;
                                let state_clone = self.state.clone();
                                tokio::spawn(async move {
                                    let _ = state_clone.save().await;
                                });
                                self.active_view = ActiveView::Monitoring;

                                // Restart monitoring loop to pick up new interval
                                if let Some(tx_cancel) = self.monitoring_cancel_tx.take() {
                                    let _ = tx_cancel.send(true);
                                }
                                let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
                                self.monitoring_cancel_tx = Some(cancel_tx);
                                self.next_tick_at = Some(
                                    std::time::Instant::now()
                                        + std::time::Duration::from_secs(
                                            self.state.poll_interval_secs.max(60),
                                        ),
                                );
                                crate::monitor::start_monitoring_loop(
                                    self.state.clone(),
                                    Arc::clone(telegram),
                                    tx.clone(),
                                    cancel_rx,
                                );
                            } else {
                                self.interval_config_state.error_message =
                                    Some("Must be a number".into());
                            }
                        }
                        crossterm::event::KeyCode::Esc => {
                            self.active_view = ActiveView::Monitoring;
                        }
                        _ => {}
                    },
                }
            }
            AppEvent::Tick => {}
            AppEvent::ChannelsLoaded(res) => match res {
                Ok(channels) => {
                    self.available_channels = channels;
                    self.channel_list_state.select(Some(0));
                    self.is_loading_channels = false;
                    self.resolution_error = None;
                }
                Err(err) => {
                    self.is_loading_channels = false;
                    self.resolution_error = Some(err);
                }
            },
            AppEvent::GroupsLoaded(res) => match res {
                Ok(groups) => {
                    self.available_groups = groups;
                    self.group_list_state.select(Some(0));
                    self.is_loading_groups = false;
                    self.resolution_error = None;
                }
                Err(err) => {
                    self.is_loading_groups = false;
                    self.resolution_error = Some(err);
                }
            },
            AppEvent::TopicsLoaded(res) => match res {
                Ok(topics) => {
                    self.available_topics = topics;
                    self.topic_list_state.select(Some(0));
                    self.resolution_error = None;
                }
                Err(err) => {
                    self.resolution_error = Some(err);
                }
            },
            AppEvent::FilterConfigNextField => {
                let st = &mut self.filter_config_state;
                st.error_message = None;
                st.selected_field = st.selected_field.next();
            }
            AppEvent::FilterConfigPrevField => {
                let st = &mut self.filter_config_state;
                st.error_message = None;
                st.selected_field = st.selected_field.prev();
            }

            AppEvent::BeginEditField => {
                let st = &mut self.filter_config_state;
                st.error_message = None;
                st.editing = true;
            }
            AppEvent::TypeFilterChar(c) => {
                let st = &mut self.filter_config_state;
                st.error_message = None;
                if st.selected_field == FilterConfigField::PostCount && c.is_ascii_digit() {
                    st.post_count_threshold.push(c);
                }
            }
            AppEvent::BackspaceFilterChar => {
                let st = &mut self.filter_config_state;
                st.error_message = None;
                if st.selected_field == FilterConfigField::PostCount {
                    st.post_count_threshold.pop();
                }
            }
            AppEvent::EndEditField | AppEvent::CancelEditField => {
                let st = &mut self.filter_config_state;
                st.editing = false;
            }
            AppEvent::ExitFilterConfig => {
                self.home_error = None;
                self.active_view = ActiveView::Home;
            }
            AppEvent::SaveFilterConfig => {
                let st = &mut self.filter_config_state;
                let count_res = st.post_count_threshold.parse::<u32>();

                if count_res.is_err() {
                    st.error_message = Some("Post Count must be a valid number.".to_string());
                    return;
                }

                self.state.post_count_threshold = count_res.unwrap();

                let state_clone = self.state.clone();
                tokio::spawn(async move {
                    let _ = state_clone.save().await;
                });
                self.home_error = None;
                self.active_view = ActiveView::Home;
            }
            AppEvent::StartArchiveRun => {
                self.active_view = ActiveView::ArchiveProgress;

                let mut state_clone = self.state.clone();
                let tg_clone = Arc::clone(telegram);
                let tx_clone = tx.clone();
                let paused_clone = Arc::clone(&self.is_paused);
                let active_idx = self.active_pair_index;

                if self.state.channel_pairs[self.active_pair_index]
                    .source_channel_id
                    .is_some()
                {
                    let source_id = self.state.channel_pairs[self.active_pair_index]
                        .source_channel_id
                        .unwrap();
                    let dest_id_opt =
                        self.state.channel_pairs[self.active_pair_index].dest_group_id;
                    tokio::spawn(async move {
                        let source_missing = tg_clone.get_input_peer(source_id).await.is_none();
                        let dest_missing = match dest_id_opt {
                            Some(id) => tg_clone.get_input_peer(id).await.is_none(),
                            None => false,
                        };

                        if source_missing || (dest_id_opt.is_some() && dest_missing) {
                            let _ = tg_clone.get_joined_channels().await;
                            let _ = tg_clone.get_joined_groups().await;
                        }

                        // Handle automatic topic creation
                        if state_clone.auto_create_topic
                            && state_clone.channel_pairs[active_idx]
                                .dest_group_id
                                .is_some()
                        {
                            let group_id =
                                state_clone.channel_pairs[active_idx].dest_group_id.unwrap();
                            let topic_title = state_clone.channel_pairs[active_idx]
                                .source_channel_title
                                .clone();
                            let topic_title_str = if topic_title.is_empty() {
                                "Archive"
                            } else {
                                &topic_title
                            };
                            match tg_clone.create_topic(group_id, topic_title_str).await {
                                Ok(new_topic_id) => {
                                    let _ = tx_clone
                                        .send(AppEvent::TopicCreated(
                                            new_topic_id,
                                            topic_title_str.to_string(),
                                        ))
                                        .await;

                                    state_clone.channel_pairs[active_idx].dest_topic_id =
                                        Some(new_topic_id);
                                    state_clone.channel_pairs[active_idx].dest_topic_title =
                                        Some(topic_title_str.to_string());
                                    state_clone.auto_create_topic = false;
                                    let s_clone = state_clone.clone();
                                    let _ = s_clone.save().await;
                                }
                                Err(e) => {
                                    let _ = tx_clone.try_send(AppEvent::ArchiveError(format!(
                                        "Failed to create topic: {}",
                                        e
                                    )));
                                    return;
                                }
                            }
                        }

                        crate::archive::start_archive_run(
                            state_clone,
                            active_idx,
                            tg_clone,
                            tx_clone,
                            paused_clone,
                        );
                    });
                }
            }
            AppEvent::SaveCursor(cursor) => {
                self.state.channel_pairs[self.active_pair_index].last_forwarded_message_id =
                    Some(cursor);
                let state_clone = self.state.clone();
                tokio::spawn(async move {
                    let _ = state_clone.save().await;
                });
            }
            AppEvent::ArchiveComplete => {
                let state_clone = self.state.clone();
                tokio::spawn(async move {
                    let _ = state_clone.save().await;
                });
            }
            AppEvent::ArchiveError(err) => {
                self.home_error = Some(err);
                self.active_view = ActiveView::Home;
                let state_clone = self.state.clone();
                tokio::spawn(async move {
                    let _ = state_clone.save().await;
                });
            }
            AppEvent::TogglePause => {
                // Toggle the atomic bool
                let current = self.is_paused.load(std::sync::atomic::Ordering::Relaxed);
                self.is_paused
                    .store(!current, std::sync::atomic::Ordering::Relaxed);

                // Immediately save state when paused or unpaused
                let state_clone = self.state.clone();
                tokio::spawn(async move {
                    let _ = state_clone.save().await;
                });
            }
            AppEvent::PromptResumeResult(resume) => {
                if resume {
                    self.active_view = ActiveView::ArchiveProgress;
                    let state_clone = self.state.clone();
                    let tg_clone = Arc::clone(telegram);
                    let tx_clone = tx.clone();
                    let paused_clone = Arc::clone(&self.is_paused);
                    let active_idx = self.active_pair_index;

                    if self.state.channel_pairs[self.active_pair_index]
                        .source_channel_id
                        .is_some()
                    {
                        let source_id = self.state.channel_pairs[self.active_pair_index]
                            .source_channel_id
                            .unwrap();
                        let dest_id_opt =
                            self.state.channel_pairs[self.active_pair_index].dest_group_id;
                        tokio::spawn(async move {
                            let source_missing = tg_clone.get_input_peer(source_id).await.is_none();
                            let dest_missing = match dest_id_opt {
                                Some(id) => tg_clone.get_input_peer(id).await.is_none(),
                                None => false,
                            };

                            if source_missing || (dest_id_opt.is_some() && dest_missing) {
                                let _ = tg_clone.get_joined_channels().await;
                                let _ = tg_clone.get_joined_groups().await;
                            }
                            crate::archive::start_archive_run(
                                state_clone,
                                active_idx,
                                tg_clone,
                                tx_clone,
                                paused_clone,
                            );
                        });
                    }
                } else {
                    self.state.channel_pairs[self.active_pair_index].last_forwarded_message_id =
                        None;
                    let state_clone = self.state.clone();
                    tokio::spawn(async move {
                        let _ = state_clone.save().await;
                    });
                    self.home_error = None;
                    self.active_view = ActiveView::Home;
                }
            }
            AppEvent::TopicCreated(topic_id, title) => {
                self.state.channel_pairs[self.active_pair_index].dest_topic_id = Some(topic_id);
                self.state.channel_pairs[self.active_pair_index].dest_topic_title = Some(title);
                self.state.auto_create_topic = false;
                let state_clone = self.state.clone();
                tokio::spawn(async move {
                    let _ = state_clone.save().await;
                });
            }
            AppEvent::ArchiveTotalCount(n) => {
                self.source_message_count = Some(n);
            }
            AppEvent::MonitoringTick => {
                self.next_tick_at = Some(
                    std::time::Instant::now()
                        + std::time::Duration::from_secs(self.state.poll_interval_secs.max(60)),
                );
            }
            AppEvent::PairSyncStarted { pair_index } => {
                if pair_index < self.pair_statuses.len() {
                    self.pair_statuses[pair_index] = PairStatus::Syncing;
                }
            }
            AppEvent::PairSynced {
                pair_index,
                last_forwarded_message_id,
            } => {
                if pair_index < self.state.channel_pairs.len() {
                    self.state.channel_pairs[pair_index].last_forwarded_message_id =
                        Some(last_forwarded_message_id);
                    self.pair_statuses[pair_index] = PairStatus::Idle;
                    let state_clone = self.state.clone();
                    tokio::spawn(async move {
                        let _ = state_clone.save().await;
                    });
                }
            }
            AppEvent::PairError { pair_index, error } => {
                if pair_index < self.pair_statuses.len() {
                    self.pair_statuses[pair_index] = PairStatus::Error(error);
                }
            }
            AppEvent::EnterUpload => {
                self.upload_cwd =
                    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                self.upload_entries.clear();

                if let Ok(entries) = std::fs::read_dir(&self.upload_cwd) {
                    for entry in entries.flatten() {
                        if let Ok(metadata) = entry.metadata() {
                            let name = entry.file_name().to_string_lossy().into_owned();
                            if name.starts_with('.') {
                                continue;
                            } // skip hidden

                            if metadata.is_dir() {
                                self.upload_entries.push(UploadEntry::Dir {
                                    name,
                                    path: entry.path(),
                                });
                            } else if metadata.is_file() {
                                self.upload_entries.push(UploadEntry::File {
                                    name,
                                    path: entry.path(),
                                    size_bytes: metadata.len(),
                                    modified: metadata.modified().unwrap_or(std::time::UNIX_EPOCH),
                                });
                            }
                        }
                    }
                }

                // Default sort: alphabetical
                self.upload_sort = UploadSort::Alphabetical;
                self.upload_entries.sort_by(|a, b| {
                    let name_a = match a {
                        UploadEntry::File { name, .. } | UploadEntry::Dir { name, .. } => name,
                    };
                    let name_b = match b {
                        UploadEntry::File { name, .. } | UploadEntry::Dir { name, .. } => name,
                    };
                    name_a.cmp(name_b)
                });

                self.upload_selected = vec![false; self.upload_entries.len()];
                self.upload_list_state
                    .select(if self.upload_entries.is_empty() {
                        None
                    } else {
                        Some(0)
                    });
                self.active_view = ActiveView::UploadModeSelect;
            }
            AppEvent::UploadFileToggled(idx) => {
                if idx < self.upload_selected.len() {
                    self.upload_selected[idx] = !self.upload_selected[idx];
                }
            }
            AppEvent::UploadSelectAll => {
                let all_selected = self.upload_selected.iter().all(|&x| x);
                for selected in &mut self.upload_selected {
                    *selected = !all_selected;
                }
            }
            AppEvent::UploadSortToggle => {
                self.upload_sort = match self.upload_sort {
                    UploadSort::Alphabetical => UploadSort::ByModDate,
                    UploadSort::ByModDate => UploadSort::Alphabetical,
                };
                let sort_mode = self.upload_sort.clone();
                // We need to sort entries and preserve selection. But for simplicity, we'll just clear selection on sort toggle or map it.
                // Rebuilding selected array is tricky, so we'll just clear it for safety, or keep a tuple. Let's keep a tuple.
                let mut combined: Vec<_> = self
                    .upload_entries
                    .clone()
                    .into_iter()
                    .zip(self.upload_selected.clone())
                    .collect();
                combined.sort_by(|(a, _), (b, _)| {
                    match sort_mode {
                        UploadSort::Alphabetical => {
                            let name_a = match a {
                                UploadEntry::File { name, .. } | UploadEntry::Dir { name, .. } => {
                                    name
                                }
                            };
                            let name_b = match b {
                                UploadEntry::File { name, .. } | UploadEntry::Dir { name, .. } => {
                                    name
                                }
                            };
                            name_a.cmp(name_b)
                        }
                        UploadSort::ByModDate => {
                            let mod_a = match a {
                                UploadEntry::File { modified, .. } => *modified,
                                UploadEntry::Dir { .. } => std::time::UNIX_EPOCH,
                            };
                            let mod_b = match b {
                                UploadEntry::File { modified, .. } => *modified,
                                UploadEntry::Dir { .. } => std::time::UNIX_EPOCH,
                            };
                            mod_b.cmp(&mod_a) // Newest first
                        }
                    }
                });
                for (i, (entry, selected)) in combined.into_iter().enumerate() {
                    self.upload_entries[i] = entry;
                    self.upload_selected[i] = selected;
                }
            }
            AppEvent::UploadModeSelected(mode) => {
                self.upload_mode = Some(mode.clone());
                match mode {
                    UploadMode::Select => {
                        self.active_view = ActiveView::UploadFileSelect;
                    }
                    UploadMode::Sync => {
                        let cwd = self.upload_cwd.clone();
                        let tx_clone = tx.clone();
                        tokio::spawn(async move {
                            if let Ok(Some(state)) =
                                crate::upload::UploadSyncState::load(&cwd).await
                            {
                                let _ = tx_clone.send(AppEvent::UploadSyncStateFound(state)).await;
                            } else {
                                // Transition to file select
                                let _ = tx_clone
                                    .send(AppEvent::UploadModeSelected(UploadMode::Select))
                                    .await; // just force UI transition
                            }
                        });
                    }
                }
            }
            AppEvent::UploadSyncStateFound(state) => {
                self.upload_sync_state = Some(state);
                self.active_view = ActiveView::UploadSyncResume;
            }
            AppEvent::StartUploadRun => {
                self.active_view = ActiveView::UploadProgress;
                self.upload_progress_current = 0;
                self.upload_progress_total = 0;
                self.upload_progress_current_file = String::new();
                self.upload_warnings.clear();
                self.is_paused
                    .store(false, std::sync::atomic::Ordering::SeqCst);

                let client = Arc::clone(telegram);
                let cwd = self.upload_cwd.clone();
                let entries = self.upload_entries.clone();
                let selected = self.upload_selected.clone();
                let mode = self.upload_mode.clone().unwrap_or(UploadMode::Select);
                let dest_group_id = self.upload_dest_group_id.unwrap_or(0);
                let dest_topic_id = self.upload_dest_topic_id;
                let tx_clone = tx.clone();
                let paused_rx_tx = tokio::sync::watch::channel(false);
                let cancel_rx_tx = tokio::sync::watch::channel(());

                self.upload_pause_tx = Some(paused_rx_tx.0);
                self.upload_cancel_tx = Some(cancel_rx_tx.0);

                tokio::spawn(async move {
                    if let Err(e) = crate::upload::run_upload_loop(
                        client,
                        cwd,
                        entries,
                        selected,
                        mode,
                        dest_group_id,
                        dest_topic_id,
                        tx_clone.clone(),
                        paused_rx_tx.1,
                        cancel_rx_tx.1,
                    )
                    .await
                    {
                        let _ = tx_clone.send(AppEvent::UploadError(e.to_string())).await;
                    }
                });
            }
            AppEvent::UploadFileComplete {
                filename,
                index,
                total,
            } => {
                self.upload_progress_current_file = filename;
                self.upload_progress_current = index;
                self.upload_progress_total = total;
            }
            AppEvent::UploadComplete => {
                self.upload_progress_current_file = "Done".to_string();
                if self.upload_progress_total > 0 {
                    self.upload_progress_current = self.upload_progress_total;
                }
                self.upload_pause_tx = None;
                self.upload_cancel_tx = None;
            }
            AppEvent::UploadError(msg) => {
                self.home_error = Some(format!("Upload error: {}", msg));
                self.upload_pause_tx = None;
                self.upload_cancel_tx = None;
            }
            AppEvent::UploadWarning(msg) => {
                self.upload_warnings.push(msg);
            }
            AppEvent::UploadTopicCreated(topic_id, title) => {
                self.upload_dest_topic_id = Some(topic_id);
                self.upload_dest_topic_title = Some(title);
                self.active_view = ActiveView::UploadProgress;
                let _ = tx.try_send(AppEvent::StartUploadRun);
            }
            AppEvent::TranscodeStarted { filename, .. } => {
                self.upload_is_transcoding = true;
                self.upload_transcode_filename = filename;
                self.upload_transcode_fps = 0.0;
                self.upload_transcode_speed = 0.0;
                self.upload_transcode_time_encoded = String::new();
                self.upload_transcode_percent = 0.0;
            }
            AppEvent::TranscodeProgress {
                filename,
                fps,
                speed,
                time_encoded,
                percent,
            } => {
                self.upload_transcode_filename = filename;
                self.upload_transcode_fps = fps;
                self.upload_transcode_speed = speed;
                self.upload_transcode_time_encoded = time_encoded;
                self.upload_transcode_percent = percent;
            }
            AppEvent::TranscodeComplete { .. } => {
                self.upload_is_transcoding = false;
                self.upload_transcode_filename = String::new();
                self.upload_transcode_fps = 0.0;
                self.upload_transcode_speed = 0.0;
                self.upload_transcode_time_encoded = String::new();
                self.upload_transcode_percent = 0.0;
            }
            AppEvent::TranscodeError { .. } => {
                self.upload_is_transcoding = false;
                self.upload_transcode_filename = String::new();
                self.upload_transcode_fps = 0.0;
                self.upload_transcode_speed = 0.0;
                self.upload_transcode_time_encoded = String::new();
                self.upload_transcode_percent = 0.0;
            }
        }
    }
}
