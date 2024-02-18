//! 
//! Chip-8 parser
//!
//! There's implied whitespace everywhere
//! There's implied comment ignore
//! <end> ::= <eol> | <eof>
//! <empty> ::= <whitespace> ...
//! <comment> ::= <semi-colon> <anything> 
//! <label> ::= <literal> <colon>
//! <literal> ::= <integer> | <alphanumeric> 
//! <instruction> ::= <literal> <literal> <literal> <end> 
//!     | <literal> <literal> <eol> 
//!     | <literal> <eol>
//! <instruction-opt-label> ::= <label> <instruction> | <instruction>
//! <instruction-opt-label-opt-comment> ::= <instruction-opt-label> <comment>
//! <line> ::= <comment><end> | <instruction-opt-label-opt-comment><end> | <empty><end>
//! <assembly> ::= <assembly> <line> | <line>

use std::io::Read;

use std::collections::HashMap;
use tracing::{debug, info, error};

use crate::assembly::{Assembly, ParsedInstruction};
use crate::instructions::{Instruction, u4, u12};
use crate::assembly::lexer::{Lexer, Token};

#[derive(Debug)]
struct RawInstr {
    operation: String,
    arg1: Option<String>,
    arg2: Option<String>,
    comment: Option<String>,
}

impl RawInstr {
    fn try_to_instruction(&self) -> Result<ParsedInstruction, ()> {
        let mut label: Option<String> = None;
        let instruction = match self.operation.as_str() {
            "clear" => {
                if let Some(v) = &self.arg1 {
                    return Err(())
                }
                if let Some(v) = &self.arg1 {
                    return Err(())
                }
                Instruction::Clear
            },
            "sne" => {
                let reg_index = RawInstr::parse_as_registry(self.arg1.as_ref())?;
                let value = RawInstr::parse_as_value(self.arg2.as_ref())?;
                Instruction::SkipNotEqual(reg_index, value)
            }
            "mov" => {
                let reg_index = RawInstr::parse_as_registry(self.arg1.as_ref())?;
                let value = RawInstr::parse_as_value(self.arg2.as_ref())?;
                Instruction::Move(reg_index, value)
            },
            "add" => {
                let reg_index = RawInstr::parse_as_registry(self.arg1.as_ref())?;
                let value = RawInstr::parse_as_value(self.arg2.as_ref())?;
                Instruction::Add(reg_index, value)
            },
            "jmp" => {
                let addr = if let Some(s) = self.arg1.as_ref() {
                    match RawInstr::parse_as_address(self.arg1.as_ref()) {
                        Ok(v) => v,
                        Err(_) => {
                            // No integer found, this must be a label
                            label = Some(s.clone());
                            0.into()
                        }
                    } 
                } else {
                    return Err(())
                };
                if let Some(v) = &self.arg2 {
                    return Err(())
                }
                Instruction::Jump(addr)
            },
            _ => {
                return Err(())
            }
        };
        println!("{:?}, {:?}", instruction, label);
        let parsed = ParsedInstruction {
            instruction,
            label,
            source: None,
        };
        Ok(parsed)
    }
    fn parse_as_registry(arg: Option<&String>) -> Result<u4, ()> {
        let value = if let Some(value) = arg {
            value
        } else {
            return Err(())
        };
        let index = match value.strip_prefix("r") {
            Some(n) => n,
            None => return Err(()),
        };
        let index = index.parse::<u8>().map_err(|_| ())?;
        Ok(u4::little(index))
    }

    fn parse_as_value(arg: Option<&String>) -> Result<u8, ()> {
        let value = if let Some(value) = arg {
            value
        } else {
            return Err(())
        };
        let num = value.parse::<u8>().map_err(|_| ())?;
        Ok(num)
    }

    fn parse_as_address(arg: Option<&String>) -> Result<u12, ()> {
        let value = if let Some(value) = arg {
            value
        } else {
            return Err(())
        };
        let num = value.parse::<u16>().map_err(|_| ())?;
        Ok(u12::from_u16(num))
    }
}

#[derive(Debug)]
enum Line {
    Comment(String),
    Label(String),
    Instruction(RawInstr),
}

impl Line {
    fn is_comment(&self) -> bool {
        matches!(self, Line::Comment(_))
    }
}

pub struct Parser<T: Read> {
    lexer: Lexer<T>,
    has_peeked: bool,
    peek: Token,
}

impl<T: Read> Parser<T> {
    pub fn new(reader: T) -> Self {
        Self {
            lexer: Lexer::new(reader),
            peek: Token::EOF,
            has_peeked: false,
        }
    }

    fn pop(&mut self) -> Result<Token, ()> {
        if self.has_peeked {
            self.has_peeked = false;
            Ok(self.peek.clone())
        } else {
            Ok(self.lexer.next()?)
        }
    }

    fn peek(&mut self) -> Result<&Token, ()> {
        if self.has_peeked {
            return Ok(&self.peek);
        }
        self.peek = self.pop()?;
        self.has_peeked = true;
        Ok(&self.peek)
    }

    fn trim_whitespace(&mut self) -> Result<(), ()> {
        loop {
            if matches!(self.peek()?, Token::Whitespace) {
                self.pop()?;
            } else {
                return Ok(())
            }
        }
    }

    fn try_parse_comment(&mut self) -> Result<Line, ()> {
        if matches!(self.peek()?, Token::Semicolon) {
            self.pop()?;
        }
        loop {
            let token = self.pop()?;
            if matches!(token, Token::EOL | Token::EOF) {
                return Ok(Line::Comment("".to_string()));
            }
        }
    }

    fn try_parse_label(&mut self, previous: &Token) -> Result<Line, ()> {
        if let Token::Alphanumeric(v) = previous {
            if matches!(self.peek()?, Token::Colon) {
                self.pop()?;
                let label = v.clone();
                loop {
                    match self.peek()? {
                        Token::EOL => {
                            self.pop()?;
                            return Ok(Line::Label(label))
                        },
                        Token::Whitespace => self.pop()?,
                        t => {
                            error!("Unexpected token {:?}", t);
                            return Err(())
                        },
                    };
                }
            };
            return Err(())
        }
        return Err(())

    }

    fn try_parse_instruction(&mut self, previous: &Token) -> Result<Line, ()> {
        if let Token::Alphanumeric(op) = previous {
            self.trim_whitespace()?;

            let second = match self.peek()? {
                Token::Alphanumeric(p1) => {
                    let ret = p1.clone();
                    self.pop()?;
                    ret
                },
                Token::Integer(p2) => {
                    let ret = p2.to_string();
                    self.pop()?;
                    ret
                },
                Token::Semicolon => {
                    if let Line::Comment(comment) = self.try_parse_comment()? {
                        let instr = RawInstr {
                            operation: op.clone(),
                            arg1: None,
                            arg2: None,
                            comment: Some(comment.clone()),
                        };
                        return Ok(Line::Instruction(instr));
                    }
                    // Something weird happened
                    return Err(());
                },
                Token::EOL | Token::EOF => {
                    self.pop()?;
                    let instr = RawInstr {
                        operation: op.clone(),
                        arg1: None,
                        arg2: None,
                        comment: None,
                    };
                    return Ok(Line::Instruction(instr));
                }
                _ => return Err(()),
            };

            self.trim_whitespace()?;

            let third = match self.peek()? {
                Token::Alphanumeric(p1) => {
                    let ret = p1.clone();
                    self.pop()?;
                    ret
                },
                Token::Integer(p2) => {
                    let ret = p2.to_string();
                    self.pop()?;
                    ret
                },
                Token::Semicolon => {
                    if let Line::Comment(comment) = self.try_parse_comment()? {
                        let instr = RawInstr {
                            operation: op.clone(),
                            arg1: Some(second),
                            arg2: None,
                            comment: Some(comment.clone()),
                        };
                        return Ok(Line::Instruction(instr));
                    }
                    // Something weird happened
                    return Err(());
                },
                Token::EOL | Token::EOF => {
                    self.pop()?;
                    let instr = RawInstr {
                        operation: op.clone(),
                        arg1: Some(second),
                        arg2: None,
                        comment: None,
                    };
                    return Ok(Line::Instruction(instr));
                }
                _ => return Err(()),
            };

            self.trim_whitespace()?;

            match self.peek()? {
                Token::Semicolon => {
                    if let Line::Comment(comment) = self.try_parse_comment()? {
                        let instr = RawInstr {
                            operation: op.clone(),
                            arg1: Some(second),
                            arg2: Some(third),
                            comment: Some(comment.clone()),
                        };
                        return Ok(Line::Instruction(instr));
                    }
                    // Something weird happened
                    return Err(());
                },
                Token::EOL | Token::EOF => {
                    self.pop()?;
                    let instr = RawInstr {
                        operation: op.clone(),
                        arg1: Some(second),
                        arg2: Some(third),
                        comment: None,
                    };
                    return Ok(Line::Instruction(instr));
                }
                _ => return Err(()),
            }
        }
        // Unexpected token start
        return Err(())
    }

    fn try_parse_line(&mut self) -> Result<Option<Line>, ()> {
        loop {
            // remove empty spaces and lines
            self.trim_whitespace()?;
            match self.peek()? {
                Token::EOL => {
                    self.pop()?;
                }
                Token::EOF => {
                    self.pop()?;
                    return Ok(None)
                }
                _ => break,
            };
        }
        let token = self.pop()?;
        match &token {
            Token::EOF => {
                return Ok(None)
            }
            Token::Semicolon => {
                return self.try_parse_comment().map(|v| Some(v));
            }
            Token::Alphanumeric(_) => {
                if matches!(self.peek()?, Token::Colon) {
                    return self.try_parse_label(&token).map(|v| Some(v));
                }
                return self.try_parse_instruction(&token).map(|v| Some(v));
            }
            _ => {
                return Err(());
            }
        }
    }

    pub fn parse(&mut self) -> Result<Assembly, ()> {
        let mut lines = Vec::new();

        loop {
            let line = self.try_parse_line()?;
            if line.is_none() {
                // Reached EOF
                break;
            }
            let line = line.unwrap();
            lines.push(line);
        }

        let (instructions, labels) = convert_to_instructions(lines)?;

        // Check for non-existent addresses
        for i in &instructions {
            if let Some(label) = &i.label {
                if !labels.contains_key(label) {
                    return Err(())
                }
            }
        }

        Ok(Assembly { instructions, labels })
    }
}

fn convert_to_instructions(lines: Vec<Line>) -> Result<(Vec<ParsedInstruction>, HashMap<String, usize>), ()> {
    // Filter comments
    let lines: Vec<&Line> = lines.iter()
        .filter(|l| !l.is_comment())
        .collect();

    let mut instructions = Vec::new();
    let mut labels = HashMap::new();
    let mut cursor = 0;
    let mut instr_cursor = 0;
    loop {
        if cursor >= lines.len() {
            break;
        }

        let line = lines[cursor];
        match line {
            Line::Instruction(raw) => {
                instructions.push(raw.try_to_instruction()?);
                instr_cursor += 1;
            },
            Line::Label(label) => {
                labels.insert(label.clone(), instr_cursor);
            },
            _ => {
                return Err(())
            }
        };
        cursor += 1;
    }
    Ok((instructions, labels))
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::BufReader;

    fn parse_and_assert(input: &str, expected: Vec<ParsedInstruction>) {
        let reader = BufReader::new(input.as_bytes());
        let mut parser = Parser::new(reader);
        let assembly = parser.parse().unwrap();
        for (e, r) in (&expected).into_iter().zip(&assembly.instructions) {
            assert_eq!(e, r);
        }
    }

    #[test]
    fn parse_clear() {
        parse_and_assert(
            "clear",
            vec![
                Instruction::Clear,
            ].iter()
                .map(|e| ParsedInstruction::new(e.clone()))
                .collect(),
        );
    }

    #[test]
    fn parse_mov() {
        parse_and_assert(
            "mov r1 42",
            vec![
                Instruction::Move(u4::little(0x01), 42),
            ].iter()
                .map(|e| ParsedInstruction::new(e.clone()))
                .collect(),
        );
    }

    #[test]
    fn parse_jmp() {
        parse_and_assert(
            "jmp 123",
            vec![
                Instruction::Jump(u12::from_u16(123)),
            ].iter()
                .map(|e| ParsedInstruction::new(e.clone()))
                .collect(),
        );
    }

    #[test]
    fn parse_skip_not_equal() {
        parse_and_assert(
            "sne r5 10",
            vec![
                Instruction::SkipNotEqual(u4::little(0x05), 10),
            ].iter()
                .map(|e| ParsedInstruction::new(e.clone()))
                .collect(),
        );
    }

    #[test]
    fn parse_add() {
        parse_and_assert(
            "add r14 30",
            vec![
                Instruction::Add(u4::little(14), 30),
            ].iter()
                .map(|e| ParsedInstruction::new(e.clone()))
                .collect(),
        );
    }

    #[test]
    fn parse_label() {
        let input = "main:\nadd r14 30";
        let expected: Vec<ParsedInstruction> = vec![
                Instruction::Add(u4::little(14), 30),
            ].iter()
                .map(|e| ParsedInstruction::new(e.clone()))
                .collect();
        let reader = BufReader::new(input.as_bytes());
        let mut parser = Parser::new(reader);
        let assembly = parser.parse().unwrap();
        for (e, r) in (&expected).into_iter().zip(&assembly.instructions) {
            assert_eq!(e, r);
        }

        let location = assembly.labels.get("main");
        assert!(location.is_some());
        let location = *location.unwrap();
        assert_eq!(location, 0);
    }

    #[test]
    fn parse_integration() {
        let expected: Vec<ParsedInstruction> = vec![
            ParsedInstruction::new(
                Instruction::Move(u4::little(1), 0),
            ),
            ParsedInstruction::new(
                Instruction::Add(u4::little(1), 1),
            ),
            ParsedInstruction::new(
                Instruction::Clear,
            ),
            ParsedInstruction::new(
                Instruction::SkipNotEqual(u4::little(1), 4),
            ),
            ParsedInstruction::new(
                Instruction::Jump(u12::from_u16(123)),
            ),
        ];
        let input = "; this asm contains a little bit of everything
main:
    mov r1 0
    add r1 1
    clear
    sne r1 4 ; abort
    jmp 123


other:
    add r2 0
        ";

        let reader = BufReader::new(input.as_bytes());
        let mut parser = Parser::new(reader);
        let assembly = parser.parse().unwrap();
        for (e, r) in (&expected).into_iter().zip(&assembly.instructions) {
            assert_eq!(e, r);
        }

        let location = assembly.labels.get("main");
        assert!(location.is_some());
        let location = *location.unwrap();
        assert_eq!(location, 0);

        let location = assembly.labels.get("other");
        assert!(location.is_some());
        let location = *location.unwrap();
        assert_eq!(location, 5);
    }
}
