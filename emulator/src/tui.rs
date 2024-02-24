use crate::{app::App, event::EventHandler, ui::Renderer};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{io, panic};

pub type CrosstermTerminal = ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stderr>>;

/// Representation of terminal user interface
///
/// It is responsible for setting up the terminal
/// initializing the interface and handling the draw events
pub struct Tui {
    /// Interface to the terminal
    terminal: CrosstermTerminal,
    /// Terminal event handler
    pub events: EventHandler,
    renderer: Renderer,
}

impl Tui {
    pub fn new(terminal: CrosstermTerminal, events: EventHandler, renderer: Renderer) -> Self {
        Self {
            terminal,
            events,
            renderer,
        }
    }

    pub fn enter(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        terminal::enable_raw_mode()?;
        crossterm::execute!(io::stderr(), EnterAlternateScreen, EnableMouseCapture,)?;

        // Define custom panic hook to reset the terminal properties
        let panic_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic| {
            Self::reset().expect("failed to reset terminal");
            panic_hook(panic);
        }));

        Ok(())
    }

    fn reset() -> Result<(), Box<dyn std::error::Error>> {
        terminal::disable_raw_mode()?;
        crossterm::execute!(io::stderr(), LeaveAlternateScreen, DisableMouseCapture,)?;
        Ok(())
    }

    pub fn exit(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Self::reset()?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    pub fn draw(&mut self, app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
        self.terminal
            .draw(|frame| self.renderer.render(app, frame))?;
        Ok(())
    }
}
