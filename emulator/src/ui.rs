use ratatui::prelude::Frame;

use crate::widgets::display::Display;

use crate::app::App;

pub fn render(app: &mut App, f: &mut Frame) {
    f.render_widget(Display::new(64, 32, app.get_graphics_buffer()), f.size())
}
