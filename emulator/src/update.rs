use crossterm::event::{KeyCode, KeyEvent};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use chip8::{emulator::KeyStatus, instructions::u4};
use tracing::info;

use crate::app::{App, EmulatorState, ViewState};

// Used to deal with artifical key releases
pub struct KeyHandler {
    // store keybinds and when they where last pressed
    keys: HashMap<char, (u4, Option<Instant>)>,
    delay: Duration,
}

impl KeyHandler {
    pub fn new(delay: Duration) -> Self {
        Self {
            keys: Default::default(),
            delay,
        }
    }

    pub fn bind(&mut self, key: char, value: u8) {
        self.keys.insert(key, (value.into(), None));
    }

    pub fn handle_key(&mut self, app: &mut App, key_event: KeyEvent) {
        info!(code = ?key_event.code, "received key press event");
        match key_event.code {
            KeyCode::Esc | KeyCode::Char('q') => app.quit(),
            KeyCode::Char('p') => {
                if app.is_running() {
                    app.pause();
                } else {
                    app.start();
                }
            }
            KeyCode::Char('s') => {
                app.emulator_step();
            }
            KeyCode::F(2) => {
                app.set_view_state(ViewState::GameView);
            }
            KeyCode::F(3) => {
                app.set_view_state(ViewState::DebugView);
            }
            KeyCode::Char(c) => {
                if self.keys.contains_key(&c) {
                    let value = self.keys[&c].0;
                    info!(key=?c, value=value.value(), "registered bound key press");
                    if let Ok(_) = app.set_key(value, KeyStatus::Pressed) {
                        self.keys.insert(c, (value, Some(Instant::now())));
                    }
                }
            }
            _ => {}
        }
    }

    /// We cannot detect key release events in the terminal, so this
    /// tick function artificially mimics releasing the key every `delay`
    pub fn tick(&mut self, app: &mut App) {
        if !matches!(app.emulator(), EmulatorState::Running(_)) {
            return;
        }
        for (key, value) in self.keys.iter_mut() {
            if let Some(last) = value.1.take() {
                if last.elapsed() > self.delay {
                    info!(key=?*key, value=value.0.value(), "released key");
                    if let Err(_) = app.set_key(value.0, KeyStatus::Up) {
                        value.1 = Some(last)
                    }
                } else {
                    value.1 = Some(last);
                }
            }
        }
    }
}
