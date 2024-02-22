use clap::Parser;

use std::fs::File;

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
use update::update;

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
}

fn main() -> Result<()> {
    let args = Arguments::parse();
    // Create a rolling file appender
    File::create("emulator.log").unwrap();
    let file_appender = RollingFileAppender::new(Rotation::NEVER, ".", "emulator.log");

    // Create a subscriber with the file appender
    let subscriber = fmt::Subscriber::builder()
        .with_max_level(Level::INFO)
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
    tui.enter()?;

    while !app.should_quit {
        app.request_data();
        tui.draw(&mut app)?;

        match tui.events.next()? {
            Event::Tick => {}
            Event::Key(key_event) => update(&mut app, key_event),
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
        }
    }

    tui.exit()?;
    Ok(())
}
