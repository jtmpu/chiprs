//!
//! Chip-8 instructions
//!
use strum_macros::EnumIter;

/// Represents a 4 bit value
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[allow(non_camel_case_types)]
pub struct u4 {
    value: u8,
}

impl u4 {
    /// Creates two u4 from a byte, return as [big, little]
    pub fn decompose(value: u8) -> [Self; 2] {
        let big = Self {
            value: (value & 0xF0) >> 4,
        };
        let little = Self {
            value: value & 0x0F,
        };
        [big, little]
    }

    /// Create a u4 from the big bits
    pub fn big(value: u8) -> Self {
        Self {
            value: (value & 0xF0) >> 4,
        }
    }

    // Create a u4 from the little bits
    pub fn little(value: u8) -> Self {
        Self {
            value: value & 0x0F,
        }
    }

    pub fn value(&self) -> u8 {
        self.value
    }
}

impl PartialEq<u8> for u4 {
    fn eq(&self, other: &u8) -> bool {
        self.value == *other
    }
}

impl From<u8> for u4 {
    fn from(item: u8) -> Self {
        Self::little(item)
    }
}

/// Represents a 12 bit value
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[allow(non_camel_case_types)]
pub struct u12 {
    value: u16,
}

impl u12 {
    /// creates a u4 of the upper bits, and a u12 of the lower
    pub fn decompose(input: u16) -> (u4, u12) {
        let value = input & 0x0FFF;
        let nibble = (input & 0xF000) >> 12;
        (u4::little(nibble as u8), Self { value })
    }

    pub fn from_u16(input: u16) -> u12 {
        let (_, value) = u12::decompose(input);
        value
    }

    /// extracts the lower bits from the two bytes
    pub fn from_bytes(upper: u8, lower: u8) -> u12 {
        let value = (((upper as u16) & 0xF) << 8) | (lower as u16);
        Self { value }
    }

    pub fn value(&self) -> u16 {
        self.value
    }
}

impl PartialEq<u16> for u12 {
    fn eq(&self, other: &u16) -> bool {
        self.value == *other
    }
}

impl From<u16> for u12 {
    fn from(item: u16) -> Self {
        Self::from_u16(item)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, EnumIter)]
pub enum Instruction {
    /// f1ee - Custom code - make emulator exit
    Exit,
    /// fxef - Custom code - debug-log some data (determined by value of x)
    Debug(u4),
    /// fxff - Custom code - breakpoint - pauses the execution
    Breakpoint,
    /// 00e0
    Clear,
    /// 00EE - return from subroutine
    Return,
    /// 1nnn - Jump to addr nnn
    Jump(u12),
    /// 2nnn - Call subroutine at nnn
    Call(u12),
    /// 3xkk - Skip next instruction if Vx == kk
    SkipEqual(u4, u8),
    /// 4xkk - Skip next instruction if Vx != kk
    SkipNotEqual(u4, u8),
    /// 5xy0 - Skip next instruction if Vx == Vy
    SkipRegistersEqual(u4, u4),
    /// 6xkk - Set Vx = kk
    SetRegisterByte(u4, u8),
    /// 7xkk - Set Vx = Vx + kk
    Add(u4, u8),
    /// 8xy0 - Set Vx = Vy
    SetRegisterRegister(u4, u4),
    /// 8xy1 - Set Vx = Vx OR Vy
    Or(u4, u4),
    /// 8xy2 - Set Vx = Vx AND Vy
    And(u4, u4),
    /// 8xy3 - Set Vx = Vx XOR Vy
    Xor(u4, u4),
    /// Dxyn - Draw n-byte sprite starting at mem I at (Vx, Vy), set VF = collision
    Draw(u4, u4, u4),
    /// Ex9E - Skip next instruction if key with value of Vx is pressed
    SkipKeyPressed(u4),
    /// ExA1 - Skip next instruction if key with value of Vx is not pressed
    SkipKeyNotPressed(u4),
    /// Fx0A - Wait for a key press, store pressed key in Vx
    WaitForKey(u4),
    /// Fx29 - Set I = location of default sprite for digit Vx
    SetMemRegisterDefaultSprit(u4),
    /// Fx07 - Set Vx = delay timer
    SetRegisterDelayTimer(u4),
    /// Fx15 - Set delay timer = Vx
    SetDelayTimer(u4),
    /// Fx18 - Set sound time = Vx
    SetSoundTimer(u4),
}

impl Instruction {
    /// Deconstructs the opcode into an instruction if possible
    pub fn from_opcode_u16(opcode: u16) -> Option<Instruction> {
        let [upper, lower] = opcode.to_be_bytes();
        Self::from_opcode_u8(upper, lower)
    }

    /// Deconstructs the opcode into an instruction if possible
    pub fn from_opcode_u8(upper: u8, lower: u8) -> Option<Instruction> {
        match (upper & 0xF0, upper & 0x0F, lower & 0xF0, lower & 0x0F) {
            (0x00, 0x00, 0xe0, 0x00) => Some(Self::Clear),
            (0x00, 0x00, 0xe0, 0x0e) => Some(Self::Return),
            (0x10, _, _, _) => {
                let address = u12::from_bytes(upper, lower);
                Some(Self::Jump(address))
            }
            (0x20, _, _, _) => {
                let address = u12::from_bytes(upper, lower);
                Some(Self::Call(address))
            }
            (0x30, _, _, _) => {
                let register = u4::little(upper);
                let value = lower;
                Some(Self::SkipEqual(register, value))
            }
            (0x40, _, _, _) => {
                let register = u4::little(upper);
                let value = lower;
                Some(Self::SkipNotEqual(register, value))
            }
            (0x50, _, _, 0x00) => {
                let regx = u4::little(upper);
                let regy = u4::big(lower);
                Some(Self::SkipRegistersEqual(regx, regy))
            }
            (0x60, _, _, _) => {
                let register = u4::little(upper);
                let value = lower;
                Some(Self::SetRegisterByte(register, value))
            }
            (0x70, _, _, _) => {
                let register = u4::little(upper);
                let value = lower;
                Some(Self::Add(register, value))
            }
            (0x80, regx, regy, 0x01) => Some(Self::Or(regx.into(), (regy >> 4).into())),
            (0x80, regx, regy, 0x02) => Some(Self::And(regx.into(), (regy >> 4).into())),
            (0x80, regx, regy, 0x03) => Some(Self::Xor(regx.into(), (regy >> 4).into())),
            (0xD0, regx, regy, n) => Some(Self::Draw(regx.into(), (regy >> 4).into(), n.into())),
            (0xE0, regx, 0x90, 0x0E) => Some(Self::SkipKeyPressed(regx.into())),
            (0xE0, regx, 0xA0, 0x01) => Some(Self::SkipKeyNotPressed(regx.into())),
            (0xF0, regx, 0x00, 0x0A) => Some(Self::WaitForKey(regx.into())),
            (0x80, regx, regy, 0x00) => {
                Some(Self::SetRegisterRegister(regx.into(), (regy >> 4).into()))
            }
            (0xF0, regx, 0x20, 0x09) => Some(Self::SetMemRegisterDefaultSprit(regx.into())),
            (0xF0, regx, 0x00, 0x07) => Some(Self::SetRegisterDelayTimer(regx.into())),
            (0xF0, regx, 0x10, 0x05) => Some(Self::SetDelayTimer(regx.into())),
            (0xF0, regx, 0x10, 0x08) => Some(Self::SetSoundTimer(regx.into())),
            (0xF0, 0x01, 0xE0, 0x0E) => Some(Self::Exit),
            (0xF0, val, 0xE0, 0x0F) => Some(Self::Debug(val.into())),
            (0xF0, _, 0xF0, 0x0F) => Some(Self::Breakpoint),
            (_, _, _, _) => None,
        }
    }

    pub fn opcode(&self) -> u16 {
        match self {
            Self::Exit => 0xf1ee,
            Self::Debug(val) => {
                let big: u16 = 0xF0 | (val.value() as u16);
                let small: u16 = 0xEF;
                (big << 8) | small
            }
            Self::Breakpoint => 0xF0FF,
            Self::Clear => 0x00e0,
            Self::Return => 0x00ee,
            Self::Jump(addr) => 0x1000 | addr.value(),
            Self::Call(addr) => 0x2000 | addr.value(),
            Self::SkipEqual(reg, value) => {
                let big: u16 = 0x30 | (reg.value() as u16);
                let small: u16 = *value as u16;
                (big << 8) | small
            }
            Self::SkipNotEqual(reg, value) => {
                let big: u16 = 0x40 | (reg.value() as u16);
                let small: u16 = *value as u16;
                (big << 8) | small
            }
            Self::SkipRegistersEqual(regx, regy) => {
                let big: u16 = 0x50 | (regx.value() as u16);
                let small: u16 = (regy.value() as u16) << 4 | 0x00_u16;
                (big << 8) | small
            }
            Self::SetRegisterByte(reg, value) => {
                let big: u16 = 0x60 | (reg.value() as u16);
                let small: u16 = *value as u16;
                (big << 8) | small
            }
            Self::Add(reg, value) => {
                let big: u16 = 0x70 | (reg.value() as u16);
                let small: u16 = *value as u16;
                (big << 8) | small
            }
            Self::Or(regx, regy) => {
                let big: u16 = 0x80 | (regx.value() as u16);
                let small: u16 = (regy.value() as u16) << 4 | 0x01_u16;
                (big << 8) | small
            }
            Self::And(regx, regy) => {
                let big: u16 = 0x80 | (regx.value() as u16);
                let small: u16 = (regy.value() as u16) << 4 | 0x02_u16;
                (big << 8) | small
            }
            Self::Xor(regx, regy) => {
                let big: u16 = 0x80 | (regx.value() as u16);
                let small: u16 = (regy.value() as u16) << 4 | 0x03_u16;
                (big << 8) | small
            }
            Self::Draw(regx, regy, n) => {
                let big: u16 = 0xd0 | (regx.value() as u16);
                let small: u16 = (regy.value() as u16) << 4 | (n.value() as u16);
                (big << 8) | small
            }
            Self::SkipKeyPressed(regx) => {
                let big: u16 = 0xE0 | (regx.value() as u16);
                let small: u16 = 0x9E;
                (big << 8) | small
            }
            Self::SkipKeyNotPressed(regx) => {
                let big: u16 = 0xE0 | (regx.value() as u16);
                let small: u16 = 0xA1;
                (big << 8) | small
            }
            Self::WaitForKey(regx) => {
                let big: u16 = 0xF0 | (regx.value() as u16);
                let small: u16 = 0x0A;
                (big << 8) | small
            }
            Self::SetRegisterRegister(regx, regy) => {
                let big: u16 = 0x80 | (regx.value() as u16);
                let small: u16 = (regy.value() as u16) << 4;
                (big << 8) | small
            }
            Self::SetMemRegisterDefaultSprit(regx) => {
                let big: u16 = 0xF0 | (regx.value() as u16);
                let small: u16 = 0x29;
                (big << 8) | small
            }
            Self::SetRegisterDelayTimer(regx) => {
                let big: u16 = 0xF0 | (regx.value() as u16);
                let small: u16 = 0x07;
                (big << 8) | small
            }
            Self::SetDelayTimer(regx) => {
                let big: u16 = 0xF0 | (regx.value() as u16);
                let small: u16 = 0x15;
                (big << 8) | small
            }
            Self::SetSoundTimer(regx) => {
                let big: u16 = 0xF0 | (regx.value() as u16);
                let small: u16 = 0x18;
                (big << 8) | small
            }
        }
    }

    pub fn to_assembly(&self) -> String {
        match self {
            Self::Exit => "exit".to_string(),
            Self::Debug(val) => format!("debug {}", val.value()),
            Self::Breakpoint => "break".to_string(),
            Self::Clear => "clear".to_string(),
            Self::Return => "ret".to_string(),
            Self::Jump(addr) => format!("jmp {}", addr.value()),
            Self::Call(addr) => format!("call {}", addr.value()),
            Self::SkipEqual(reg, value) => format!("se r{} {}", reg.value(), value),
            Self::SkipNotEqual(reg, value) => format!("sne r{} {}", reg.value(), value),
            Self::SkipRegistersEqual(regx, regy) => {
                format!("sre r{} r{}", regx.value(), regy.value())
            }
            Self::SetRegisterByte(reg, value) => format!("ldb r{} {}", reg.value(), value),
            Self::SetRegisterRegister(regx, regy) => {
                format!("ldr r{} r{}", regx.value(), regy.value())
            }
            Self::Add(reg, value) => format!("add r{} {}", reg.value(), value),
            Self::Or(regx, regy) => format!("or r{} r{}", regx.value(), regy.value()),
            Self::And(regx, regy) => format!("and r{} r{}", regx.value(), regy.value()),
            Self::Xor(regx, regy) => format!("xor r{} r{}", regx.value(), regy.value()),
            Self::Draw(regx, regy, n) => {
                format!("draw r{} r{} {}", regx.value(), regy.value(), n.value())
            }
            Self::SkipKeyPressed(reg) => format!("skp r{}", reg.value()),
            Self::SkipKeyNotPressed(reg) => format!("sknp r{}", reg.value()),
            Self::WaitForKey(reg) => format!("input r{}", reg.value()),
            Self::SetMemRegisterDefaultSprit(reg) => format!("ldf {}", reg.value()),
            Self::SetRegisterDelayTimer(reg) => format!("ldd r{}", reg.value()),
            Self::SetDelayTimer(reg) => format!("delay r{}", reg.value()),
            Self::SetSoundTimer(reg) => format!("sound r{}", reg.value()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u4() {
        let input = 0xAB;
        let [first, second] = u4::decompose(input);
        assert_eq!(first, 0xA);
        assert_eq!(second, 0xB);
        assert!(first != second);
        assert!(first != 0xAB);
        assert!(first != 0xA0);

        let value = u4::big(0xEA);
        assert_eq!(value, 0xE);

        let value = u4::little(0xEA);
        assert_eq!(value, 0xA);
    }

    #[test]
    fn test_u12() {
        let input = 0xABCD;
        let (first, second) = u12::decompose(input);
        assert_eq!(first, 0xA);
        assert_eq!(second, 0xBCD);

        let fourth = u12::from_bytes(0xEF, 0xDB);
        assert_eq!(fourth, 0xFDB);
    }

    #[test]
    fn test_instruction_from_opcode() {
        let cases: Vec<(u16, Instruction)> = vec![
            (0xF1EE, Instruction::Exit),
            (0xF3EF, Instruction::Debug(0x03.into())),
            (0xF0FF, Instruction::Breakpoint),
            (0x00E0, Instruction::Clear),
            (0x00EE, Instruction::Return),
            (0x1BFD, Instruction::Jump(u12::from_u16(0xBFD))),
            (0x2ABC, Instruction::Call(u12::from_u16(0xABC))),
            (0x61FF, Instruction::SetRegisterByte(u4::little(0x01), 0xFF)),
            (
                0x8130,
                Instruction::SetRegisterRegister(0x01.into(), 0x03.into()),
            ),
            (0x7812, Instruction::Add(u4::little(0x08), 0x12)),
            (0x32FF, Instruction::SkipEqual(u4::little(0x02), 0xFF)),
            (0x42EC, Instruction::SkipNotEqual(u4::little(0x02), 0xEC)),
            (
                0x5280,
                Instruction::SkipRegistersEqual(0x02.into(), 0x08.into()),
            ),
            (0x8121, Instruction::Or(0x01.into(), 0x02.into())),
            (0x8122, Instruction::And(0x01.into(), 0x02.into())),
            (0x8123, Instruction::Xor(0x01.into(), 0x02.into())),
            (
                0xD265,
                Instruction::Draw(0x02.into(), 0x06.into(), 0x05.into()),
            ),
            (0xE29E, Instruction::SkipKeyPressed(0x02.into())),
            (0xE5A1, Instruction::SkipKeyNotPressed(0x05.into())),
            (0xF70A, Instruction::WaitForKey(0x07.into())),
            (0xFA29, Instruction::SetMemRegisterDefaultSprit(0x0A.into())),
            (0xF107, Instruction::SetRegisterDelayTimer(0x01.into())),
            (0xF915, Instruction::SetDelayTimer(0x09.into())),
            (0xF918, Instruction::SetSoundTimer(0x09.into())),
        ];

        for case in cases {
            let value = Instruction::from_opcode_u16(case.0).unwrap();
            assert_eq!(value, case.1);
            let [upper, lower] = case.0.to_be_bytes();
            let value = Instruction::from_opcode_u8(upper, lower).unwrap();
            assert_eq!(value, case.1);
        }
    }

    #[test]
    fn test_instruction_to_opcode() {
        let cases: Vec<(Instruction, u16)> = vec![
            (Instruction::Exit, 0xF1EE),
            (Instruction::Debug(0x03.into()), 0xF3EF),
            (Instruction::Breakpoint, 0xF0FF),
            (Instruction::Clear, 0x00E0),
            (Instruction::Return, 0x00EE),
            (Instruction::Jump(0x123.into()), 0x1123),
            (Instruction::Call(0x321.into()), 0x2321),
            (Instruction::SetRegisterByte(0x02.into(), 0x42), 0x6242),
            (
                Instruction::SetRegisterRegister(0x03.into(), 0x04.into()),
                0x8340,
            ),
            (Instruction::Add(0x04.into(), 0x2), 0x7402),
            (Instruction::SkipEqual(0x03.into(), 8), 0x3308),
            (Instruction::SkipNotEqual(0x05.into(), 4), 0x4504),
            (
                Instruction::SkipRegistersEqual(0x01.into(), 0x07.into()),
                0x5170,
            ),
            (Instruction::Or(0x02.into(), 0x03.into()), 0x8231),
            (Instruction::And(0x02.into(), 0x03.into()), 0x8232),
            (Instruction::Xor(0x02.into(), 0x03.into()), 0x8233),
            (
                Instruction::Draw(0x04.into(), 0x05.into(), 0x0F.into()),
                0xD45F,
            ),
            (Instruction::SkipKeyPressed(0x06.into()), 0xE69E),
            (Instruction::SkipKeyNotPressed(0x05.into()), 0xE5A1),
            (Instruction::WaitForKey(0x03.into()), 0xF30A),
            (Instruction::SetMemRegisterDefaultSprit(0x02.into()), 0xF229),
            (Instruction::SetRegisterDelayTimer(0x07.into()), 0xF707),
            (Instruction::SetDelayTimer(0x02.into()), 0xF215),
            (Instruction::SetSoundTimer(0x02.into()), 0xF218),
        ];
        for case in cases {
            let opcode = case.0.opcode();
            assert_eq!(
                opcode, case.1,
                "[{:?}]: expected '0x{:04x}', received '0x{:04x}'",
                case.0, case.1, opcode
            );
        }
    }
}
