use crate::config::Config;
use crate::state::State;
use crossterm::event::KeyEvent;

pub enum AppEvent {
    Input(KeyEvent),
    Tick,
}

pub struct App {
    config: Config,
    state: State,
    should_quit: bool,
}

impl App {
    pub fn new(config: Config, state: State) -> Self {
        Self {
            config,
            state,
            should_quit: false,
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Input(key) => match key.code {
                crossterm::event::KeyCode::Char('q') => self.should_quit = true,
                crossterm::event::KeyCode::Char('c')
                    if key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL) =>
                {
                    self.should_quit = true;
                }
                _ => {}
            },
            AppEvent::Tick => {}
        }
    }
}
