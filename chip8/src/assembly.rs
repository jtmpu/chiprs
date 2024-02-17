pub mod lexer;
pub mod parser;

use crate::instructions::Instruction;

#[derive(Debug)]
pub struct Assembly {
    pub instructions: Vec<ParsedInstruction>,
}

impl Assembly {
    /// Converts the assembly to binary
    pub fn binary(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        for instr in &self.instructions {
            let opcode = instr.instruction.opcode();
            let bytes = opcode.to_be_bytes();
            buffer.extend(bytes);
        }
        buffer
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedInstruction {
    pub instruction: Instruction,
    pub label: Option<String>,
    pub source: Option<Source>,
}

impl ParsedInstruction {
    fn from_instruction(instruction: Instruction) -> Self {
        Self {
            instruction,
            label: None,
            source: None,
        }
    }

    fn from_instruction_label(instruction: Instruction, label: String) -> Self {
        Self {
            instruction,
            label: Some(label),
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
