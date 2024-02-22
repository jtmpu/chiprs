use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use tracing::info;

pub fn update(app: &mut App, key_event: KeyEvent) {
    info!(code = ?key_event.code, "received key event");

    match key_event.code {
        KeyCode::Esc | KeyCode::Char('q') => app.quit(),
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if key_event.modifiers == KeyModifiers::CONTROL {
                app.quit()
            }
        }
        _ => {}
    }
}
