use ratatui::{
    prelude::{Alignment, Color, Constraint, Direction, Frame, Layout, Rect, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::{
    app::{App, EmulatorState, ViewState},
    widgets::display::Display,
};

pub struct RendererBuilder {
    color_main_fg: Color,
    color_main_bg: Color,
    color_general_fg: Color,
    color_general_bg: Color,
    color_view_fg: Color,
    color_view_bg: Color,
    pixel_filled: String,
    pixel_empty: String,
}

impl RendererBuilder {
    pub fn new() -> Self {
        Self {
            color_main_fg: Color::Gray,
            color_main_bg: Color::DarkGray,
            color_general_fg: Color::Rgb(0x00, 0x66, 0x00),
            color_general_bg: Color::DarkGray,
            color_view_fg: Color::Yellow,
            color_view_bg: Color::Blue,
            pixel_filled: "█".to_string(),
            pixel_empty: " ".to_string(),
        }
    }

    pub fn build(&self) -> Renderer {
        Renderer {
            style_main: Style::default()
                .fg(self.color_main_fg)
                .bg(self.color_main_bg),
            style_general: Style::default()
                .fg(self.color_general_fg)
                .bg(self.color_general_bg),
            style_view: Style::default()
                .fg(self.color_view_fg)
                .bg(self.color_view_bg),
            pixel_filled: self.pixel_filled.clone(),
            pixel_empty: self.pixel_empty.clone(),
        }
    }
}

pub struct Renderer {
    style_main: Style,
    style_general: Style,
    style_view: Style,
    pixel_filled: String,
    pixel_empty: String,
}

impl Renderer {
    ///
    /// Renders the layout
    /// ------------------------
    /// |  general info
    /// ------------------------
    /// | specialized-view
    /// |
    /// |
    /// ------------------------
    ///
    pub fn render(&self, app: &mut App, frame: &mut Frame) {
        let layout = Layout::default()
            .margin(1) // Allows for the frame
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(3), Constraint::Fill(1)])
            .split(frame.size());

        // Frame around entire app
        frame.render_widget(
            Block::default()
                .title("chip-8")
                .title_alignment(Alignment::Left)
                .borders(Borders::ALL)
                .border_type(BorderType::Plain)
                .style(self.style_main),
            frame.size(),
        );
        // Top general info bar
        self.render_general_bar(app, frame, layout[0]);
        match app.view_state() {
            ViewState::GameView => {
                // f.render_widget(Display::new(64, 32, app.graphics_buffer), f.size())
                frame.render_widget(
                    Display::new(
                        app.graphics_buffer(),
                        self.pixel_filled.clone(),
                        self.pixel_empty.clone(),
                        self.style_view,
                    ),
                    layout[1],
                );
            }
            ViewState::DebugView => {
                frame.render_widget(
                    Paragraph::new("DebugView".to_string())
                        .style(Style::default())
                        .alignment(Alignment::Left),
                    layout[1],
                );
            }
        }
    }

    ///
    /// | Emulator-state | CPU Hz | FPS | File:
    ///
    fn render_general_bar(&self, app: &mut App, frame: &mut Frame, rect: Rect) {
        let emu_state = match app.emulator() {
            EmulatorState::Unloaded => "unloaded",
            EmulatorState::Running(_) => "running",
            EmulatorState::Paused(_) => "paused",
        };
        let (hz, file) = match app.emulator() {
            EmulatorState::Unloaded => ("N/A".to_string(), "N/A".to_string()),
            _ => {
                let f = if let Some(f) = app.file() {
                    f.clone()
                } else {
                    "N/A".to_string()
                };
                (format!("{}hz/{}", app.hertz(), app.timeboxes()), f)
            }
        };
        let msg = format!(
            "Emulator: {} | FPS: {} | CPU Hz: {} | File: {} | 'q' - quit, 'p' - play/pause",
            emu_state,
            app.fps(),
            hz,
            file,
        );
        frame.render_widget(
            Paragraph::new(msg)
                .style(self.style_general)
                .alignment(Alignment::Left)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded),
                ),
            rect,
        );
    }
}
