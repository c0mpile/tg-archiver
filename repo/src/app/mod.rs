use crate::config::Config;
use crate::state::State;
use crossterm::event::KeyEvent;
use std::sync::Arc;
use tokio::sync::mpsc;

pub enum AppEvent {
    Input(KeyEvent),
    Tick,
    ChannelResolved(Result<(i64, String), String>),
    GroupResolved(Result<(i64, String), String>),
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
    pub input_buffer: String,
    pub resolution_error: Option<String>,
    pub available_topics: Vec<(i32, String)>,
    pub selected_topic_index: usize,
    pub filter_config_state: FilterConfigState,
}

impl App {
    pub fn new(config: Config, state: State) -> Self {
        Self {
            config,
            state,
            should_quit: false,
            active_view: ActiveView::Home,
            input_buffer: String::new(),
            resolution_error: None,
            available_topics: Vec::new(),
            selected_topic_index: 0,
            filter_config_state: FilterConfigState::default(),
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
                            self.active_view = ActiveView::ChannelSelect
                        }
                        crossterm::event::KeyCode::Char('2') => {
                            self.active_view = ActiveView::GroupSelect
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
                        crossterm::event::KeyCode::Char(c) => self.input_buffer.push(c),
                        crossterm::event::KeyCode::Backspace => {
                            self.input_buffer.pop();
                        }
                        crossterm::event::KeyCode::Esc => {
                            self.active_view = ActiveView::Home;
                            self.input_buffer.clear();
                            self.resolution_error = None;
                        }
                        crossterm::event::KeyCode::Enter => {
                            if !self.input_buffer.is_empty() {
                                let username = self.input_buffer.clone();
                                let tg = Arc::clone(telegram);
                                let tx = tx.clone();

                                tokio::spawn(async move {
                                    let res = tg
                                        .resolve_channel(&username)
                                        .await
                                        .map_err(|e| e.to_string());
                                    let _ = tx.send(AppEvent::ChannelResolved(res)).await;
                                });
                            }
                        }
                        _ => {}
                    },
                    ActiveView::GroupSelect => {
                        match key.code {
                            crossterm::event::KeyCode::Char(c) => self.input_buffer.push(c),
                            crossterm::event::KeyCode::Backspace => {
                                self.input_buffer.pop();
                            }
                            crossterm::event::KeyCode::Esc => {
                                self.active_view = ActiveView::Home;
                                self.input_buffer.clear();
                                self.resolution_error = None;
                            }
                            crossterm::event::KeyCode::Enter => {
                                if !self.input_buffer.is_empty() {
                                    let username = self.input_buffer.clone();
                                    let tg = Arc::clone(telegram);
                                    let tx = tx.clone();

                                    tokio::spawn(async move {
                                        let res_group = tg
                                            .resolve_group(&username)
                                            .await
                                            .map_err(|e| e.to_string());
                                        match &res_group {
                                            Ok((id, _title)) => {
                                                // After resolving group, we automatically load topics
                                                let res_topics = tg
                                                    .list_topics(*id)
                                                    .await
                                                    .map_err(|e| e.to_string());
                                                let _ = tx
                                                    .send(AppEvent::GroupResolved(res_group))
                                                    .await;
                                                let _ = tx
                                                    .send(AppEvent::TopicsLoaded(res_topics))
                                                    .await;
                                            }
                                            Err(_) => {
                                                let _ = tx
                                                    .send(AppEvent::GroupResolved(res_group))
                                                    .await;
                                            }
                                        }
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                    ActiveView::TopicSelect => match key.code {
                        crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                            if !self.available_topics.is_empty()
                                && self.selected_topic_index < self.available_topics.len() - 1
                            {
                                self.selected_topic_index += 1;
                            }
                        }
                        crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                            if self.selected_topic_index > 0 {
                                self.selected_topic_index -= 1;
                            }
                        }
                        crossterm::event::KeyCode::Esc => {
                            self.active_view = ActiveView::Home;
                        }
                        crossterm::event::KeyCode::Enter => {
                            if let Some((id, title)) =
                                self.available_topics.get(self.selected_topic_index)
                            {
                                self.state.dest_topic_id = Some(*id);
                                self.state.dest_topic_title = Some(title.clone());
                                let state_clone = self.state.clone();
                                tokio::spawn(async move {
                                    let _ = state_clone.save().await;
                                });
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
                    ActiveView::ArchiveProgress => {
                        // TODO: handle user input for archive progress
                    }
                }
            }
            AppEvent::Tick => {}
            AppEvent::ChannelResolved(res) => match res {
                Ok((id, title)) => {
                    self.state.source_channel_id = Some(id);
                    self.state.source_channel_title = Some(title);
                    self.active_view = ActiveView::GroupSelect;
                    self.input_buffer.clear();
                    self.resolution_error = None;
                    let state_clone = self.state.clone();
                    tokio::spawn(async move {
                        let _ = state_clone.save().await;
                    });
                }
                Err(err) => {
                    self.resolution_error = Some(err);
                }
            },
            AppEvent::GroupResolved(res) => match res {
                Ok((id, title)) => {
                    self.state.dest_group_id = Some(id);
                    self.state.dest_group_title = Some(title);
                    self.active_view = ActiveView::TopicSelect;
                    self.input_buffer.clear();
                    self.resolution_error = None;
                    let state_clone = self.state.clone();
                    tokio::spawn(async move {
                        let _ = state_clone.save().await;
                    });
                }
                Err(err) => {
                    self.resolution_error = Some(err);
                }
            },
            AppEvent::TopicsLoaded(res) => match res {
                Ok(topics) => {
                    self.available_topics = topics;
                    self.selected_topic_index = 0;
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
                self.active_view = ActiveView::Home;
            }
            AppEvent::StartArchiveRun => {
                self.active_view = ActiveView::ArchiveProgress;
                crate::archive::start_archive_run(
                    self.state.clone(),
                    Arc::clone(telegram),
                    tx.clone(),
                );
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
                self.resolution_error = Some(err);
                let state_clone = self.state.clone();
                tokio::spawn(async move {
                    let _ = state_clone.save().await;
                });
            }
        }
    }
}
