pub mod lexer;
pub mod parser;

use crate::instructions::Instruction;

#[derive(Debug)]
pub struct Assembly {
    pub instructions: Vec<ParsedInstruction>,
}

#[derive(Debug, Clone)]
pub struct ParsedInstruction {
    pub instruction: Instruction,
    pub label: Option<String>,
    pub source: Option<Source>,
}

#[derive(Debug, Clone)]
pub struct Source {
    pub file: String,
    pub line: usize,
    pub column: usize,
}
