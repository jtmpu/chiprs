use std::fs::File;

use chip8::emulator::{self, Emulator};

pub struct App {
    pub should_quit: bool,
    emulator: Emulator,
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            emulator: Emulator::new(),
        }
    }

    // Handles tick event of the terminal
    pub fn tick(&self) {}

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn get_graphics_buffer(&self) -> [u8; emulator::GRAPHICS_BUFFER_SIZE] {
        self.emulator.copy_graphics_buffer()
    }

    pub fn load_and_run(
        &mut self,
        file: &str,
        _ticks: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let reader = File::open(file)?;
        self.emulator.load(reader)?;
        self.emulator.run();
        Ok(())
    }
}
