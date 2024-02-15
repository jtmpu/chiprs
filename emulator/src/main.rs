use std::{path::Path, fs::File};

use chip8::emulator::Emulator;
use clap::Parser;
use tracing::{error, span, Level};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// File to load chip-8 binary from
    #[arg(short, long)]
    file: String,

    #[arg(short, long)]
    debug: bool,

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

fn main() {
    let args = Args::parse();

    configure_logger(&args);

    let span = span!(Level::INFO, "emulator-bin:main");
    let _guard = span.enter();
    let path = Path::new(args.file.as_str());
    if !path.exists() {
        error!("chip-8 bin file doesn't exist: {}", path.display()); 
        return;
    }

    let reader = match File::open(path) {
        Ok(r) => r,
        Err(err) => {
            error!(error = ?err, "failed to open chip-8 binary file");
            return;
        }
    };

    let mut emulator = Emulator::new();
    match emulator.load(reader) {
        Ok(_) => {},
        Err(e) => {
            error!("failed to load emulator: {}", e);
            return;
        }
    }
    for _ in 1..32 {
        emulator.tick().unwrap();
    }
    emulator.dump_state();
}
