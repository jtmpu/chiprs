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

use tracing::{debug, info, error};

use crate::assembly::{Assembly, ParsedInstruction};
use crate::instructions::{Instruction};
use crate::assembly::lexer::{Lexer, Token};

#[derive(Debug)]
enum Line {
    Comment(String),
    Label(String),
    Instruction(String, Option<String>, Option<String>),
    InstructionWithComment(String, Option<String>, Option<String>, String),
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
        let mut counter = 0;
        loop {
            if matches!(self.peek()?, Token::Whitespace) {
                counter += 1;
                self.pop()?;
            } else {
                debug!("trimmed {} whitespace", counter);
                return Ok(())
            }
        }
    }

    fn try_parse_comment(&mut self) -> Result<Line, ()> {
        debug!("parsing comment, needs to start with semicolon: {:?}", self.peek()?);
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
            // Unexpected token
            error!("Unexpected token in label parsing: {:?}", self.peek()?);
            return Err(())
        }
        // Unexpected token start
        error!("Unexpected token in label parsing start: {:?}", previous);
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
                        return Ok(Line::InstructionWithComment(op.clone(), None, None, comment.clone()));
                    }
                    // Something weird happened
                    return Err(());
                },
                Token::EOL | Token::EOF => {
                    self.pop()?;
                    return Ok(Line::Instruction(op.clone(), None, None));
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
                        return Ok(Line::InstructionWithComment(op.clone(), Some(second), None, comment.clone()));
                    }
                    // Something weird happened
                    return Err(());
                },
                Token::EOL | Token::EOF => {
                    self.pop()?;
                    return Ok(Line::Instruction(op.clone(), Some(second), None));
                }
                _ => return Err(()),
            };

            self.trim_whitespace()?;

            match self.peek()? {
                Token::Semicolon => {
                    if let Line::Comment(comment) = self.try_parse_comment()? {
                        return Ok(Line::InstructionWithComment(op.clone(), Some(second), Some(third), comment.clone()));
                    }
                    // Something weird happened
                    return Err(());
                },
                Token::EOL | Token::EOF => {
                    self.pop()?;
                    return Ok(Line::Instruction(op.clone(), Some(second), Some(third)));
                }
                _ => return Err(()),
            }
        }
        // Unexpected token start
        return Err(())
    }

    fn try_parse_line(&mut self) -> Result<Option<Line>, ()> {
        self.trim_whitespace()?;
        let token = self.pop()?;
        debug!("parsing line starting with {:?}", token);
        match &token {
            Token::EOF => {
                debug!("reached EOF");
                return Ok(None)
            }
            Token::Semicolon => {
                debug!("found comment start");
                return self.try_parse_comment().map(|v| Some(v));
            }
            Token::Alphanumeric(_) => {
                if matches!(self.peek()?, Token::Colon) {
                    debug!("found label start");
                    return self.try_parse_label(&token).map(|v| Some(v));
                }
                debug!("found instruction start");
                return self.try_parse_instruction(&token).map(|v| Some(v));
            }
            _ => {
                // Unexpected token
                error!("unknown token {:?}", token);
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

        for line in lines {
            debug!("{:?}", line);
        }

        let instructions = Vec::new();
        Ok(Assembly { instructions })
    }
}
