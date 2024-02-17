pub mod lexer;
pub mod parser;

use crate::instructions::Instruction;

#[derive(Debug)]
pub struct Assembly {
    pub instructions: Vec<ParsedInstruction>,
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
