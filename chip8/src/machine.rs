///  
/// Machine
/// Runs the emulator in a separate thread, and exposes
/// a communication interface to manage and retrieve information
/// about the emulator
///
use std::{
    sync::mpsc::{Receiver, Sender},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use tracing::{debug, info};

use crate::emulator::{Emulator, GRAPHICS_BUFFER_SIZE};

pub enum Message {
    Terminate,
    SendGraphics(Sender<[u8; GRAPHICS_BUFFER_SIZE]>),
}

pub struct Machine {
    // The "CPU" Hz, commonly between 400-4000
    hertz: usize,
    // Divide second into <timeboxes> many controlled executions
    // to spread out execution of instructions over the second
    timeboxes: usize,
    emulator: Emulator,
    receiver: Receiver<Message>,
}

impl Machine {
    pub fn new(
        hertz: usize,
        timeboxes: usize,
        emulator: Emulator,
        receiver: Receiver<Message>,
    ) -> Self {
        Self {
            hertz,
            timeboxes,
            emulator,
            receiver,
        }
    }

    pub fn start(self) -> JoinHandle<()> {
        thread::spawn(move || {
            let mut own = self;
            own.run();
        })
    }

    fn process_message(&self, message: Message) -> bool {
        match message {
            Message::Terminate => {
                info!("received terminate");
                return true;
            }
            Message::SendGraphics(channel) => {
                debug!("received graphics request");
                match channel.send(self.emulator.copy_graphics_buffer()) {
                    Ok(_) => {}
                    Err(_) => {
                        info!("failed to send graphics buffer, terminating");
                        return true;
                    }
                };
            }
        };
        false
    }

    fn run(&mut self) {
        // in microseconds
        let delay_per_second = 1_000_000_000;
        let delay_per_timebox = (delay_per_second / self.timeboxes) as u128;
        let ticks_per_timebox = self.hertz / self.timeboxes;

        info!(%ticks_per_timebox, %delay_per_timebox, "starting chip-8 machine");
        let mut ticks = 0;
        let mut last_tick = Instant::now();
        loop {
            if ticks < ticks_per_timebox {
                // keep ticking while we're allowed in the timebox
                // check and handle any message requests
                let should_abort = match self.receiver.try_recv() {
                    Ok(message) => self.process_message(message),
                    Err(_) => false,
                };
                if should_abort {
                    break;
                }

                match self.emulator.tick() {
                    Ok(_) => {}
                    Err(_) => {}
                }
                ticks += 1;
            } else {
                if last_tick.elapsed().as_nanos() < delay_per_timebox {
                    // listen for message requests until we can execute more ticks
                    let timeout = delay_per_timebox - last_tick.elapsed().as_nanos();
                    let should_abort = match self
                        .receiver
                        .recv_timeout(Duration::from_nanos(timeout as u64))
                    {
                        Ok(message) => self.process_message(message),
                        Err(_) => false,
                    };
                    if should_abort {
                        break;
                    }
                }
                ticks = 0;
                last_tick = Instant::now();
            }
        }

        info!("terminating chip-8 machine");
    }
}
