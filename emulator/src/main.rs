use clap::Parser;

use std::{fs::File, time::Duration};

use tracing::Level;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt;

mod app;
mod event;
mod tui;
mod ui;
mod update;
mod widgets;

use app::App;
use event::{Event, EventHandler};
use ratatui::{backend::CrosstermBackend, Terminal};
use tui::Tui;

type Err = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Err>;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Arguments {
    #[arg(short, long)]
    file: String,
    #[arg(long, default_value_t = 30)]
    fps: usize,
    #[arg(long, default_value_t = 400)]
    hz: usize,
    #[arg(long, default_value_t = 100)]
    timeboxes: usize,
    #[arg(long, default_value_t = 100)]
    key_press_delay: u64,
}

fn main() -> Result<()> {
    let args = Arguments::parse();
    // Create a rolling file appender
    File::create("emulator.log").unwrap();
    let file_appender = RollingFileAppender::new(Rotation::NEVER, ".", "emulator.log");

    // Create a subscriber with the file appender
    let subscriber = fmt::Subscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_writer(file_appender)
        .finish();

    // Initialize the tracing subscriber
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let mut app = App::new();
    app.load_and_run(&args.file, args.hz, args.timeboxes)
        .unwrap();

    let backend = CrosstermBackend::new(std::io::stderr());
    let terminal = Terminal::new(backend)?;
    let tick_rate = 1_000_000 / args.fps;
    let events = EventHandler::new(tick_rate as u64);
    let mut tui = Tui::new(terminal, events);

    let mut key_handler = update::KeyHandler::new(Duration::from_millis(args.key_press_delay));
    key_handler.bind('0', 0);
    key_handler.bind('1', 1);
    key_handler.bind('2', 2);
    key_handler.bind('3', 3);
    key_handler.bind('4', 4);
    key_handler.bind('5', 5);
    key_handler.bind('6', 6);
    key_handler.bind('7', 7);
    key_handler.bind('8', 8);
    key_handler.bind('9', 9);
    key_handler.bind('a', 10);
    key_handler.bind('b', 11);
    key_handler.bind('c', 12);
    key_handler.bind('d', 13);
    key_handler.bind('e', 14);
    key_handler.bind('f', 15);

    tui.enter()?;
    while !app.should_quit {
        app.request_data();
        tui.draw(&mut app)?;

        match tui.events.next()? {
            Event::Tick => key_handler.tick(&mut app),
            Event::KeyEvent(key_event) => key_handler.handle_key(&mut app, key_event),
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
        }
    }

    tui.exit()?;
    Ok(())
}
