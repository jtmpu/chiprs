use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

/// Terminal events
#[derive(Clone, Copy, Debug)]
pub enum Event {
    /// Terminal tick
    Tick,
    /// Key press
    KeyPress(KeyEvent),
    KeyRelease(KeyEvent),
    /// Mouse click/scroll
    Mouse(MouseEvent),
    /// Terminal resize
    Resize(u16, u16),
}

#[derive(Debug)]
pub struct EventHandler {
    /// Event sender channel
    #[allow(dead_code)]
    sender: mpsc::Sender<Event>,
    /// Event receiver channel
    receiver: mpsc::Receiver<Event>,
    /// Event handler thread
    #[allow(dead_code)]
    handler: thread::JoinHandle<()>,
}

impl EventHandler {
    pub fn new(tick_rate: u64) -> Self {
        let tick_rate = Duration::from_micros(tick_rate);
        let (sender, receiver) = mpsc::channel();
        let handler = {
            let sender = sender.clone();
            thread::spawn(move || {
                let mut last_tick = Instant::now();
                loop {
                    let timeout = tick_rate
                        .checked_sub(last_tick.elapsed())
                        .unwrap_or(tick_rate);

                    if event::poll(timeout).expect("unable to poll for events") {
                        match event::read().expect("unable to read event") {
                            CrosstermEvent::Key(e) => {
                                if e.kind == event::KeyEventKind::Press {
                                    sender.send(Event::KeyPress(e))
                                } else if e.kind == event::KeyEventKind::Release {
                                    sender.send(Event::KeyRelease(e))
                                } else {
                                    Ok(())
                                }
                            }
                            CrosstermEvent::Mouse(e) => sender.send(Event::Mouse(e)),
                            CrosstermEvent::Resize(w, h) => sender.send(Event::Resize(w, h)),
                            _ => unimplemented!(),
                        }
                        .expect("failed to send terminal event")
                    }

                    if last_tick.elapsed() >= tick_rate {
                        sender.send(Event::Tick).expect("failed to send tick event");
                        last_tick = Instant::now();
                    }
                }
            })
        };
        Self {
            sender,
            receiver,
            handler,
        }
    }

    pub fn next(&self) -> Result<Event, Box<dyn std::error::Error>> {
        Ok(self.receiver.recv()?)
    }
}
