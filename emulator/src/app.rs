use std::{
    fs::File,
    sync::mpsc::{Sender, Receiver, channel},
    thread::JoinHandle,
};

use tracing::info;

use chip8::{
    emulator,
    machine,
};

pub struct App {
    pub should_quit: bool,
    sender: Option<Sender<machine::Message>>,
    handle: Option<JoinHandle<()>>,
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
            sender.send(machine::Message::Terminate).unwrap();
        }

        if let Some(handle) = self.handle.take() {
            handle.join().unwrap();
        }
    }

    pub fn get_graphics_buffer(&self) -> [u8; emulator::GRAPHICS_BUFFER_SIZE] {
        if let Some(sender) = &self.sender {
            let (gs, gr) = channel();
            sender.send(machine::Message::SendGraphics(gs)).unwrap();
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
        let reader = File::open(file)?;
        let mut emulator = emulator::Emulator::new();
        emulator.load(reader)?;
        let (sender, receiver) = channel();
        let machine = machine::Machine::new(hertz, timeboxes, emulator, receiver);
        self.sender = Some(sender);
        self.handle = Some(machine.start());
        Ok(())
    }
}
