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

pub enum Instruction {
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
}
