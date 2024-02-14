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
}

fn main() {
    tracing_subscriber::fmt()
        .json()
        .init();
    let args = Args::parse();

    let _guard = span!(Level::INFO, "emulator-bin:main");
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
        emulator.dump_state();
    }
}
