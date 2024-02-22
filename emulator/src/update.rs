use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use chip8::emulator::Key;
use tracing::info;

pub fn update_keypress(app: &mut App, key_event: KeyEvent) {
    info!(code = ?key_event.code, "received key press event");

    match key_event.code {
        KeyCode::Esc | KeyCode::Char('q') => app.quit(),
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if key_event.modifiers == KeyModifiers::CONTROL {
                app.quit()
            }
        }
        KeyCode::Char('1') => app.set_key(1.into(), Key::Pressed),
        _ => {}
    }
}

pub fn update_keyup(app: &mut App, key_event: KeyEvent) {
    info!(code = ?key_event.code, "received key up event");

    match key_event.code {
        KeyCode::Char('1') => app.set_key(1.into(), Key::Up),
        _ => {}
    }
}
