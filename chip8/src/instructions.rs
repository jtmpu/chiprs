//!
//! Chip-8 instructions
//!

/// Represents a 4 bit value
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Nibble {
    value: u8,
}

impl Nibble {
    /// Creates two nibbles from a byte, return as [big, little] 
    pub fn new(value: u8) -> [Self; 2] {
        let big = Self { value: (value & 0xF0) >> 4 };
        let little = Self { value: value & 0x0F };
        [big, little]
    }

    pub fn value(&self) -> u8 {
        self.value
    }
}

impl PartialEq<u8> for Nibble {
    fn eq(&self, other: &u8) -> bool {
        self.value == *other
    }
}

pub enum Instruction {
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nibble() {
        let input = 0xAB;
        let [first, second] = Nibble::new(input);
        assert_eq!(first, 0xA);
        assert_eq!(second, 0xB);
        assert!(first != second);
        assert!(first != 0xAB);
        assert!(first != 0xA0);
    }
}
