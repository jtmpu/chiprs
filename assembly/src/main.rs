
use std::fs::File;
use std::io::{self, BufReader, Write, Read};

use clap::{Parser, Subcommand, Args};
use tracing::{Level, error};

use chip8;
use chip8::assembly::lexer::Lexer;
use chip8::instructions::Instruction;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CliArgs {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long)]
    debug: bool,

    #[arg(short, long)]
    verbose: bool,

    #[clap(long)]
    #[clap(help = "message format for logging")]
    #[clap(value_enum, default_value_t=LogFormat::Plain)]
    log_format: LogFormat,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Asm(AssemblyCommands),
    Disasm(DisassembleCommands),
}

#[derive(Debug, Args)]
struct AssemblyCommands {
    #[arg(short, long)]
    input: Option<String>,

    #[arg(short, long)]
    ast: bool,

    #[arg(short, long)]
    output: Option<String>,
}

#[derive(Debug, Args)]
struct DisassembleCommands {
    #[arg(short, long)]
    input: Option<String>,

    #[arg(short, long)]
    ast: bool,

    #[arg(short, long)]
    output: Option<String>,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum LogFormat {
    Json,
    Pretty,
    Plain,
}

fn configure_logger(args: &CliArgs) {
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
    let args = CliArgs::parse();
    configure_logger(&args);

    match &args.command {
        Some(Commands::Asm(a)) => {
            run_assembler(a, &args);
        },
        Some(Commands::Disasm(a)) => {
            run_disassembler(a, &args);
        },
        None => {},
    };
}

fn run_assembler(args: &AssemblyCommands, _global_args: &CliArgs) {
    let lexer: Box<dyn Lexer> = if let Some(f) = &args.input {
        let file = File::open(f).unwrap();
        let lexer = chip8::assembly::lexer::StreamLexer::new(file);
        Box::new(lexer)
    } else {
        let reader = BufReader::new(io::stdin());
        let lexer = chip8::assembly::lexer::StreamLexer::new(reader);
        Box::new(lexer)
    };
    let mut parser = chip8::assembly::parser::Parser::new(lexer);
    let assembly = match parser.parse() {
        Ok(asm) => asm,
        Err(e) => {
            error!("failed to parse assembly: {}", e.to_string());
            return;
        }
    };

    if args.ast {
        for i in assembly.instructions {
            println!("{:?}", i);
        }
        return;
    }

    let binary = assembly.binary().unwrap();
    if let Some(output) = &args.output {
        let mut file = File::create(output).unwrap(); 
        file.write_all(binary.as_ref()).unwrap();
    } else {
        let mut stdout = io::stdout();
        stdout.write_all(binary.as_ref()).unwrap();
    }
}

fn run_disassembler(args: &DisassembleCommands, _global_args: &CliArgs) {
    let mut reader: Box<dyn Read> = if let Some(f) = &args.input {
        Box::new(File::open(f).unwrap())
    } else {
        Box::new(BufReader::new(io::stdin()))
    };
    
    let mut buffer: Vec<u8> = Vec::new();
    reader.read_to_end(&mut buffer).unwrap();

    let mut cursor = 0;
    let mut instructions: Vec<Instruction> = Vec::new();
    loop {
        if cursor >= buffer.len() || cursor + 1 >= buffer.len() {
            break;
        }
        let b1 = buffer[cursor];
        let b2 = buffer[cursor + 1];
        match Instruction::from_opcode_u8(b1, b2) {
            Some(i) => instructions.push(i),
            None => {
                error!("unknown opcode '0x{:02x}{:02x}'", b1, b2);
            }

        };
        cursor += 2;
    }

    if args.ast {
        for i in instructions {
            println!("{:?}", i);
        }
        return;
    }

    let mut writer: Box<dyn Write> = if let Some(f) = &args.output {
        Box::new(File::create(f).unwrap())
    } else {
        Box::new(io::stdout())
    };

    for i in instructions {
        let mut asm = i.to_assembly();
        asm.push_str("\n");
        writer.write_all(asm.as_bytes()).unwrap();
    }
}
