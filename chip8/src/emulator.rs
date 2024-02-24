//!
//! Chip-8 emulator
//!
use std::{
    error::Error,
    fmt,
    fs::File,
    io::{self, Read},
    sync::mpsc::{Receiver, Sender},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use tracing::{debug, error, info, span, Level};

use crate::instructions::{u4, Instruction};

#[derive(Debug)]
pub enum Chip8Error {
    UnimplementedInstruction,
    InvalidOpcode(String),
    StackEmpty,
    StackFull,
    IO(io::Error),
}

impl fmt::Display for Chip8Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "chip-8 failure: {:?}", self)
    }
}
impl From<io::Error> for Chip8Error {
    fn from(err: io::Error) -> Self {
        Chip8Error::IO(err)
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
pub const KEY_COUNT: usize = 16;

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

// 60 hz at microsecond scale
const TIME_BETWEEN_DECREMENT: u128 = Duration::from_micros(1_000_000 / 60).as_micros();

// Control/read messages supported by the
// chip-8 emulator
pub enum Message {
    Pause,
    SendGraphics(Sender<[u8; GRAPHICS_BUFFER_SIZE]>),
    KeyEvent(u4, KeyStatus),
}

pub struct Builder {
    hertz: usize,
    timeboxes: usize,
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder {
    pub fn new() -> Self {
        Self {
            hertz: 400,
            timeboxes: 100,
        }
    }

    pub fn with_hertz(mut self, hertz: usize) -> Self {
        self.hertz = hertz;
        self
    }

    pub fn with_timeboxes(mut self, timeboxes: usize) -> Self {
        self.timeboxes = timeboxes;
        self
    }

    pub fn load_program(self, filepath: &str) -> Result<Emulator, Chip8Error> {
        let mut emulator = Emulator::new(self.hertz, self.timeboxes);
        emulator.reset();
        let file = File::open(filepath)?;
        emulator.load(file)?;
        Ok(emulator)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum KeyStatus {
    #[default]
    Up,
    Pressed,
}

#[derive(Debug, Default, Clone)]
pub struct Snapshot {
    pub registries: [u8; REGISTRY_COUNT],
    pub program_counter: usize,
    pub stack_pointer: usize,
    pub address_register: usize,
    pub delay_timer: u8,
    pub stack: [usize; STACK_SIZE],
    pub key_status: [KeyStatus; KEY_COUNT],
    pub instruction: Option<Instruction>,
}

pub struct Emulator {
    // hardware
    memory: [u8; MEMSIZE],
    registries: [u8; REGISTRY_COUNT],
    program_counter: usize,
    stack_pointer: usize,
    // I registry
    address_register: usize,
    delay_timer: u8,
    sound_timer: u8,
    stack: [usize; STACK_SIZE],
    graphics_buffer: [u8; GRAPHICS_BUFFER_SIZE],
    last_delay_decrement: Option<Instant>,
    last_sound_decrement: Option<Instant>,
    key_status: [KeyStatus; KEY_COUNT],
    wait_for_key: Option<u8>,

    // configurations
    hertz: usize,
    timeboxes: usize,

    // thread communication
    receiver: Option<Receiver<Message>>,
}

impl Emulator {
    fn new(hertz: usize, timeboxes: usize) -> Self {
        let mut ret = Self {
            memory: [0; MEMSIZE],
            registries: [0; REGISTRY_COUNT],
            program_counter: START_ADDR,
            stack_pointer: 0,
            address_register: 0,
            delay_timer: 0,
            sound_timer: 0,
            stack: [0; STACK_SIZE],
            graphics_buffer: [0; GRAPHICS_BUFFER_SIZE],
            last_delay_decrement: None,
            last_sound_decrement: None,
            key_status: [KeyStatus::Up; KEY_COUNT],
            wait_for_key: None,
            hertz,
            timeboxes,
            receiver: None,
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
        self.delay_timer = 0;
        self.sound_timer = 0;
        self.stack = [0; STACK_SIZE];
        self.graphics_buffer = [0; GRAPHICS_BUFFER_SIZE];
        self.last_delay_decrement = None;
        self.last_sound_decrement = None;
        self.key_status = [KeyStatus::Up; KEY_COUNT];
        self.wait_for_key = None;
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

    pub fn load<T: Read>(&mut self, mut reader: T) -> Result<(), Chip8Error> {
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

        if let Some(regx) = self.wait_for_key {
            for (i, key) in self.key_status.iter().enumerate() {
                if *key == KeyStatus::Pressed {
                    self.registries[regx as usize] = i as u8;
                    self.wait_for_key = None;
                    break;
                }
            }
            self.decrement_timers();
            return Ok(true);
        }

        let instruction = match self.instruction() {
            Ok(i) => i,
            Err(e) => {
                debug!(
                    pc = self.program_counter,
                    error = ?e,
                    "failed to parse instruction opcode"
                );
                return Err(e);
            }
        };
        self.program_counter += 2;

        let ret = match self.execute(instruction) {
            Err(e) => {
                debug!(
                    pc = self.program_counter,
                    error = ?e,
                    "failed to execute instruction"
                );
                Err(e)
            }
            res => res,
        }?;

        self.decrement_timers();
        Ok(ret)
    }

    pub fn execute(&mut self, instruction: Instruction) -> Result<bool, Box<dyn Error>> {
        debug!(instruction = ?instruction, "executing instruction");
        match instruction {
            Instruction::Exit => {
                // Kill execution
                return Ok(false);
            }
            Instruction::Clear => {
                self.graphics_buffer = [0; GRAPHICS_BUFFER_SIZE];
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
            Instruction::SetRegisterByte(register, value) => {
                self.registries[register.value() as usize] = value;
            }
            Instruction::SetRegisterRegister(regx, regy) => {
                self.registries[regx.value() as usize] = self.registries[regy.value() as usize];
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
            Instruction::SkipKeyPressed(regx) => {
                if self.key_status[regx.value() as usize] == KeyStatus::Pressed {
                    self.program_counter += 2;
                }
            }
            Instruction::SkipKeyNotPressed(regx) => {
                if self.key_status[regx.value() as usize] == KeyStatus::Up {
                    self.program_counter += 2;
                }
            }
            Instruction::WaitForKey(regx) => {
                self.wait_for_key = Some(regx.value());
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
            Instruction::SetRegisterDelayTimer(regx) => {
                self.registries[regx.value() as usize] = self.delay_timer;
            }
            Instruction::SetDelayTimer(regx) => {
                self.delay_timer = self.registries[regx.value() as usize];
            }
            Instruction::SetSoundTimer(regx) => {
                self.sound_timer = self.registries[regx.value() as usize];
            }
            Instruction::Debug(value) => {
                let msg = match value {
                    x if x == u4::little(1) => {
                        format!("{:?}", self.registries)
                    }
                    _ => return Ok(true),
                };
                info!(source = "debug-instruction", msg);
            }
            Instruction::Breakpoint => return Ok(false),
        };
        Ok(true)
    }

    /// Decrement timers at a rate of 60hz, when a timer reaches
    /// zero this does nothing. The upper bound is 1 decrement per instruction
    /// execution
    fn decrement_timers(&mut self) {
        if self.delay_timer > 0 {
            if let Some(last_delay_decrement) = self.last_delay_decrement {
                if last_delay_decrement.elapsed().as_micros() > TIME_BETWEEN_DECREMENT {
                    self.delay_timer -= 1;
                    self.last_delay_decrement = Some(Instant::now());
                }

                if self.delay_timer == 0 {
                    // Reached zero now, remove last delay decrement
                    self.last_delay_decrement = None;
                }
            } else {
                // Just started a delay timer, start keeping track of time
                self.last_delay_decrement = Some(Instant::now());
            }
        }
        if self.sound_timer > 0 {
            if let Some(last_sound_decrement) = self.last_sound_decrement {
                if last_sound_decrement.elapsed().as_micros() > TIME_BETWEEN_DECREMENT {
                    self.sound_timer -= 1;
                    self.last_sound_decrement = Some(Instant::now());
                }

                if self.sound_timer == 0 {
                    self.last_sound_decrement = None;
                }
            } else {
                self.last_sound_decrement = Some(Instant::now());
            }
        }
    }

    pub fn copy_graphics_buffer(&self) -> [u8; GRAPHICS_BUFFER_SIZE] {
        self.graphics_buffer
    }

    pub fn create_snapshot(&self) -> Snapshot {
        Snapshot {
            registries: self.registries.clone(),
            program_counter: self.program_counter.clone(),
            address_register: self.address_register.clone(),
            delay_timer: self.delay_timer.clone(),
            stack: self.stack.clone(),
            stack_pointer: self.stack_pointer.clone(),
            key_status: self.key_status.clone(),
            instruction: match self.instruction() {
                Ok(i) => Some(i),
                Err(_) => None,
            },
        }
    }

    pub fn set_key(&mut self, key: u4, status: KeyStatus) {
        info!(?key, ?status, "key event");
        self.key_status[key.value() as usize] = status;
    }

    pub fn key_pressed(&mut self, key: u4) {
        self.set_key(key, KeyStatus::Pressed);
    }

    pub fn key_up(&mut self, key: u4) {
        self.set_key(key, KeyStatus::Up);
    }

    /// runs the emulator in a separate thread
    pub fn run(self, receiver: Option<Receiver<Message>>) -> JoinHandle<Emulator> {
        thread::spawn(move || {
            let mut owned = self;
            owned.receiver = receiver;
            owned.threaded_run();
            if let Some(recv) = owned.receiver.take() {
                // Deallocating receiver allows the blocked send to unblock
                drop(recv);
            }
            owned
        })
    }

    // the main loop of the emulator when executing in a thread
    fn threaded_run(&mut self) {
        let delay_per_second = 1_000_000_000;
        let delay_per_timebox = (delay_per_second / self.timeboxes) as u128;
        let ticks_per_timebox = self.hertz / self.timeboxes;

        info!(%ticks_per_timebox, %delay_per_timebox, "starting chip-8 machine");
        let mut ticks = 0;
        let mut last_tick = Instant::now();
        loop {
            if ticks < ticks_per_timebox {
                // keep ticking while we're allowed in the timebox
                // check and handle any message requests if a receiver
                // exists
                if let Some(receiver) = &self.receiver {
                    let should_abort = match receiver.try_recv() {
                        Ok(message) => self.process_message(message),
                        Err(_) => {
                            // error!(%error, "failed polling receiver for message");
                            false
                        }
                    };
                    if should_abort {
                        break;
                    }
                }

                match self.tick() {
                    Ok(true) => {}
                    Ok(false) => {
                        // Breakpoint hit, pause execution
                        break;
                    }
                    Err(error) => {
                        error!(%error, "pausing emulator execution");
                        break;
                    }
                }
                ticks += 1;
            } else {
                if last_tick.elapsed().as_nanos() < delay_per_timebox {
                    // listen for message requests, or if no receiver is configured sleep,
                    // until we can execute more ticks
                    let timeout = delay_per_timebox - last_tick.elapsed().as_nanos();
                    if let Some(receiver) = &self.receiver {
                        let should_abort =
                            match receiver.recv_timeout(Duration::from_nanos(timeout as u64)) {
                                Ok(message) => self.process_message(message),
                                Err(_) => {
                                    // error!(%error, "failed polling receiver for message");
                                    false
                                }
                            };
                        if should_abort {
                            break;
                        }
                    } else {
                        thread::sleep(Duration::from_nanos(timeout as u64));
                    }
                }
                ticks = 0;
                last_tick = Instant::now();
            }
        }
        info!("pausing chip-8 machine");
    }

    fn process_message(&mut self, message: Message) -> bool {
        match message {
            Message::Pause => {
                info!("received pause");
                return true;
            }
            Message::SendGraphics(channel) => {
                match channel.send(self.copy_graphics_buffer()) {
                    Ok(_) => {}
                    Err(_) => {
                        info!("failed to send graphics buffer, terminating");
                        return true;
                    }
                };
            }
            Message::KeyEvent(key, status) => {
                info!(key = ?key, status = ?status, "received key event");
                self.set_key(key, status);
            }
        };
        false
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
        let mut emulator = Emulator::new(400, 100);
        let cursor = Cursor::new(binary);
        emulator.load(cursor).unwrap();
        loop {
            match emulator.tick() {
                Ok(false) => break,
                Err(_) => break,
                _ => {}
            }
        }
        emulator
    }

    fn reg_value(emu: &Emulator, index: usize) -> u8 {
        emu.registries[index]
    }

    #[test]
    fn test_add() {
        let e = create_execute(
            "
            ldb r2 1
            ldb r1 0
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
            ldb r1 0
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
            ldb r1 0
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
            ldb r1 1
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
            ldb r1 1
            ldf r1
            ldb r1 0
            ldb r2 0
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
            ldb r1 1
            ldf r1
            ldb r1 60
            ldb r2 0
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
            ldb r1 1
            ldf r1
            ldb r1 0
            ldb r2 0
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
            ldb r1 1
            ldf r1
            ldb r1 0
            ldb r2 0
            draw r1 r2 5
            ldb r1 1
            ldf r1
            ldb r1 0
            ldb r2 0
            draw r1 r2 5
            ",
        );
        assert_eq!(e.registries[0x0F_usize], 1);
    }

    #[test]
    fn test_delay_timer_start() {
        let input = "
        main:
            ldb r1 3
            ldb r3 0
            delay r1
            ldb r4 0
            ldb r4 0
            ldb r4 0
            ldb r4 0
            ";
        let reader = BufReader::new(input.as_bytes());
        let lexer = StreamLexer::new(reader);
        let mut parser = Parser::new(Box::new(lexer));
        let binary = parser.parse().unwrap().binary().unwrap();
        let mut emulator = Emulator::new(400, 100);
        let cursor = Cursor::new(binary);
        emulator.load(cursor).unwrap();

        // 3 ticks to start delay timer
        emulator.tick().unwrap();
        emulator.tick().unwrap();
        emulator.tick().unwrap();

        assert!(emulator.delay_timer > 0);
    }

    #[test]
    fn test_delay_timer_tick() {
        let input = "
        main:
            ldb r1 3
            ldb r3 0
            delay r1
            ldb r4 0
            ldb r4 0
            ldb r4 0
            ldb r4 0
            ";
        let reader = BufReader::new(input.as_bytes());
        let lexer = StreamLexer::new(reader);
        let mut parser = Parser::new(Box::new(lexer));
        let binary = parser.parse().unwrap().binary().unwrap();
        let mut emulator = Emulator::new(400, 100);
        let cursor = Cursor::new(binary);
        emulator.load(cursor).unwrap();

        // 3 ticks to start delay timer
        emulator.tick().unwrap();
        emulator.tick().unwrap();
        emulator.tick().unwrap();

        // Teleport 2 seconds into the future to force a timer decrement
        // reaction - we're limited to one decrement per tick
        let t = Instant::now().checked_sub(Duration::from_secs(2)).unwrap();
        emulator.last_delay_decrement = Some(t);
        emulator.tick().unwrap();
        emulator.last_delay_decrement = Some(t);
        emulator.tick().unwrap();
        // Two decrements should've occured
        assert_eq!(emulator.delay_timer, 1);
    }

    #[test]
    fn test_sound_timer_start() {
        let input = "
        main:
            ldb r1 3
            ldb r3 0
            sound r1
            ldb r4 0
            ldb r4 0
            ldb r4 0
            ldb r4 0
            ";
        let reader = BufReader::new(input.as_bytes());
        let lexer = StreamLexer::new(reader);
        let mut parser = Parser::new(Box::new(lexer));
        let binary = parser.parse().unwrap().binary().unwrap();
        let mut emulator = Emulator::new(400, 100);
        let cursor = Cursor::new(binary);
        emulator.load(cursor).unwrap();
        //
        // 3 ticks to start delay timer
        emulator.tick().unwrap();
        emulator.tick().unwrap();
        emulator.tick().unwrap();

        assert!(emulator.sound_timer > 0);
    }

    #[test]
    fn test_sound_timer_tick() {
        let input = "
        main:
            ldb r1 3
            ldb r3 0
            sound r1
            ldb r4 0
            ldb r4 0
            ldb r4 0
            ldb r4 0
            ";
        let reader = BufReader::new(input.as_bytes());
        let lexer = StreamLexer::new(reader);
        let mut parser = Parser::new(Box::new(lexer));
        let binary = parser.parse().unwrap().binary().unwrap();
        let mut emulator = Emulator::new(400, 100);
        let cursor = Cursor::new(binary);
        emulator.load(cursor).unwrap();
        //
        // 3 ticks to start delay timer
        emulator.tick().unwrap();
        emulator.tick().unwrap();
        emulator.tick().unwrap();

        // Teleport 2 seconds into the future to force a timer decrement
        // reaction - we're limited to one decrement per tick
        let t = Instant::now().checked_sub(Duration::from_secs(2)).unwrap();
        emulator.last_sound_decrement = Some(t);
        emulator.tick().unwrap();
        emulator.last_sound_decrement = Some(t);
        emulator.tick().unwrap();
        // Two decrements should've occured
        assert_eq!(emulator.sound_timer, 1);
    }

    #[test]
    fn test_skip_key_press() {
        let input = "
        main:
            ldb r1 0
            ldb r2 2
            ldb r3 0
            skp r2
            add r1 2
            skp r3
            add r1 4
            ";
        let reader = BufReader::new(input.as_bytes());
        let lexer = StreamLexer::new(reader);
        let mut parser = Parser::new(Box::new(lexer));
        let binary = parser.parse().unwrap().binary().unwrap();
        let mut emulator = Emulator::new(400, 100);
        let cursor = Cursor::new(binary);
        emulator.load(cursor).unwrap();
        emulator.key_pressed(2.into());
        loop {
            match emulator.tick() {
                Ok(false) => break,
                Err(_) => break,
                _ => {}
            }
        }

        assert_eq!(emulator.registries[1], 4);
    }

    #[test]
    fn test_skip_key_not_press() {
        let input = "
        main:
            ldb r1 0
            ldb r2 2
            ldb r3 0
            sknp r2
            add r1 2
            sknp r3
            add r1 4
            ";
        let reader = BufReader::new(input.as_bytes());
        let lexer = StreamLexer::new(reader);
        let mut parser = Parser::new(Box::new(lexer));
        let binary = parser.parse().unwrap().binary().unwrap();
        let mut emulator = Emulator::new(400, 100);
        let cursor = Cursor::new(binary);
        emulator.load(cursor).unwrap();
        emulator.key_pressed(2.into());
        loop {
            match emulator.tick() {
                Ok(false) => break,
                Err(_) => break,
                _ => {}
            }
        }

        assert_eq!(emulator.registries[1], 2);
    }
}
