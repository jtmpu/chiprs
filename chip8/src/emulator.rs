//!
//! Chip-8 emulator
//!

use std::error::Error;
use std::fmt;
use std::io::Read;

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
impl Error for Chip8Error {}

pub const MEMSIZE: usize = 4096;
pub const START_ADDR: usize = 0x200;

pub const REGISTRY_COUNT: usize = 16;
pub const STACK_SIZE: usize = 32;
pub const DISPLAY_WIDTH: usize = 64;
pub const DISPLAY_HEIGHT: usize = 32;
pub const GRAPHICS_BUFFER_SIZE: usize = (DISPLAY_WIDTH * DISPLAY_HEIGHT) / 8;

pub const DEFAULT_SPRITE_START_ADDR: usize = 0x00;
pub const DEFAULT_SPRITES: [[u8; 5]; 16] = [
    // 0
    [0xF0, 0x90, 0x90, 0x90, 0xF0],
    // 1
    [0x20, 0x60, 0x20, 0x20, 0x70],
    // 2
    [0xF0, 0x10, 0xF0, 0x80, 0xF0],
    // 3
    [0xF0, 0x10, 0xF0, 0x10, 0xF0],
    // 4
    [0x90, 0x90, 0xF0, 0x10, 0x10],
    // 5
    [0xF0, 0x80, 0xF0, 0x10, 0xF0],
    // 6
    [0xF0, 0x80, 0xF0, 0x90, 0xF0],
    // 7
    [0xF0, 0x10, 0x20, 0x40, 0x40],
    // 8
    [0xF0, 0x90, 0xF0, 0x90, 0xF0],
    // 9
    [0xF0, 0x90, 0xF0, 0x10, 0xF0],
    // A
    [0xF0, 0x90, 0xF0, 0x90, 0x90],
    // B
    [0xE0, 0x90, 0xE0, 0x90, 0xE0],
    // C
    [0xF0, 0x80, 0x80, 0x80, 0xF0],
    // D
    [0xE0, 0x90, 0x90, 0x90, 0xE0],
    // E
    [0xF0, 0x80, 0xF0, 0x80, 0xF0],
    // F
    [0xF0, 0x80, 0xF0, 0x80, 0x80],
];

pub struct Emulator {
    memory: [u8; MEMSIZE],
    registries: [u8; REGISTRY_COUNT],
    program_counter: usize,
    stack_pointer: usize,
    // I registry
    address_register: usize,
    stack: [usize; STACK_SIZE],
    graphics_buffer: [u8; GRAPHICS_BUFFER_SIZE],
}

impl Default for Emulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Emulator {
    pub fn new() -> Self {
        let mut ret = Self {
            memory: [0; MEMSIZE],
            registries: [0; REGISTRY_COUNT],
            program_counter: START_ADDR,
            stack_pointer: 0,
            address_register: 0,
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
        self.address_register = 0;
        self.stack = [0; STACK_SIZE];
        self.graphics_buffer = [0; GRAPHICS_BUFFER_SIZE];
        self.load_default_sprites().unwrap();
    }

    /// Loads the default sprites which should be available.
    /// These are placed in the 0x00-0x1FF range
    fn load_default_sprites(&mut self) -> std::io::Result<()> {
        for (offset, sprite) in DEFAULT_SPRITES.iter().enumerate() {
            for (i, item) in sprite.iter().enumerate() {
                self.memory[DEFAULT_SPRITE_START_ADDR + (offset * 5) + i] = *item;
            }
        }
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
        let instruction = Instruction::from_opcode_u8(big, little);
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
                return Err(e);
            }
        };
        self.program_counter += 2;

        match self.execute(instruction) {
            Err(e) => {
                error!(
                    pc = self.program_counter,
                    error = ?e,
                    "failed to execute instruction"
                );
                Err(e)
            }
            res => res,
        }
    }

    pub fn execute(&mut self, instruction: Instruction) -> Result<bool, Box<dyn Error>> {
        debug!(instruction = ?instruction, "executing instruction");
        match instruction {
            Instruction::Exit => {
                // Kill execution
                return Ok(false);
            }
            Instruction::Clear => {
                // Currently noop
            }
            Instruction::Return => {
                if self.stack_pointer == 0 {
                    return Err(Chip8Error::StackEmpty.into());
                }
                self.stack_pointer -= 1;
                self.program_counter = self.stack[self.stack_pointer];
            }
            Instruction::Call(addr) => {
                if self.stack_pointer >= STACK_SIZE {
                    return Err(Chip8Error::StackFull.into());
                }
                self.stack[self.stack_pointer] = self.program_counter;
                self.stack_pointer += 1;
                self.program_counter = addr.value() as usize;
            }
            Instruction::Jump(addr) => {
                self.program_counter = addr.value() as usize;
            }
            Instruction::SkipNotEqual(register, value) => {
                let index = register.value() as usize;
                if self.registries[index] != value {
                    self.program_counter += 2;
                }
            }
            Instruction::Move(register, value) => {
                self.registries[register.value() as usize] = value;
            }
            Instruction::Add(register, value) => {
                self.registries[register.value() as usize] += value;
            }
            Instruction::Or(regx, regy) => {
                let vx = self.registries[regx.value() as usize];
                let vy = self.registries[regy.value() as usize];
                self.registries[regx.value() as usize] = vx | vy;
            }
            Instruction::Draw(regx, regy, n) => {
                let mut vf = 0;

                let x = self.registries[regx.value() as usize] as usize;
                let y = self.registries[regy.value() as usize] as usize;
                let start = x / 8 + y * 8;
                // render each line separetly
                for i in 0..n.value() {
                    let sprite = self.memory[self.address_register + (i as usize)];
                    let sp1 = sprite >> (x % 8);
                    let sp2 = ((sprite as u16) << (8 - (x % 8))) as u8;

                    let i1 = start + (i as usize) * 8;
                    let i2 = start + 1 + (i as usize) * 8;

                    let byte1 = self.graphics_buffer[i1];
                    self.graphics_buffer[i1] = byte1 ^ sp1;
                    let byte2 = self.graphics_buffer[i2];
                    self.graphics_buffer[i2] = byte2 ^ sp2;

                    vf |= (byte1 ^ sp1) ^ (byte1 | sp1);
                    vf |= (byte2 ^ sp2) ^ (byte2 | sp2);
                }

                if vf > 0 {
                    self.registries[0x0F_usize] = 1;
                }
            }
            Instruction::SetMemRegisterDefaultSprit(regx) => {
                let hex_digit = self.registries[regx.value() as usize];
                if hex_digit > 0x0F {
                    // Panic? Fail?
                }
                // sprites are sequential, 0 -> F, and always 5 bytes. Just calculate
                // the offset from the start location
                self.address_register = DEFAULT_SPRITE_START_ADDR + ((hex_digit as usize) * 5);
            }
        };
        Ok(true)
    }

    /// Executes machine until abort opcode is called
    pub fn run(&mut self) {
        loop {
            match self.tick() {
                Ok(false) => break,
                Err(_) => break,
                _ => {}
            }
        }
    }

    pub fn copy_graphics_buffer(&self) -> [u8; GRAPHICS_BUFFER_SIZE] {
        self.graphics_buffer
    }

    pub fn dump_state(&self) {
        println!("Emulator state:");
        println!();

        println!("program-counter: {:04x}", self.program_counter);
        println!(" - instruction: {:?}", self.instruction());
        println!();

        println!("stack:");
        println!(" - stack-pointer: {}", self.stack_pointer);
        for index in (0..self.stack_pointer).rev() {
            println!("  [{}]: 0x{:04x}", index, self.stack[index]);
        }
        println!();

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

    use crate::assembly::lexer::StreamLexer;
    use crate::assembly::parser::Parser;
    use std::io::{BufReader, Cursor};

    fn create_execute(input: &'static str) -> Emulator {
        let reader = BufReader::new(input.as_bytes());
        let lexer = StreamLexer::new(reader);
        let mut parser = Parser::new(Box::new(lexer));
        let binary = parser.parse().unwrap().binary().unwrap();
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
        let e = create_execute(
            "
            mov r2 1
            mov r1 0
            add r1 2
            add r1 10
            add r2 4
            exit",
        );
        assert_eq!(reg_value(&e, 1), 12);
        assert_eq!(reg_value(&e, 2), 5);
    }

    #[test]
    fn test_branch_jmp_sne() {
        let e = create_execute(
            "
            mov r1 0
            add r2 0
        loop:
            sne r1 4
            jmp exit
            add r1 1
            add r2 4
            jmp loop
        exit:
            exit",
        );
        assert_eq!(reg_value(&e, 1), 4);
        assert_eq!(reg_value(&e, 2), 16);
    }

    #[test]
    fn test_call_ret() {
        let e = create_execute(
            "
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
            ret",
        );
        assert_eq!(reg_value(&e, 1), 16);
    }

    #[test]
    fn test_default_sprites() {
        // address registry should point at default sprite "1"
        let e = create_execute(
            "
        main:
            mov r1 1
            ldf r1
            ",
        );

        let bytes = e.copy_bytes(e.address_register, 5);
        assert_eq!(bytes[0], 0x20, "byte {} is invalid", 0);
        assert_eq!(bytes[1], 0x60, "byte {} is invalid", 1);
        assert_eq!(bytes[2], 0x20, "byte {} is invalid", 2);
        assert_eq!(bytes[3], 0x20, "byte {} is invalid", 3);
        assert_eq!(bytes[4], 0x70, "byte {} is invalid", 4);
    }

    #[test]
    fn test_draw_simple() {
        // point address registry to 1 and render it on (0, 0)
        let e = create_execute(
            "
        main:
            mov r1 1
            ldf r1
            mov r1 0
            mov r2 0
            draw r1 r2 5
            ",
        );

        assert_eq!(e.graphics_buffer[0], 0x20, "byte {} is invalid", 0);
        assert_eq!(e.graphics_buffer[8], 0x60, "byte {} is invalid", 8);
        assert_eq!(e.graphics_buffer[16], 0x20, "byte {} is invalid", 16);
        assert_eq!(e.graphics_buffer[24], 0x20, "byte {} is invalid", 24);
        assert_eq!(e.graphics_buffer[32], 0x70, "byte {} is invalid", 32);
        for i in 0..GRAPHICS_BUFFER_SIZE {
            match i {
                0 | 8 | 16 | 24 | 32 => continue,
                x => assert_eq!(
                    e.graphics_buffer[i], 0x00,
                    "byte {} is invalid (0x{:02x})",
                    x, e.graphics_buffer[i]
                ),
            }
        }
    }

    #[test]
    fn test_draw_wrapping() {
        // point address registry to 1 and render it on (0, 0)
        let e = create_execute(
            "
        main:
            mov r1 1
            ldf r1
            mov r1 60
            mov r2 0
            draw r1 r2 5
            ",
        );

        assert_eq!(e.graphics_buffer[7], 0x02, "byte {} is invalid", 7);
        assert_eq!(e.graphics_buffer[8], 0x00, "byte {} is invalid", 8);
        assert_eq!(e.graphics_buffer[15], 0x06, "byte {} is invalid", 15);
        assert_eq!(e.graphics_buffer[16], 0x00, "byte {} is invalid", 16);
        assert_eq!(e.graphics_buffer[23], 0x02, "byte {} is invalid", 23);
        assert_eq!(e.graphics_buffer[24], 0x00, "byte {} is invalid", 24);
        assert_eq!(e.graphics_buffer[31], 0x02, "byte {} is invalid", 31);
        assert_eq!(e.graphics_buffer[32], 0x00, "byte {} is invalid", 32);
        assert_eq!(e.graphics_buffer[39], 0x07, "byte {} is invalid", 39);
        assert_eq!(e.graphics_buffer[40], 0x00, "byte {} is invalid", 40);
        for i in 0..GRAPHICS_BUFFER_SIZE {
            match i {
                7 | 8 | 15 | 16 | 23 | 24 | 31 | 32 | 39 | 40 => continue,
                x => assert_eq!(
                    e.graphics_buffer[i], 0x00,
                    "byte {} is invalid (0x{:02x})",
                    x, e.graphics_buffer[i]
                ),
            }
        }
    }

    #[test]
    fn test_draw_no_collision() {
        // Render 1 twice on the same location, should give a collision
        let e = create_execute(
            "
        main:
            mov r1 1
            ldf r1
            mov r1 0
            mov r2 0
            draw r1 r2 5
            ",
        );
        assert_eq!(e.registries[0x0F_usize], 0);
    }

    #[test]
    fn test_draw_collision() {
        // Render 1 twice on the same location, should give a collision
        let e = create_execute(
            "
        main:
            mov r1 1
            ldf r1
            mov r1 0
            mov r2 0
            draw r1 r2 5
            mov r1 1
            ldf r1
            mov r1 0
            mov r2 0
            draw r1 r2 5
            ",
        );
        assert_eq!(e.registries[0x0F_usize], 1);
    }
}
