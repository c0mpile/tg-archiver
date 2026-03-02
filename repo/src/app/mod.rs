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
    ToggleFilter(FilterConfigField),
    BeginEditField,
    TypeFilterChar(char),
    BackspaceFilterChar,
    EndEditField,
    CancelEditField,
    ExitFilterConfig,
    SaveFilterConfig,
    StartArchiveRun,
    DownloadProgress {
        msg_id: i32,
        status: crate::state::DownloadStatus,
    },
    ArchiveComplete,
    ArchiveError(String),
    SaveCursor(i32),
    TogglePause,
    PromptResumeResult(bool),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum ActiveView {
    #[default]
    Home,
    ChannelSelect,
    GroupSelect,
    TopicSelect,
    FilterConfig,
    ConfirmDownloadPath,
    ArchiveProgress,
    ResumePrompt,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum FilterConfigField {
    #[default]
    Video,
    Audio,
    Image,
    Archive,
    IncludeText,
    MinSize,
    PostCount,
    DownloadPath,
    Save, // Button to confirm and exit
}

impl FilterConfigField {
    pub fn next(&self) -> Self {
        match self {
            Self::Video => Self::Audio,
            Self::Audio => Self::Image,
            Self::Image => Self::Archive,
            Self::Archive => Self::IncludeText,
            Self::IncludeText => Self::MinSize,
            Self::MinSize => Self::PostCount,
            Self::PostCount => Self::DownloadPath,
            Self::DownloadPath => Self::Save,
            Self::Save => Self::Save,
        }
    }
    pub fn prev(&self) -> Self {
        match self {
            Self::Video => Self::Video,
            Self::Audio => Self::Video,
            Self::Image => Self::Audio,
            Self::Archive => Self::Image,
            Self::IncludeText => Self::Archive,
            Self::MinSize => Self::IncludeText,
            Self::PostCount => Self::MinSize,
            Self::DownloadPath => Self::PostCount,
            Self::Save => Self::DownloadPath,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FilterConfigState {
    pub selected_field: FilterConfigField,
    pub filter_video: bool,
    pub filter_audio: bool,
    pub filter_image: bool,
    pub filter_archive: bool,
    pub include_text_descriptions: bool,
    pub min_size_mb: String,
    pub post_count_threshold: String,
    pub local_download_path: String,
    pub editing: bool,
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
    pub is_paused: Arc<std::sync::atomic::AtomicBool>,
}

impl App {
    pub fn new(config: Config, state: State) -> Self {
        let has_partial_state = state.message_cursor.is_some()
            || state.download_status.values().any(|s| {
                matches!(
                    s,
                    crate::state::DownloadStatus::Pending
                        | crate::state::DownloadStatus::InProgress { .. }
                )
            });
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
            is_paused: Arc::new(std::sync::atomic::AtomicBool::new(false)),
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
                                selected_field: FilterConfigField::Video,
                                filter_video: self.state.filters.filter_video,
                                filter_audio: self.state.filters.filter_audio,
                                filter_image: self.state.filters.filter_image,
                                filter_archive: self.state.filters.filter_archive,
                                include_text_descriptions: self
                                    .state
                                    .filters
                                    .include_text_descriptions,
                                min_size_mb: (self.state.filters.min_size_bytes / 1024 / 1024)
                                    .to_string(),
                                post_count_threshold: self
                                    .state
                                    .filters
                                    .post_count_threshold
                                    .to_string(),
                                local_download_path: self.state.local_download_path.clone(),
                                editing: false,
                                error_message: None,
                            };
                        }
                        crossterm::event::KeyCode::Char('s') => {
                            let mut missing = Vec::new();
                            if self.state.source_channel_id.is_none() {
                                missing.push("Source Channel");
                            }
                            if self.state.dest_group_id.is_none() {
                                missing.push("Destination Group");
                            }
                            if self.state.dest_topic_id.is_none() {
                                missing.push("Destination Topic");
                            }

                            if !missing.is_empty() {
                                self.home_error =
                                    Some(format!("Missing configuration: {}", missing.join(", ")));
                                return;
                            }
                            self.home_error = None;

                            if self.state.local_download_path == "/tmp"
                                || self.state.local_download_path.starts_with("/tmp/")
                            {
                                self.active_view = ActiveView::ConfirmDownloadPath;
                            } else {
                                let tx = tx.clone();
                                let _ = tx.try_send(AppEvent::StartArchiveRun);
                            }
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
                                self.state.source_channel_id = Some(*id);
                                self.state.source_channel_title = Some(title.clone());
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
                                self.state.dest_group_id = Some(*id);
                                self.state.dest_group_title = Some(title.clone());
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
                            if let Some(i) = self.topic_list_state.selected()
                                && let Some((id, title)) = self.available_topics.get(i)
                            {
                                self.state.dest_topic_id = Some(*id);
                                self.state.dest_topic_title = Some(title.clone());
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
                                    FilterConfigField::Video
                                    | FilterConfigField::Audio
                                    | FilterConfigField::Image
                                    | FilterConfigField::Archive
                                    | FilterConfigField::IncludeText => {
                                        let _ = tx.try_send(AppEvent::ToggleFilter(
                                            st.selected_field.clone(),
                                        ));
                                    }
                                    FilterConfigField::MinSize
                                    | FilterConfigField::PostCount
                                    | FilterConfigField::DownloadPath => {
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
                    ActiveView::ConfirmDownloadPath => match key.code {
                        crossterm::event::KeyCode::Char('y')
                        | crossterm::event::KeyCode::Char('Y')
                        | crossterm::event::KeyCode::Enter => {
                            let tx = tx.clone();
                            let _ = tx.try_send(AppEvent::StartArchiveRun);
                        }
                        crossterm::event::KeyCode::Char('n')
                        | crossterm::event::KeyCode::Char('N')
                        | crossterm::event::KeyCode::Esc => {
                            self.home_error = None;
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
            AppEvent::ToggleFilter(field) => {
                let st = &mut self.filter_config_state;
                st.error_message = None;
                match field {
                    FilterConfigField::Video => st.filter_video = !st.filter_video,
                    FilterConfigField::Audio => st.filter_audio = !st.filter_audio,
                    FilterConfigField::Image => st.filter_image = !st.filter_image,
                    FilterConfigField::Archive => st.filter_archive = !st.filter_archive,
                    FilterConfigField::IncludeText => {
                        st.include_text_descriptions = !st.include_text_descriptions
                    }
                    _ => {}
                }
            }
            AppEvent::BeginEditField => {
                let st = &mut self.filter_config_state;
                st.error_message = None;
                st.editing = true;
            }
            AppEvent::TypeFilterChar(c) => {
                let st = &mut self.filter_config_state;
                st.error_message = None;
                match st.selected_field {
                    FilterConfigField::MinSize => {
                        if c.is_ascii_digit() {
                            st.min_size_mb.push(c);
                        }
                    }
                    FilterConfigField::PostCount => {
                        if c.is_ascii_digit() {
                            st.post_count_threshold.push(c);
                        }
                    }
                    FilterConfigField::DownloadPath => {
                        st.local_download_path.push(c);
                    }
                    _ => {}
                }
            }
            AppEvent::BackspaceFilterChar => {
                let st = &mut self.filter_config_state;
                st.error_message = None;
                match st.selected_field {
                    FilterConfigField::MinSize => {
                        st.min_size_mb.pop();
                    }
                    FilterConfigField::PostCount => {
                        st.post_count_threshold.pop();
                    }
                    FilterConfigField::DownloadPath => {
                        st.local_download_path.pop();
                    }
                    _ => {}
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
                let mb_res = st.min_size_mb.parse::<u64>();
                let count_res = st.post_count_threshold.parse::<u32>();

                if mb_res.is_err() {
                    st.error_message = Some("Min Size must be a valid number.".to_string());
                    return;
                }

                if count_res.is_err() {
                    st.error_message = Some("Post Count must be a valid number.".to_string());
                    return;
                }

                self.state.filters.filter_video = st.filter_video;
                self.state.filters.filter_audio = st.filter_audio;
                self.state.filters.filter_image = st.filter_image;
                self.state.filters.filter_archive = st.filter_archive;
                self.state.filters.include_text_descriptions = st.include_text_descriptions;

                self.state.filters.min_size_bytes = mb_res.unwrap() * 1024 * 1024;
                self.state.filters.post_count_threshold = count_res.unwrap();
                self.state.local_download_path = st.local_download_path.clone();

                let state_clone = self.state.clone();
                tokio::spawn(async move {
                    let _ = state_clone.save().await;
                });
                self.home_error = None;
                self.active_view = ActiveView::Home;
            }
            AppEvent::StartArchiveRun => {
                self.active_view = ActiveView::ArchiveProgress;

                let state_clone = self.state.clone();
                let tg_clone = Arc::clone(telegram);
                let tx_clone = tx.clone();
                let paused_clone = Arc::clone(&self.is_paused);

                if let Some(source_id) = self.state.source_channel_id {
                    tokio::spawn(async move {
                        if tg_clone.get_input_peer(source_id).await.is_none() {
                            let _ = tg_clone.get_joined_channels().await;
                            let _ = tg_clone.get_joined_groups().await;
                        }
                        crate::archive::start_archive_run(
                            state_clone,
                            tg_clone,
                            tx_clone,
                            paused_clone,
                        );
                    });
                }
            }
            AppEvent::DownloadProgress { msg_id, status } => {
                self.state.download_status.insert(msg_id, status);
                let state_clone = self.state.clone();
                tokio::spawn(async move {
                    let _ = state_clone.save().await;
                });
            }
            AppEvent::SaveCursor(cursor) => {
                self.state.message_cursor = Some(cursor);
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

                    if let Some(source_id) = self.state.source_channel_id {
                        tokio::spawn(async move {
                            if tg_clone.get_input_peer(source_id).await.is_none() {
                                let _ = tg_clone.get_joined_channels().await;
                                let _ = tg_clone.get_joined_groups().await;
                            }
                            crate::archive::start_archive_run(
                                state_clone,
                                tg_clone,
                                tx_clone,
                                paused_clone,
                            );
                        });
                    }
                } else {
                    self.state.message_cursor = None;
                    self.state.download_status.clear();
                    let state_clone = self.state.clone();
                    tokio::spawn(async move {
                        let _ = state_clone.save().await;
                    });
                    self.home_error = None;
                    self.active_view = ActiveView::Home;
                }
            }
        }
    }
}
