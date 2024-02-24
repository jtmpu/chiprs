use std::{
    sync::mpsc::{channel, Sender},
    thread::JoinHandle,
};

use tracing::{error, info};

use chip8::{
    emulator::{self, Emulator, KeyStatus, Message, Snapshot, GRAPHICS_BUFFER_SIZE},
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
    last_snapshot: Snapshot,
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
            last_snapshot: Snapshot::default(),
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

    pub fn is_running(&self) -> bool {
        matches!(self.emulator_state, EmulatorState::Running(_))
    }

    pub fn pause(&mut self) {
        let emulator_state = std::mem::replace(&mut self.emulator_state, EmulatorState::Unloaded);
        match emulator_state {
            EmulatorState::Running(state) => {
                info!("pausing emulator");
                match state.sender.send(Message::Pause) {
                    Ok(_) => {}
                    Err(error) => error!(%error, "failed to send pause command to emulator"),
                };
                let emulator = match state.handle.join() {
                    Ok(e) => e,
                    Err(error) => {
                        error!(?error, "failed to thread::join on emulator");
                        return;
                    }
                };
                self.emulator_state = EmulatorState::Paused(PausedEmulator { emulator });
            }
            _ => {
                self.emulator_state = emulator_state;
            }
        }
    }

    pub fn start(&mut self) {
        let emulator_state = std::mem::replace(&mut self.emulator_state, EmulatorState::Unloaded);
        match emulator_state {
            EmulatorState::Paused(state) => {
                info!("starting emulator");
                let (sender, receiver) = channel::<Message>();
                let handle = state.emulator.run(Some(receiver));
                let state = RunningEmulator { handle, sender };
                self.emulator_state = EmulatorState::Running(state);
            }
            _ => {
                self.emulator_state = emulator_state;
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

    pub fn set_view_state(&mut self, state: ViewState) {
        self.view_state = state;
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
                error!("detected emulator termination, pausing emulator");
                self.pause();
            }
        }
    }

    pub fn emulator_from_file(&mut self, file: &str) -> Result<(), Box<dyn std::error::Error>> {
        let emulator = emulator::Builder::new()
            .with_hertz(self.hertz)
            .with_timeboxes(self.timeboxes)
            .load_program(file)?;
        self.file = Some(file.to_string());
        self.last_snapshot = emulator.create_snapshot();
        self.emulator_state = EmulatorState::Paused(PausedEmulator { emulator });
        Ok(())
    }

    pub fn set_key(
        &mut self,
        key: u4,
        status: KeyStatus,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match &mut self.emulator_state {
            EmulatorState::Paused(state) => {
                // Key presses become toggles in pause mode
                let snapshot = state.emulator.create_snapshot();
                // If key == pressed
                if snapshot.key_status[key.value() as usize] == status {
                    info!(key=?key, "detected previously pressed key, unsetting key");
                    state.emulator.set_key(key, KeyStatus::Up);
                } else {
                    state.emulator.set_key(key, status);
                }
                self.last_snapshot = state.emulator.create_snapshot();
            }
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

    pub fn emulator_step(&mut self) {
        match &mut self.emulator_state {
            EmulatorState::Paused(state) => match state.emulator.tick() {
                Ok(_) => {
                    self.last_snapshot = state.emulator.create_snapshot();
                }
                Err(error) => error!(%error, "failed to step emulator"),
            },
            _ => {}
        }
    }

    pub fn emulator_snapshot(&mut self) -> &Snapshot {
        &self.last_snapshot
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
