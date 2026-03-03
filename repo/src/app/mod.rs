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
    ChannelStateLoaded(crate::state::State),
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
    ArchiveLog(String),
    ArchiveStarted { start_id: i32, highest_msg_id: i32 },
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
    ArchiveProgress,
    ResumePrompt,
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
pub struct ArchiveLogLine {
    pub timestamp: String,
    pub msg: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ArchiveProgressState {
    pub logs: Vec<ArchiveLogLine>,
    pub start_id: i32,
    pub highest_msg_id: i32,
    pub completed: bool,
    pub scroll_offset: usize,
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
    pub archive_progress_state: ArchiveProgressState,
    pub is_paused: Arc<std::sync::atomic::AtomicBool>,
    pub channel_loading: bool,
}

impl App {
    pub fn new(config: Config, state: State) -> Self {
        Self {
            config,
            state,
            should_quit: false,
            active_view: ActiveView::Home,
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
            archive_progress_state: ArchiveProgressState::default(),
            is_paused: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            channel_loading: false,
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
                        crossterm::event::KeyCode::Char('s') => {
                            let mut missing = Vec::new();
                            if self.state.source_channel_id.is_none() {
                                missing.push("Source Channel");
                            }
                            if self.state.dest_group_id.is_none() {
                                missing.push("Destination Group");
                            }
                            if self.state.dest_topic_id.is_none() && !self.state.auto_create_topic {
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
                            if self.channel_loading {
                                return;
                            }
                            if let Some(i) = self.channel_list_state.selected()
                                && let Some((id, title)) = self.available_channels.get(i)
                            {
                                self.channel_loading = true;
                                let new_id = *id;
                                let title_clone = title.clone();
                                let current_state = self.state.clone();
                                let tx = tx.clone();

                                tokio::spawn(async move {
                                    if current_state.source_channel_id == Some(new_id) {
                                        // No-op reload
                                        let _ = tx
                                            .send(AppEvent::ChannelStateLoaded(current_state))
                                            .await;
                                        return;
                                    }

                                    if current_state.source_channel_id.is_some() {
                                        let _ = current_state.save().await;
                                    }

                                    let mut new_state = match State::load_for_channel(new_id).await
                                    {
                                        Ok(s) => s,
                                        Err(e) => {
                                            let _ = tx
                                                .send(AppEvent::ArchiveError(format!(
                                                    "Failed to load state: {}",
                                                    e
                                                )))
                                                .await;
                                            return;
                                        }
                                    };
                                    new_state.source_channel_id = Some(new_id);
                                    new_state.source_channel_title = Some(title_clone);
                                    let _ = tx.send(AppEvent::ChannelStateLoaded(new_state)).await;
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
                            if let Some(i) = self.topic_list_state.selected() {
                                if i == 0 {
                                    // "Create new topic automatically"
                                    self.state.dest_topic_id = None;
                                    self.state.dest_topic_title = None;
                                    self.state.auto_create_topic = true;
                                } else if let Some((id, title)) = self.available_topics.get(i - 1) {
                                    self.state.dest_topic_id = Some(*id);
                                    self.state.dest_topic_title = Some(title.clone());
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
                            if !self.archive_progress_state.completed {
                                let tx = tx.clone();
                                let _ = tx.try_send(AppEvent::TogglePause);
                            }
                        }
                        crossterm::event::KeyCode::Char('q') => {
                            if self.archive_progress_state.completed {
                                self.home_error = None;
                                self.active_view = ActiveView::Home;
                            }
                        }
                        crossterm::event::KeyCode::Char('r') => {
                            if self.archive_progress_state.completed {
                                let tx = tx.clone();
                                let _ = tx.try_send(AppEvent::StartArchiveRun);
                            }
                        }
                        crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                            if self.archive_progress_state.scroll_offset > 0 {
                                self.archive_progress_state.scroll_offset -= 1;
                            }
                        }
                        crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                            self.archive_progress_state.scroll_offset += 1;
                        }
                        crossterm::event::KeyCode::PageUp => {
                            self.archive_progress_state.scroll_offset =
                                self.archive_progress_state.scroll_offset.saturating_sub(10);
                        }
                        crossterm::event::KeyCode::PageDown => {
                            self.archive_progress_state.scroll_offset += 10;
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
            AppEvent::ChannelStateLoaded(new_state) => {
                self.channel_loading = false;
                self.state = new_state;

                if let Some(channel_id) = self.state.source_channel_id {
                    let session = crate::state::LastSession {
                        last_channel_id: Some(channel_id),
                    };
                    tokio::spawn(async move {
                        let _ = session.save().await;
                    });
                }

                let has_partial_state = self.state.last_forwarded_message_id.is_some()
                    && self.state.source_message_count.is_some();

                if has_partial_state {
                    self.active_view = ActiveView::ResumePrompt;
                } else {
                    self.active_view = ActiveView::GroupSelect;
                    self.is_loading_groups = true;
                    self.available_groups.clear();
                    self.group_list_state.select(Some(0));

                    let tg = Arc::clone(telegram);
                    let tx_clone = tx.clone();
                    tokio::spawn(async move {
                        let res = tg.get_joined_groups().await.map_err(|e| e.to_string());
                        let _ = tx_clone.send(AppEvent::GroupsLoaded(res)).await;
                    });
                }
            }
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
                self.archive_progress_state = ArchiveProgressState::default();

                let mut state_clone = self.state.clone();
                let tg_clone = Arc::clone(telegram);
                let tx_clone = tx.clone();
                let paused_clone = Arc::clone(&self.is_paused);

                if let Some(source_id) = self.state.source_channel_id {
                    let dest_id = self.state.dest_group_id;
                    tokio::spawn(async move {
                        let source_missing = tg_clone.get_input_peer(source_id).await.is_none();
                        let dest_missing = match dest_id {
                            Some(id) => tg_clone.get_input_peer(id).await.is_none(),
                            None => false,
                        };

                        if source_missing || (dest_id.is_some() && dest_missing) {
                            let _ = tg_clone.get_joined_channels().await;
                            let _ = tg_clone.get_joined_groups().await;
                        }

                        // Handle automatic topic creation
                        if state_clone.auto_create_topic
                            && let Some(group_id) = state_clone.dest_group_id
                        {
                            let topic_title = state_clone
                                .source_channel_title
                                .as_deref()
                                .unwrap_or("Archive");
                            match tg_clone.create_topic(group_id, topic_title).await {
                                Ok(new_topic_id) => {
                                    state_clone.dest_topic_id = Some(new_topic_id);
                                    state_clone.dest_topic_title = Some(topic_title.to_string());
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
                            tg_clone,
                            tx_clone,
                            paused_clone,
                        );
                    });
                }
            }
            AppEvent::ArchiveStarted {
                start_id,
                highest_msg_id,
            } => {
                self.archive_progress_state.start_id = start_id;
                self.archive_progress_state.highest_msg_id = highest_msg_id;
            }
            AppEvent::ArchiveLog(msg) => {
                let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
                self.archive_progress_state
                    .logs
                    .push(ArchiveLogLine { timestamp, msg });

                // Auto-scroll logic: if user is not scrolled up, keep them at the bottom.
                // We'll reset scroll_offset to max length so the view ensures it sticks to the bottom.
                self.archive_progress_state.scroll_offset = usize::MAX;
            }
            AppEvent::SaveCursor(cursor) => {
                self.state.last_forwarded_message_id = Some(cursor);
                let state_clone = self.state.clone();
                tokio::spawn(async move {
                    let _ = state_clone.save().await;
                });
            }
            AppEvent::ArchiveComplete => {
                self.archive_progress_state.completed = true;
                let state_clone = self.state.clone();
                tokio::spawn(async move {
                    let _ = state_clone.save().await;
                });
            }
            AppEvent::ArchiveError(err) => {
                self.channel_loading = false;
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
                    self.archive_progress_state = ArchiveProgressState::default();
                    let state_clone = self.state.clone();
                    let tg_clone = Arc::clone(telegram);
                    let tx_clone = tx.clone();
                    let paused_clone = Arc::clone(&self.is_paused);

                    if let Some(source_id) = self.state.source_channel_id {
                        let dest_id = self.state.dest_group_id;
                        tokio::spawn(async move {
                            let source_missing = tg_clone.get_input_peer(source_id).await.is_none();
                            let dest_missing = match dest_id {
                                Some(id) => tg_clone.get_input_peer(id).await.is_none(),
                                None => false,
                            };

                            if source_missing || (dest_id.is_some() && dest_missing) {
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
                    self.state.last_forwarded_message_id = None;
                    self.state.source_message_count = None;
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
