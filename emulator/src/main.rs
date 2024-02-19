use std::{path::Path, fs::File, io::Read, io, io::stdout};

use crossterm::{
    event::{self, KeyCode, KeyEventKind},
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
        LeaveAlternateScreen
    },
    ExecutableCommand,
};
use ratatui::{
    prelude::{CrosstermBackend, Stylize, Terminal},
    widgets::Paragraph,
};

use chip8::emulator::Emulator;
use clap::Parser;
use tracing::{error, span, Level};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// File to load chip-8 binary from
    #[arg(short, long)]
    file: Option<String>,

    #[arg(short, long)]
    ticks: Option<usize>,

    #[arg(short, long)]
    debug: bool,

    #[arg(long)]
    headless: bool,

    #[arg(short, long)]
    verbose: bool,
    
    #[clap(long)]
    #[clap(help = "message format for logging")]
    #[clap(value_enum, default_value_t=LogFormat::Plain)]
    log_format: LogFormat,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum LogFormat {
    Json,
    Pretty,
    Plain,
}

fn configure_logger(args: &Args) {
    let level = if args.debug {
        Level::DEBUG
    } else if args.verbose {
        Level::INFO
    } else {
        Level::WARN
    };

    let sub = tracing_subscriber::fmt()
        .with_max_level(level);

    match args.log_format {
        LogFormat::Json => {
            sub.json().init();
        }, 
        LogFormat::Pretty => {
            sub.pretty().init();
        },
        LogFormat::Plain => {
            sub.init();
        },
    };
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    // configure_logger(&args);

    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    loop {
        terminal.draw(|frame| {
            let area = frame.size();
            frame.render_widget(
                Paragraph::new("Chip-8 (press 'q' to quit)")
                    .white()
                    .on_blue(),
                area,
            );
        })?;

        if event::poll(std::time::Duration::from_millis(16))? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press 
                    && key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}


fn create_load_emulator(args: &Args) -> Result<Emulator, ()> {
    let reader: Box<dyn Read> = if let Some(f) = &args.file {
        let path = Path::new(f);
        if !path.exists() {
            error!("chip-8 bin file doesn't exist: {}", path.display()); 
            return Err(());
        }

        let reader = match File::open(path) {
            Ok(r) => r,
            Err(err) => {
                error!(error = ?err, "failed to open chip-8 binary file");
                return Err(());
            }
        };
        Box::new(reader)
    } else {
        Box::new(io::stdin()) 
    };

    let mut emulator = Emulator::new();
    match emulator.load(reader) {
        Ok(_) => return Ok(emulator),
        Err(e) => {
            error!("failed to load emulator: {}", e);
            return Err(());
        }
    }
}
