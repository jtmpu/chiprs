
use std::fs::File;

use clap::Parser;
use tracing::{error, span, Level};

use chip8::assembly::parser;

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

    let reader = File::open(args.file).unwrap();
    let mut parser = parser::Parser::new(reader);
    let assembly = parser.parse();

    println!("{:?}", assembly);
}
