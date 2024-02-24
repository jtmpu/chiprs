use std::{
    sync::mpsc::{channel, Sender},
    thread::JoinHandle,
};

use tracing::{debug, error, info};

use chip8::{
    emulator::{self, Emulator, KeyStatus, Message, GRAPHICS_BUFFER_SIZE},
    instructions::u4,
};

pub struct App {
    fps: usize,
    hertz: usize,
    timeboxes: usize,
    file: Option<String>,
    should_quit: bool,
    view_state: ViewState,
    emulator_state: EmulatorState,
    graphics_buffer: [u8; GRAPHICS_BUFFER_SIZE],
}

impl App {
    pub fn new(fps: usize, hertz: usize, timeboxes: usize) -> Self {
        Self {
            should_quit: false,
            fps,
            hertz,
            timeboxes,
            file: None,
            view_state: ViewState::GameView,
            emulator_state: EmulatorState::Unloaded,
            graphics_buffer: [0; GRAPHICS_BUFFER_SIZE],
        }
    }

    pub fn fps(&self) -> usize {
        self.fps
    }

    pub fn quit(&mut self) {
        info!("quitting");
        self.should_quit = true;
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn toggle_run(&mut self) {
        let emulator_state = std::mem::replace(&mut self.emulator_state, EmulatorState::Unloaded);
        match emulator_state {
            EmulatorState::Unloaded => {
                self.emulator_state = emulator_state;
            }
            EmulatorState::Running(_) => {
                self.emulator_state = emulator_state;
            }
            EmulatorState::Paused(e) => {
                let (sender, receiver) = channel::<Message>();
                let handle = e.emulator.run(Some(receiver));
                let state = RunningEmulator { handle, sender };
                self.emulator_state = EmulatorState::Running(state);
            }
        }
    }

    pub fn view_state(&self) -> ViewState {
        self.view_state
    }

    pub fn emulator(&mut self) -> &mut EmulatorState {
        &mut self.emulator_state
    }

    pub fn graphics_buffer(&self) -> &[u8; GRAPHICS_BUFFER_SIZE] {
        &self.graphics_buffer
    }

    pub fn file(&self) -> Option<&String> {
        self.file.as_ref()
    }

    pub fn hertz(&self) -> usize {
        self.hertz
    }

    pub fn timeboxes(&self) -> usize {
        self.timeboxes
    }

    pub fn tick(&mut self) {
        match &mut self.emulator_state {
            EmulatorState::Unloaded => {}
            EmulatorState::Paused(state) => {
                self.graphics_buffer = state.emulator.copy_graphics_buffer();
            }
            EmulatorState::Running(state) => {
                let (gs, gr) = channel();
                if state.sender.send(Message::SendGraphics(gs)).is_ok() {
                    if let Ok(buffer) = gr.recv() {
                        self.graphics_buffer = buffer;
                        return;
                    }
                }

                // An error occurred == channel disconnect
                // the emulator has paused.
                self.toggle_run();
            }
        }
    }

    pub fn emulator_from_file(&mut self, file: &str) -> Result<(), Box<dyn std::error::Error>> {
        let emulator = emulator::Builder::new()
            .with_hertz(self.hertz)
            .with_timeboxes(self.timeboxes)
            .load_program(file)?;
        self.file = Some(file.to_string());
        self.emulator_state = EmulatorState::Paused(PausedEmulator { emulator });
        Ok(())
    }

    pub fn set_key(
        &mut self,
        key: u4,
        status: KeyStatus,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match &self.emulator_state {
            EmulatorState::Running(state) => {
                match state.sender.send(Message::KeyEvent(key, status)) {
                    Ok(_) => {}
                    Err(error) => {
                        error!(%error, "failed to send key event to emulator");
                        return Err(error.into());
                    }
                };
            }
            _ => {
                info!(
                    key = key.value(),
                    ?status,
                    "app received keypress, not forwarding to emulator"
                )
            }
        };
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ViewState {
    GameView,
    DebugView,
}

pub enum EmulatorState {
    Unloaded,
    Paused(PausedEmulator),
    Running(RunningEmulator),
}

pub struct PausedEmulator {
    pub emulator: Emulator,
}

pub struct RunningEmulator {
    pub handle: JoinHandle<Emulator>,
    pub sender: Sender<Message>,
}
