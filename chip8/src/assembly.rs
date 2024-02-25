pub mod lexer;
pub mod parser;

use std::collections::HashMap;

use crate::emulator::START_ADDR;
use crate::instructions::{u12, Instruction};

#[derive(Debug)]
pub enum BinaryError {
    MissingLabelAddress(String),
}

#[derive(Debug)]
pub struct Assembly {
    pub instructions: Vec<ParsedInstruction>,
    pub labels: HashMap<String, usize>,
}

impl Assembly {
    /// Converts the assembly to binary
    pub fn binary(&self) -> Result<Vec<u8>, BinaryError> {
        let mut buffer = Vec::new();
        for instr in &self.instructions {
            // Check if we need to resolve labels
            let instruction = match instr.instruction {
                Instruction::Call(a) => {
                    if let Some(label) = &instr.label {
                        if let Some(offset) = self.labels.get(label) {
                            let address: u12 = ((START_ADDR + (offset * 2)) as u16).into();
                            Instruction::Call(address)
                        } else {
                            return Err(BinaryError::MissingLabelAddress(label.clone()));
                        }
                    } else {
                        Instruction::Call(a)
                    }
                }
                Instruction::Jump(a) => {
                    if let Some(label) = &instr.label {
                        if let Some(offset) = self.labels.get(label) {
                            let address: u12 = ((START_ADDR + (offset * 2)) as u16).into();
                            Instruction::Jump(address)
                        } else {
                            return Err(BinaryError::MissingLabelAddress(label.clone()));
                        }
                    } else {
                        Instruction::Jump(a)
                    }
                }
                Instruction::SetMemRegister(a) => {
                    if let Some(label) = &instr.label {
                        if let Some(offset) = self.labels.get(label) {
                            let address: u12 = ((START_ADDR + (offset * 2)) as u16).into();
                            Instruction::SetMemRegister(address)
                        } else {
                            return Err(BinaryError::MissingLabelAddress(label.clone()));
                        }
                    } else {
                        Instruction::SetMemRegister(a)
                    }
                }
                Instruction::JumpOffset(a) => {
                    if let Some(label) = &instr.label {
                        if let Some(offset) = self.labels.get(label) {
                            let address: u12 = ((START_ADDR + (offset * 2)) as u16).into();
                            Instruction::JumpOffset(address)
                        } else {
                            return Err(BinaryError::MissingLabelAddress(label.clone()));
                        }
                    } else {
                        Instruction::JumpOffset(a)
                    }
                }
                i => i,
            };
            let opcode = instruction.opcode();
            let bytes = opcode.to_be_bytes();
            buffer.extend(bytes);
        }
        Ok(buffer)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedInstruction {
    pub instruction: Instruction,
    pub label: Option<String>,
    pub source: Option<Source>,
}

impl ParsedInstruction {
    pub fn new(instruction: Instruction) -> Self {
        Self {
            instruction,
            label: None,
            source: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Source {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_addr_resolve() {
        let instructions = vec![
            ParsedInstruction {
                instruction: Instruction::SetRegisterByte(1.into(), 0),
                label: Some("main".to_string()),
                source: None,
            },
            ParsedInstruction {
                instruction: Instruction::SetRegisterByte(2.into(), 4),
                label: None,
                source: None,
            },
            ParsedInstruction {
                instruction: Instruction::Call(0.into()),
                label: Some("loop".to_string()),
                source: None,
            },
            ParsedInstruction {
                instruction: Instruction::SkipNotEqual(2.into(), 3),
                label: None,
                source: None,
            },
            ParsedInstruction {
                instruction: Instruction::Jump(0.into()),
                label: Some("exit".to_string()),
                source: None,
            },
            ParsedInstruction {
                instruction: Instruction::Add(1.into(), 4),
                label: None,
                source: None,
            },
            ParsedInstruction {
                instruction: Instruction::Add(2.into(), 1),
                label: None,
                source: None,
            },
            ParsedInstruction {
                instruction: Instruction::Jump(0.into()),
                label: Some("loop".to_string()),
                source: None,
            },
            ParsedInstruction {
                instruction: Instruction::SetRegisterByte(4.into(), 0),
                label: Some("exit".to_string()),
                source: None,
            },
        ];
        let mut labels = HashMap::new();
        labels.insert("main".to_string(), 0);
        labels.insert("loop".to_string(), 3);
        labels.insert("exit".to_string(), 8);
        let assembly = Assembly {
            instructions,
            labels,
        };
        let binary = assembly.binary().unwrap();
        assert_addr(binary.as_ref(), 2, 0x206.into());
        assert_addr(binary.as_ref(), 4, 0x210.into());
        assert_addr(binary.as_ref(), 7, 0x206.into());
    }

    fn assert_addr(binary: &[u8], location: usize, addr: u12) {
        let b1 = binary[location * 2];
        let b2 = binary[location * 2 + 1];
        let instr = Instruction::from_opcode_u8(b1, b2).unwrap();
        let a = match instr {
            Instruction::Jump(a) => a,
            Instruction::Call(a) => a,
            _ => panic!("invalid opcode"),
        };
        assert_eq!(
            a,
            addr,
            "expected addr '0x{:04x}', received '0x{:04x}'",
            addr.value(),
            a.value()
        );
    }
}
