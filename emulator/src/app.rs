use std::{
    sync::mpsc::{channel, Sender},
    thread::JoinHandle,
};

use tracing::info;

use chip8::emulator::{self, Message};

pub struct App {
    pub should_quit: bool,
    sender: Option<Sender<emulator::Message>>,
    handle: Option<JoinHandle<emulator::Emulator>>,
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            sender: None,
            handle: None,
        }
    }

    pub fn quit(&mut self) {
        info!("quitting");

        self.should_quit = true;
        if let Some(sender) = &self.sender {
            sender.send(Message::Pause).unwrap();
        }

        if let Some(handle) = self.handle.take() {
            handle.join().unwrap();
        }
    }

    pub fn get_graphics_buffer(&self) -> [u8; emulator::GRAPHICS_BUFFER_SIZE] {
        if let Some(sender) = &self.sender {
            let (gs, gr) = channel();
            sender.send(Message::SendGraphics(gs)).unwrap();
            gr.recv().unwrap()
        } else {
            [0; emulator::GRAPHICS_BUFFER_SIZE]
        }
    }

    pub fn load_and_run(
        &mut self,
        file: &str,
        hertz: usize,
        timeboxes: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (sender, receiver) = channel();
        let emulator = emulator::Builder::new()
            .with_hertz(hertz)
            .with_timeboxes(timeboxes)
            .with_channel(receiver)
            .load_program(file)
            .unwrap();
        self.sender = Some(sender);
        self.handle = Some(emulator.run());
        Ok(())
    }
}
