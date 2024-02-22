use std::{
    sync::mpsc::{channel, Sender},
    thread::JoinHandle,
};

use tracing::info;

use chip8::emulator::{self, Message};

pub struct App {
    pub should_quit: bool,
    pub graphics_buffer: [u8; emulator::GRAPHICS_BUFFER_SIZE],
    sender: Option<Sender<emulator::Message>>,
    handle: Option<JoinHandle<emulator::Emulator>>,
    emulator: Option<emulator::Emulator>,
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            graphics_buffer: [0; emulator::GRAPHICS_BUFFER_SIZE],
            sender: None,
            handle: None,
            emulator: None,
        }
    } pub fn quit(&mut self) { info!("quitting");

        self.should_quit = true;
        if let Some(sender) = &self.sender {
            match sender.send(Message::Pause) {
                Ok(_) => {}
                Err(_) => {}
            }
        }

        if let Some(handle) = self.handle.take() {
            handle.join().unwrap();
        }
    }

    pub fn request_data(&mut self) {
        if let Some(sender) = &self.sender {
            let (gs, gr) = channel();
            info!("getting graphics buffer");
            if let Ok(_) = sender.send(Message::SendGraphics(gs)) {
                if let Ok(buffer) = gr.recv() {
                    self.graphics_buffer = buffer;
                    return;
                }
            }

            // An error occurred == channel disconnect
            // the emulator has paused.
            if let Some(handle) = self.handle.take() {
                let emulator = handle.join().unwrap();
                self.graphics_buffer = emulator.copy_graphics_buffer();
                self.emulator = Some(emulator);
            }
        }
    }

    pub fn load_and_run(
        &mut self,
        file: &str,
        hertz: usize,
        timeboxes: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let emulator = emulator::Builder::new()
            .with_hertz(hertz)
            .with_timeboxes(timeboxes)
            .load_program(file)
            .unwrap();
        let (sender, receiver) = channel();
        self.sender = Some(sender);
        self.handle = Some(emulator.run(Some(receiver)));
        Ok(())
    }
}
