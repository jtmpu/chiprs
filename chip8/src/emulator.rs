//!
//! Chip-8 emulator
//!

use std::fmt;
use std::io::{Read, Cursor, Write, Seek, SeekFrom};
use std::error::Error;

use tracing::{debug, error, span, Level};

use crate::instructions::Instruction;

#[derive(Debug, Clone)]
pub enum Chip8Error {
    UnimplementedInstruction,
    InvalidOpcode(String),
    StackEmpty,
    StackFull,
}

impl fmt::Display for Chip8Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "chip-8 failure: {:?}", self)
    }
}
impl Error for Chip8Error {
}


pub const MEMSIZE: usize = 4096;
pub const START_ADDR: usize = 0x200;

pub const REGISTRY_COUNT: usize = 16;
pub const STACK_SIZE: usize = 32;
pub const GRAPHICS_BUFFER_SIZE: usize = 256;

#[macro_export]
macro_rules! fill_mem {
    ($mem:expr,$offset:expr,$($elem:expr),*) => {
        {
            let mut data: Vec<u8> = Vec::new();
            $(
                data.push($elem);
            )*
            let mut cursor = Cursor::new($mem.as_mut_slice());
            cursor.seek(SeekFrom::Start($offset as u64))?;
            let result = cursor.write_all(&data);
            result
        }
    }
}

pub struct Emulator {
    memory: [u8; MEMSIZE],
    registries: [u8; REGISTRY_COUNT],
    program_counter: usize,
    stack_pointer: usize,
    stack: [usize; STACK_SIZE],
    graphics_buffer: [u8; GRAPHICS_BUFFER_SIZE],
}

impl Emulator {
    pub fn new() -> Self {
        let mut ret = Self {
            memory: [0; MEMSIZE],
            registries: [0; REGISTRY_COUNT],
            program_counter: START_ADDR,
            stack_pointer: 0,
            stack: [0; STACK_SIZE],
            graphics_buffer: [0; GRAPHICS_BUFFER_SIZE],
        };
        ret.reset();
        ret
    }

    /// Resets everything in the emulator
    pub fn reset(&mut self) {
        self.memory = [0; MEMSIZE];
        self.registries = [0; REGISTRY_COUNT];
        self.program_counter = START_ADDR;
        self.stack_pointer = 0;
        self.stack = [0; STACK_SIZE];
        self.graphics_buffer = [0; GRAPHICS_BUFFER_SIZE];
        self.load_default_sprites().unwrap();
    }

    /// Loads the default sprites which should be available.
    /// These are placed in the 0x00-0x1FF range
    fn load_default_sprites(&mut self) -> std::io::Result<()> {
        const START: usize = 0x00;
        // 0
        fill_mem!(self.memory, (START), 0xF0, 0x90, 0x90, 0x90, 0xF0)?;
        // 1
        fill_mem!(self.memory, (START + 6), 0x20, 0x60, 0x20, 0x20, 0x70)?;
        // 2
        fill_mem!(self.memory, (START + 11), 0xF0, 0x10, 0xF0, 0x80, 0xF0)?;
        // 3
        fill_mem!(self.memory, (START + 16), 0xF0, 0x10, 0xF0, 0x10, 0xF0)?;
        Ok(())
    }

    pub fn copy_bytes(&self, start: usize, amount: usize) -> Vec<u8> {
        let mut ret = Vec::new();
        for index in start..(start + amount) {
            ret.push(self.memory[index]);
        }
        ret
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
            return Err(Chip8Error::InvalidOpcode(format!("0x{:02x}{:02x}", big, little)).into());
        };
        Ok(instruction)
    }

    pub fn tick(&mut self) -> Result<bool, Box<dyn Error>> {
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
        let res = match self.execute(instruction) {
            Err(e) => {
                error!(
                    pc = self.program_counter,
                    error = ?e,
                    "failed to execute instruction"
                );
                return Err(e.into())
            },
            res => res,
        };
        res
    }

    pub fn execute(&mut self, instruction: Instruction) -> Result<bool, Box<dyn Error>> {
        debug!(instruction = ?instruction, "executing instruction");
        match instruction {
            Instruction::Exit => {
                // Kill execution
                return Ok(false);
            },
            Instruction::Clear => {
                // Currently noop
            },
            Instruction::Return => {
                if self.stack_pointer <= 0 {
                    return Err(Chip8Error::StackEmpty.into());
                }
                self.stack_pointer -= 1;
                self.program_counter = self.stack[self.stack_pointer];
            },
            Instruction::Call(addr) => {
                if self.stack_pointer >= STACK_SIZE {
                    return Err(Chip8Error::StackFull.into());
                }
                self.stack[self.stack_pointer] = self.program_counter;
                self.stack_pointer += 1;
                self.program_counter = addr.value() as usize;
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
            Instruction::Or(regx, regy) => {
                let vx = self.registries[regx.value() as usize];
                let vy = self.registries[regy.value() as usize];
                self.registries[regx.value() as usize] = vx | vy;
            },
            Instruction::Draw(regx, regy, n) => {
                todo!();
            },
            Instruction::SetMemRegisterDefaultSprit(regx) => {
                todo!();
            },
        };
        Ok(true)
    }

    /// Executes machine until abort opcode is called
    pub fn run(&mut self) {
        loop {
            match self.tick() {
                Ok(false) => break,
                Err(_) => break,
                _ => {},
            }
        }
    }

    pub fn dump_state(&self) {
        println!("Emulator state:");
        println!("");

        println!("program-counter: {:04x}", self.program_counter);
        println!(" - instruction: {:?}", self.instruction());
        println!("");

        println!("stack:");
        println!(" - stack-pointer: {}", self.stack_pointer);
        for index in (0..self.stack_pointer).rev() {
            println!("  [{}]: 0x{:04x}", index, self.stack[index]);
        }
        println!("");

        let mut regstr = "regs: ".to_string();
        for (index, reg) in self.registries.iter().enumerate() {
            regstr.push_str(format!(" r{}:{:02x}", index, reg).as_str());
        }
        println!("{}", regstr);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::io::{BufReader, Cursor};
    use crate::assembly::parser::Parser;
    use crate::assembly::lexer::StreamLexer;

    fn create_execute(input: &'static str) -> Emulator {
        let reader = BufReader::new(input.as_bytes());
        let lexer = StreamLexer::new(reader);
        let mut parser = Parser::new(Box::new(lexer));
        let binary = parser
            .parse().unwrap()
            .binary().unwrap();
        let mut emulator = Emulator::new();
        let cursor = Cursor::new(binary);
        emulator.load(cursor).unwrap();
        emulator.run();
        emulator
    }

    fn reg_value(emu: &Emulator, index: usize) -> u8 {
        emu.registries[index]
    }

    #[test]
    fn test_add() {
        let e = create_execute("
            mov r2 1
            mov r1 0
            add r1 2
            add r1 10
            add r2 4
            exit"
        );
        assert_eq!(reg_value(&e, 1), 12);
        assert_eq!(reg_value(&e, 2), 5);
    }

    #[test]
    fn test_branch_jmp_sne() {
        let e = create_execute("
            mov r1 0
            add r2 0
        loop:
            sne r1 4
            jmp exit
            add r1 1
            add r2 4
            jmp loop
        exit:
            exit"
        );
        assert_eq!(reg_value(&e, 1), 4);
        assert_eq!(reg_value(&e, 2), 16);
    }

    #[test]
    fn test_call_ret() {
        let e = create_execute("
        main:
            mov r1 0
            add r1 2
            call func1
            call func2
            call func1
            exit

        func1:
            add r1 4
            call func2
            ret
        
        func2:
            add r1 2
            ret"
        );
        assert_eq!(reg_value(&e, 1), 16);
    }

    #[test]
    fn test_default_sprites() {
        let mut emulator = Emulator::new();
        emulator.reset();

        // offset 6, length 5, should be "1" sprite
        let sprite = emulator.copy_bytes(6, 5);
        assert_eq!(sprite[0], 0x20, "failed on offset: {}", 0);
        assert_eq!(sprite[1], 0x60, "failed on offset: {}", 1);
        assert_eq!(sprite[2], 0x20, "failed on offset: {}", 2);
        assert_eq!(sprite[3], 0x20, "failed on offset: {}", 3);
        assert_eq!(sprite[4], 0x70, "failed on offset: {}", 4);
    }
}
