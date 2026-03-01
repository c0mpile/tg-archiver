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
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum ActiveView {
    #[default]
    Home,
    ChannelSelect,
    GroupSelect,
    TopicSelect,
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
        }
    }

    #[allow(dead_code)]
    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    #[allow(dead_code)]
    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
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
        }
    }
}
