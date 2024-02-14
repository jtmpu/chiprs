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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Instruction {
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
}
