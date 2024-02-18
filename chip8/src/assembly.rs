pub mod lexer;
pub mod parser;

use std::collections::HashMap;

use crate::instructions::Instruction;

#[derive(Debug)]
pub struct Assembly {
    pub instructions: Vec<ParsedInstruction>,
    pub labels: HashMap<String, usize>,
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
    pub source: Option<Source>,
}

impl ParsedInstruction {
    pub fn new(instruction: Instruction) -> Self {
        Self {
            instruction,
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
