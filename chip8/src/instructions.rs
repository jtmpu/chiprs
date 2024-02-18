//!
//! Chip-8 instructions
//!

/// Represents a 4 bit value
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(non_camel_case_types)]
pub struct u4 {
    value: u8,
}

impl u4 {
    /// Creates two u4 from a byte, return as [big, little] 
    pub fn decompose(value: u8) -> [Self; 2] {
        let big = Self { value: (value & 0xF0) >> 4 };
        let little = Self { value: value & 0x0F };
        [big, little]
    }

    /// Create a u4 from the big bits
    pub fn big(value: u8) -> Self {
        Self { value: (value & 0xF0) >> 4 }
    }

    // Create a u4 from the little bits
    pub fn little(value: u8) -> Self {
        Self { value: value & 0x0F }
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
#[derive(Debug, Clone, Copy, PartialEq)]
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Instruction {
    /// 00e0
    Clear,
    /// 1nnn - Jump to addr nnn
    Jump(u12),
    /// 4xkk - Skip next instruction if Vx != kk
    SkipNotEqual(u4, u8),
    /// 6xkk - Set Vx = kk
    Move(u4, u8),
    /// 7xkk - Set Vx = Vx + kk
    Add(u4, u8),
}

impl Instruction {
    /// Deconstructs the opcode into an instruction if possible
    pub fn from_opcode_u16(opcode: u16) -> Option<Instruction> {
        let [upper, lower] = opcode.to_be_bytes();
        Self::from_opcode_u8(upper, lower)
    }

    /// Deconstructs the opcode into an instruction if possible
    pub fn from_opcode_u8(upper: u8, lower: u8) -> Option<Instruction> {
        match (upper & 0xF0, lower) {
            (0x00, 0xe0) => {
                Some(Self::Clear)
            },
            (0x10, _) => {
                let address = u12::from_bytes(upper, lower);
                Some(Self::Jump(address))
            },
            (0x40, _) => {
                let register = u4::little(upper);
                let value = lower;
                Some(Self::SkipNotEqual(register, value))
            },
            (0x60, _) => {
                let register = u4::little(upper);
                let value = lower;
                Some(Self::Move(register, value))
            },
            (0x70, _) => {
                let register = u4::little(upper);
                let value = lower;
                Some(Self::Add(register, value))
            },
            (_, _) => None,
        }
    }

    pub fn opcode(&self) -> u16 {
        match self {
            Self::Clear => 0x00e0,
            Self::Jump(addr) => 0x1000 | addr.value(),
            Self::SkipNotEqual(reg, value) => {
                let big: u16 = 0x40 | (reg.value() as u16);
                let small: u16 = *value as u16;
                return (big << 8) | small;
            },
            Self::Move(reg, value) => {
                let big: u16 = 0x60 | (reg.value() as u16);
                let small: u16 = *value as u16;
                return (big << 8) | small;
            },
            Self::Add(reg, value) => {
                let big: u16 = 0x70 | (reg.value() as u16);
                let small: u16 = *value as u16;
                return (big << 8) | small;
            },
        }
    }

    pub fn to_assembly(&self) -> String {
        match self {
            Self::Clear => format!("clear"),
            Self::Jump(addr) => format!("jmp {}", addr.value()),
            Self::SkipNotEqual(reg, value) => format!("sne r{} {}", reg.value(), value),
            Self::Move(reg, value) => format!("mov r{} {}", reg.value(), value),
            Self::Add(reg, value) => format!("add r{} {}", reg.value(), value),
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
            (0x1BFD, Instruction::Jump(u12::from_u16(0xBFD))),
            (0x61FF, Instruction::Move(u4::little(0x01), 0xFF)),
            (0x7812, Instruction::Add(u4::little(0x08), 0x12)),
            (0x42EC, Instruction::SkipNotEqual(u4::little(0x02), 0xEC)),
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
            (Instruction::Clear, 0x00e0),
            (Instruction::Jump(0x123.into()), 0x1123),
            (Instruction::Move(0x02.into(), 0x42), 0x6242),
            (Instruction::Add(0x04.into(), 0x2), 0x7402),
            (Instruction::SkipNotEqual(0x05.into(), 4), 0x4504),
        ];
        for case in cases {
            let opcode = case.0.opcode();
            assert_eq!(opcode, case.1, "[{:?}]: expected '0x{:04x}', received '0x{:04x}'", case.0, case.1, opcode);
        }
    }
}
