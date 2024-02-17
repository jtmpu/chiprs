//!
//! Chip-8 emulator
//!

use std::fmt;
use std::io::Read;
use std::error::Error;

use tracing::{debug, error, span, Level};

use crate::instructions::Instruction;

#[derive(Debug, Clone)]
pub enum Chip8Error {
    UnimplementedInstruction,
    InvalidOpcode,
}

impl fmt::Display for Chip8Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "chip-8 failure")
    }
}
impl Error for Chip8Error {
}


pub const MEMSIZE: usize = 4096;
pub const START_ADDR: usize = 0x200;

pub const REGISTRY_COUNT: usize = 16;
pub const STACK_SIZE: usize = 16;

pub struct Emulator {
    memory: [u8; MEMSIZE],
    registries: [u8; REGISTRY_COUNT],
    program_counter: usize,
    stack_pointer: u8,
    stack: [u16; STACK_SIZE],
}

impl Emulator {
    pub fn new() -> Self {
        Self {
            memory: [0; MEMSIZE],
            registries: [0; REGISTRY_COUNT],
            program_counter: START_ADDR,
            stack_pointer: 0,
            stack: [0; STACK_SIZE],
        }
    }

    /// Resets everything in the emulator
    pub fn reset(&mut self) {
        debug!("reseting emulator");

        self.memory = [0; MEMSIZE];
        self.registries = [0; REGISTRY_COUNT];
        self.program_counter = START_ADDR;
        self.stack_pointer = 0;
        self.stack = [0; STACK_SIZE];
    }

    pub fn load<T: Read>(&mut self, mut reader: T) -> Result<(), Box<dyn Error>> {
        self.reset();
        let bytes = match reader.read(&mut self.memory[START_ADDR..]) {
            Ok(n) => n,
            Err(e) => {
                error!(error = ?e, "failed to load bytes into emulator memory");
                return Err(e.into());
            }
        };
        debug!(%bytes, "loaded bytes into emulator memory");
        Ok(())
    }

    fn instruction(&self) -> Result<Instruction, Box<dyn Error>> {
        // Expecting big endian
        let big = self.memory[self.program_counter];
        let little = self.memory[self.program_counter + 1];
        let instruction = Instruction::from_opcode_u8(
            big,
            little,
        );
        let instruction = if let Some(i) = instruction {
            i
        } else {
            return Err(Chip8Error::InvalidOpcode.into());
        };
        Ok(instruction)
    }

    pub fn tick(&mut self) -> Result<(), Box<dyn Error>> {
        let span = span!(Level::INFO, "emulator.tick");
        let _guard = span.enter();
        let instruction = match self.instruction() {
            Ok(i) => i,
            Err(e) => {
                error!(
                    pc = self.program_counter,
                    error = ?e, 
                    "failed to parse instruction opcode"
                );
                return Err(e.into())
            }
        };
        self.program_counter += 2;
        if let Err(e) = self.execute(instruction) {
            error!(
                pc = self.program_counter,
                error = ?e,
                "failed to execute instruction"
            );
            return Err(e.into())
        }
        Ok(())
    }

    pub fn execute(&mut self, instruction: Instruction) -> Result<(), Box<dyn Error>> {
        debug!(instruction = ?instruction, "executing instruction");
        match instruction {
            Instruction::Clear => {
                // Currently noop
            },
            Instruction::Jump(addr) => {
                self.program_counter = addr.value() as usize;
            },
            Instruction::SkipNotEqual(register, value) => {
                let index = register.value() as usize;
                if self.registries[index] != value {
                    self.program_counter += 2;
                }
            },
            Instruction::Move(register, value) => {
                self.registries[register.value() as usize] = value;
            },
            Instruction::Add(register, value) => {
                self.registries[register.value() as usize] += value;
            },
        };
        Ok(())
    }

    pub fn dump_state(&self) {
        println!("Emulator state:");
        println!("");

        println!("Next instruction: {:?}", self.instruction());
        println!("");

        println!("PC: {:04x}", self.program_counter);
        println!("SP: {:02x}", self.stack_pointer);
        println!("");

        let mut regstr = "regs: ".to_string();
        for (index, reg) in self.registries.iter().enumerate() {
            regstr.push_str(format!(" r{}:{:02x}", index, reg).as_str());
        }
        println!("{}", regstr);
    }
}
