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

use std::collections::HashMap;

use crate::assembly::lexer::{Lexer, LexerError, Token};
use crate::assembly::{Assembly, ParsedInstruction};
use crate::instructions::{u12, u4, Instruction};

use std::error::Error;
use std::fmt;
use std::num::ParseIntError;

#[derive(Debug)]
pub enum ArgumentError {
    IntegerParse(ParseIntError),
    MissingRegistryPrefix(String),
    UnexpectedArgument(String),
    MissingArgument,
}

impl fmt::Display for ArgumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::IntegerParse(ref err) => err.fmt(f),
            Self::MissingRegistryPrefix(ref v) => {
                write!(f, "Missing registry prefix 'r' for '{}'", v)
            }
            Self::UnexpectedArgument(ref v) => write!(f, "Unexpected argument '{}'", v),
            Self::MissingArgument => write!(f, "Missing required argument"),
        }
    }
}

impl Error for ArgumentError {}

impl From<ParseIntError> for ArgumentError {
    fn from(err: ParseIntError) -> Self {
        ArgumentError::IntegerParse(err)
    }
}

type Location = (usize, usize);
#[derive(Debug)]
pub enum ParsingError {
    Lexer(LexerError),
    ArgumentError(&'static str, Location, ArgumentError),
    UnknownInstruction(String, Location),
    UnexpectedToken(&'static str, Token, Location),
    MissingReferencedLabel(String),
    Unknown(String),
}

impl fmt::Display for ParsingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Lexer(ref err) => err.fmt(f),
            Self::ArgumentError(instruction, location, ref e) => {
                write!(
                    f,
                    "Failed parsing instruction '{}' (Loc: {},{}), argument error: {}",
                    instruction, location.0, location.1, e
                )
            }
            Self::UnknownInstruction(ref instr, location) => {
                write!(
                    f,
                    "Unknown instruction '{}' (Loc: {},{})",
                    instr, location.0, location.1
                )
            }
            Self::UnexpectedToken(step, ref token, location) => {
                write!(
                    f,
                    "Unexpected token '{}' (Loc: {},{}) while processing step '{}'",
                    token, location.0, location.1, step
                )
            }
            Self::Unknown(ref msg) => {
                write!(f, "Unknown state and error: {}", msg)
            }
            Self::MissingReferencedLabel(ref label) => {
                write!(f, "Missing referenced label {}", label)
            }
        }
    }
}

impl Error for ParsingError {}

impl From<LexerError> for ParsingError {
    fn from(err: LexerError) -> ParsingError {
        ParsingError::Lexer(err)
    }
}

#[derive(Debug)]
struct RawInstr {
    operation: String,
    arg1: Option<String>,
    arg2: Option<String>,
    arg3: Option<String>,
    _comment: Option<String>,
    location: Location,
}

impl RawInstr {
    fn try_to_instruction(&self) -> Result<ParsedInstruction, ParsingError> {
        let mut label: Option<String> = None;
        let instruction = match self.operation.as_str() {
            "exit" => {
                if let Some(v) = &self.arg1 {
                    return Err(ParsingError::ArgumentError(
                        "exit",
                        self.location,
                        ArgumentError::UnexpectedArgument(v.clone()),
                    ));
                }
                if let Some(v) = &self.arg2 {
                    return Err(ParsingError::ArgumentError(
                        "exit",
                        self.location,
                        ArgumentError::UnexpectedArgument(v.clone()),
                    ));
                }
                Instruction::Exit
            }
            "clear" => {
                if let Some(v) = &self.arg1 {
                    return Err(ParsingError::ArgumentError(
                        "clear",
                        self.location,
                        ArgumentError::UnexpectedArgument(v.clone()),
                    ));
                }
                if let Some(v) = &self.arg2 {
                    return Err(ParsingError::ArgumentError(
                        "clear",
                        self.location,
                        ArgumentError::UnexpectedArgument(v.clone()),
                    ));
                }
                Instruction::Clear
            }
            "ret" => {
                if let Some(v) = &self.arg1 {
                    return Err(ParsingError::ArgumentError(
                        "ret",
                        self.location,
                        ArgumentError::UnexpectedArgument(v.clone()),
                    ));
                }
                if let Some(v) = &self.arg2 {
                    return Err(ParsingError::ArgumentError(
                        "ret",
                        self.location,
                        ArgumentError::UnexpectedArgument(v.clone()),
                    ));
                }
                Instruction::Return
            }
            "call" => {
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
                    return Err(ParsingError::ArgumentError(
                        "call",
                        self.location,
                        ArgumentError::MissingArgument,
                    ));
                };
                if let Some(v) = &self.arg2 {
                    return Err(ParsingError::ArgumentError(
                        "call",
                        self.location,
                        ArgumentError::UnexpectedArgument(v.clone()),
                    ));
                }
                Instruction::Call(addr)
            }
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
                    return Err(ParsingError::ArgumentError(
                        "jmp",
                        self.location,
                        ArgumentError::MissingArgument,
                    ));
                };
                if let Some(v) = &self.arg2 {
                    return Err(ParsingError::ArgumentError(
                        "jmp",
                        self.location,
                        ArgumentError::UnexpectedArgument(v.clone()),
                    ));
                }
                Instruction::Jump(addr)
            }
            "sne" => {
                let reg_index = RawInstr::parse_as_registry(self.arg1.as_ref())
                    .map_err(|e| ParsingError::ArgumentError("sne", self.location, e))?;
                let value = RawInstr::parse_as_value(self.arg2.as_ref())
                    .map_err(|e| ParsingError::ArgumentError("sne", self.location, e))?;
                if let Some(v) = &self.arg3 {
                    return Err(ParsingError::ArgumentError(
                        "sne",
                        self.location,
                        ArgumentError::UnexpectedArgument(v.clone()),
                    ));
                }
                Instruction::SkipNotEqual(reg_index, value)
            }
            "ldb" => {
                let reg_index = RawInstr::parse_as_registry(self.arg1.as_ref())
                    .map_err(|e| ParsingError::ArgumentError("ldb", self.location, e))?;
                let value = RawInstr::parse_as_value(self.arg2.as_ref())
                    .map_err(|e| ParsingError::ArgumentError("ldb", self.location, e))?;
                if let Some(v) = &self.arg3 {
                    return Err(ParsingError::ArgumentError(
                        "ldb",
                        self.location,
                        ArgumentError::UnexpectedArgument(v.clone()),
                    ));
                }
                Instruction::SetRegisterByte(reg_index, value)
            }
            "add" => {
                let reg_index = RawInstr::parse_as_registry(self.arg1.as_ref())
                    .map_err(|e| ParsingError::ArgumentError("add", self.location, e))?;
                let value = RawInstr::parse_as_value(self.arg2.as_ref())
                    .map_err(|e| ParsingError::ArgumentError("add", self.location, e))?;
                if let Some(v) = &self.arg3 {
                    return Err(ParsingError::ArgumentError(
                        "add",
                        self.location,
                        ArgumentError::UnexpectedArgument(v.clone()),
                    ));
                }
                Instruction::Add(reg_index, value)
            }
            "or" => {
                let regx_index = RawInstr::parse_as_registry(self.arg1.as_ref())
                    .map_err(|e| ParsingError::ArgumentError("or", self.location, e))?;
                let regy_index = RawInstr::parse_as_registry(self.arg2.as_ref())
                    .map_err(|e| ParsingError::ArgumentError("or", self.location, e))?;
                if let Some(v) = &self.arg3 {
                    return Err(ParsingError::ArgumentError(
                        "or",
                        self.location,
                        ArgumentError::UnexpectedArgument(v.clone()),
                    ));
                }
                Instruction::Or(regx_index, regy_index)
            }
            "ldf" => {
                let regx_index = RawInstr::parse_as_registry(self.arg1.as_ref())
                    .map_err(|e| ParsingError::ArgumentError("ldf", self.location, e))?;
                if let Some(v) = &self.arg2 {
                    return Err(ParsingError::ArgumentError(
                        "ldf",
                        self.location,
                        ArgumentError::UnexpectedArgument(v.clone()),
                    ));
                }
                if let Some(v) = &self.arg3 {
                    return Err(ParsingError::ArgumentError(
                        "ldf",
                        self.location,
                        ArgumentError::UnexpectedArgument(v.clone()),
                    ));
                }
                Instruction::SetMemRegisterDefaultSprit(regx_index)
            }
            "draw" => {
                let regx_index = RawInstr::parse_as_registry(self.arg1.as_ref())
                    .map_err(|e| ParsingError::ArgumentError("draw", self.location, e))?;
                let regy_index = RawInstr::parse_as_registry(self.arg2.as_ref())
                    .map_err(|e| ParsingError::ArgumentError("draw", self.location, e))?;
                let value = RawInstr::parse_as_nibble(self.arg3.as_ref())
                    .map_err(|e| ParsingError::ArgumentError("draw", self.location, e))?;
                Instruction::Draw(regx_index, regy_index, value)
            }
            "ldd" => {
                let regx_index = RawInstr::parse_as_registry(self.arg1.as_ref())
                    .map_err(|e| ParsingError::ArgumentError("ldd", self.location, e))?;
                if let Some(v) = &self.arg2 {
                    return Err(ParsingError::ArgumentError(
                        "ldd",
                        self.location,
                        ArgumentError::UnexpectedArgument(v.clone()),
                    ));
                }
                if let Some(v) = &self.arg3 {
                    return Err(ParsingError::ArgumentError(
                        "ldd",
                        self.location,
                        ArgumentError::UnexpectedArgument(v.clone()),
                    ));
                }
                Instruction::SetRegisterDelayTimer(regx_index)
            }
            "delay" => {
                let regx_index = RawInstr::parse_as_registry(self.arg1.as_ref())
                    .map_err(|e| ParsingError::ArgumentError("delay", self.location, e))?;
                if let Some(v) = &self.arg2 {
                    return Err(ParsingError::ArgumentError(
                        "delay",
                        self.location,
                        ArgumentError::UnexpectedArgument(v.clone()),
                    ));
                }
                if let Some(v) = &self.arg3 {
                    return Err(ParsingError::ArgumentError(
                        "delay",
                        self.location,
                        ArgumentError::UnexpectedArgument(v.clone()),
                    ));
                }
                Instruction::SetDelayTimer(regx_index)
            }
            "skp" => {
                let regx_index = RawInstr::parse_as_registry(self.arg1.as_ref())
                    .map_err(|e| ParsingError::ArgumentError("skp", self.location, e))?;
                if let Some(v) = &self.arg2 {
                    return Err(ParsingError::ArgumentError(
                        "skp",
                        self.location,
                        ArgumentError::UnexpectedArgument(v.clone()),
                    ));
                }
                if let Some(v) = &self.arg3 {
                    return Err(ParsingError::ArgumentError(
                        "skp",
                        self.location,
                        ArgumentError::UnexpectedArgument(v.clone()),
                    ));
                }
                Instruction::SkipKeyPressed(regx_index)
            }
            "sknp" => {
                let regx_index = RawInstr::parse_as_registry(self.arg1.as_ref())
                    .map_err(|e| ParsingError::ArgumentError("sknp", self.location, e))?;
                if let Some(v) = &self.arg2 {
                    return Err(ParsingError::ArgumentError(
                        "sknp",
                        self.location,
                        ArgumentError::UnexpectedArgument(v.clone()),
                    ));
                }
                if let Some(v) = &self.arg3 {
                    return Err(ParsingError::ArgumentError(
                        "sknp",
                        self.location,
                        ArgumentError::UnexpectedArgument(v.clone()),
                    ));
                }
                Instruction::SkipKeyNotPressed(regx_index)
            }
            "input" => {
                let regx_index = RawInstr::parse_as_registry(self.arg1.as_ref())
                    .map_err(|e| ParsingError::ArgumentError("input", self.location, e))?;
                if let Some(v) = &self.arg2 {
                    return Err(ParsingError::ArgumentError(
                        "input",
                        self.location,
                        ArgumentError::UnexpectedArgument(v.clone()),
                    ));
                }
                if let Some(v) = &self.arg3 {
                    return Err(ParsingError::ArgumentError(
                        "input",
                        self.location,
                        ArgumentError::UnexpectedArgument(v.clone()),
                    ));
                }
                Instruction::WaitForKey(regx_index)
            }
            instr => {
                return Err(ParsingError::UnknownInstruction(
                    instr.to_string(),
                    self.location,
                ));
            }
        };
        let parsed = ParsedInstruction {
            instruction,
            label,
            source: None,
        };
        Ok(parsed)
    }
    fn parse_as_registry(arg: Option<&String>) -> Result<u4, ArgumentError> {
        let value = if let Some(value) = arg {
            value
        } else {
            return Err(ArgumentError::MissingArgument);
        };
        let index = match value.strip_prefix('r') {
            Some(n) => n,
            None => return Err(ArgumentError::MissingRegistryPrefix(value.clone())),
        };
        let index = index.parse::<u8>()?;
        Ok(u4::little(index))
    }

    fn parse_as_nibble(arg: Option<&String>) -> Result<u4, ArgumentError> {
        let value = if let Some(value) = arg {
            value
        } else {
            return Err(ArgumentError::MissingArgument);
        };
        let index = value.parse::<u8>()?;
        Ok(u4::little(index))
    }

    fn parse_as_value(arg: Option<&String>) -> Result<u8, ArgumentError> {
        let value = if let Some(value) = arg {
            value
        } else {
            return Err(ArgumentError::MissingArgument);
        };
        let num = value.parse::<u8>()?;
        Ok(num)
    }

    fn parse_as_address(arg: Option<&String>) -> Result<u12, ArgumentError> {
        let value = if let Some(value) = arg {
            value
        } else {
            return Err(ArgumentError::MissingArgument);
        };
        let num = value.parse::<u16>()?;
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

pub struct Parser {
    lexer: Box<dyn Lexer>,
    has_peeked: bool,
    peek: Token,
}

impl Parser {
    pub fn new(lexer: Box<dyn Lexer>) -> Self {
        Self {
            lexer,
            peek: Token::EOF,
            has_peeked: false,
        }
    }

    fn pop(&mut self) -> Result<Token, ParsingError> {
        if self.has_peeked {
            self.has_peeked = false;
            Ok(self.peek.clone())
        } else {
            Ok(self.lexer.next()?)
        }
    }

    fn peek(&mut self) -> Result<&Token, ParsingError> {
        if self.has_peeked {
            return Ok(&self.peek);
        }
        self.peek = self.pop()?;
        self.has_peeked = true;
        Ok(&self.peek)
    }

    fn trim_whitespace(&mut self) -> Result<(), ParsingError> {
        loop {
            if matches!(self.peek()?, Token::Whitespace) {
                self.pop()?;
            } else {
                return Ok(());
            }
        }
    }

    fn try_parse_comment(&mut self) -> Result<Line, ParsingError> {
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

    fn try_parse_label(
        &mut self,
        previous: &Token,
        location: Location,
    ) -> Result<Line, ParsingError> {
        if let Token::Alphanumeric(v) = previous {
            let location = self.lexer.location();
            let token = self.peek()?;
            if matches!(token, Token::Colon) {
                self.pop()?;
                let label = v.clone();
                loop {
                    let location = self.lexer.location();
                    match self.peek()? {
                        Token::EOL => {
                            self.pop()?;
                            return Ok(Line::Label(label));
                        }
                        Token::Whitespace => self.pop()?,
                        t => {
                            return Err(ParsingError::UnexpectedToken(
                                "parse:label:consume-line",
                                t.clone(),
                                location,
                            ));
                        }
                    };
                }
            };
            return Err(ParsingError::UnexpectedToken(
                "parse:label:end-label",
                token.clone(),
                location,
            ));
        }
        Err(ParsingError::UnexpectedToken(
            "parse:label:start",
            previous.clone(),
            location,
        ))
    }

    fn try_parse_instruction(&mut self, previous: &Token) -> Result<Line, ParsingError> {
        let line = self.lexer.line();
        let start_location = (line, 0);
        if let Token::Alphanumeric(op) = previous {
            self.trim_whitespace()?;

            let location = self.lexer.location();
            let second = match self.peek()? {
                Token::Alphanumeric(p1) => {
                    let ret = p1.clone();
                    self.pop()?;
                    ret
                }
                Token::Integer(p2) => {
                    let ret = p2.to_string();
                    self.pop()?;
                    ret
                }
                Token::Semicolon => {
                    let result = self.try_parse_comment()?;
                    if let Line::Comment(comment) = result {
                        let instr = RawInstr {
                            operation: op.clone(),
                            arg1: None,
                            arg2: None,
                            arg3: None,
                            _comment: Some(comment.clone()),
                            location: start_location,
                        };
                        return Ok(Line::Instruction(instr));
                    }
                    // Something weird happened
                    return Err(ParsingError::Unknown(format!(
                        "expected parsed comment, received {:?}",
                        result
                    )));
                }
                Token::EOL | Token::EOF => {
                    self.pop()?;
                    let instr = RawInstr {
                        operation: op.clone(),
                        arg1: None,
                        arg2: None,
                        arg3: None,
                        _comment: None,
                        location: start_location,
                    };
                    return Ok(Line::Instruction(instr));
                }
                token => {
                    return Err(ParsingError::UnexpectedToken(
                        "parse:instruction:second",
                        token.clone(),
                        location,
                    ))
                }
            };

            self.trim_whitespace()?;

            let location = self.lexer.location();
            let third = match self.peek()? {
                Token::Alphanumeric(p1) => {
                    let ret = p1.clone();
                    self.pop()?;
                    ret
                }
                Token::Integer(p2) => {
                    let ret = p2.to_string();
                    self.pop()?;
                    ret
                }
                Token::Semicolon => {
                    let result = self.try_parse_comment()?;
                    if let Line::Comment(comment) = result {
                        let instr = RawInstr {
                            operation: op.clone(),
                            arg1: Some(second),
                            arg2: None,
                            arg3: None,
                            _comment: Some(comment.clone()),
                            location: start_location,
                        };
                        return Ok(Line::Instruction(instr));
                    }
                    // Something weird happened
                    return Err(ParsingError::Unknown(format!(
                        "expected parsed comment, received {:?}",
                        result
                    )));
                }
                Token::EOL | Token::EOF => {
                    self.pop()?;
                    let instr = RawInstr {
                        operation: op.clone(),
                        arg1: Some(second),
                        arg2: None,
                        arg3: None,
                        _comment: None,
                        location: start_location,
                    };
                    return Ok(Line::Instruction(instr));
                }
                token => {
                    return Err(ParsingError::UnexpectedToken(
                        "parse:instruction:third",
                        token.clone(),
                        location,
                    ))
                }
            };

            self.trim_whitespace()?;

            let location = self.lexer.location();
            let fourth = match self.peek()? {
                Token::Alphanumeric(p1) => {
                    let ret = p1.clone();
                    self.pop()?;
                    ret
                }
                Token::Integer(p2) => {
                    let ret = p2.to_string();
                    self.pop()?;
                    ret
                }
                Token::Semicolon => {
                    let result = self.try_parse_comment()?;
                    if let Line::Comment(comment) = result {
                        let instr = RawInstr {
                            operation: op.clone(),
                            arg1: Some(second),
                            arg2: Some(third),
                            arg3: None,
                            _comment: Some(comment.clone()),
                            location: start_location,
                        };
                        return Ok(Line::Instruction(instr));
                    }
                    // Something weird happened
                    return Err(ParsingError::Unknown(format!(
                        "expected parsed comment, received {:?}",
                        result
                    )));
                }
                Token::EOL | Token::EOF => {
                    self.pop()?;
                    let instr = RawInstr {
                        operation: op.clone(),
                        arg1: Some(second),
                        arg2: Some(third),
                        arg3: None,
                        _comment: None,
                        location: start_location,
                    };
                    return Ok(Line::Instruction(instr));
                }
                token => {
                    return Err(ParsingError::UnexpectedToken(
                        "parse:instruction:fourth",
                        token.clone(),
                        location,
                    ))
                }
            };

            self.trim_whitespace()?;
            let location = self.lexer.location();
            match self.peek()? {
                Token::Semicolon => {
                    let result = self.try_parse_comment()?;
                    if let Line::Comment(comment) = result {
                        let instr = RawInstr {
                            operation: op.clone(),
                            arg1: Some(second),
                            arg2: Some(third),
                            arg3: Some(fourth),
                            _comment: Some(comment.clone()),
                            location: start_location,
                        };
                        return Ok(Line::Instruction(instr));
                    }
                    // Something weird happened
                    return Err(ParsingError::Unknown(format!(
                        "expected parsed comment, received {:?}",
                        result
                    )));
                }
                Token::EOL | Token::EOF => {
                    self.pop()?;
                    let instr = RawInstr {
                        operation: op.clone(),
                        arg1: Some(second),
                        arg2: Some(third),
                        arg3: Some(fourth),
                        _comment: None,
                        location: start_location,
                    };
                    return Ok(Line::Instruction(instr));
                }
                token => {
                    return Err(ParsingError::UnexpectedToken(
                        "parse:instruction:end",
                        token.clone(),
                        location,
                    ))
                }
            }
        }
        // Unexpected token start
        Err(ParsingError::Unknown(format!(
            "failed to find complete instruction, starting with: {:?}",
            previous
        )))
    }

    fn try_parse_line(&mut self) -> Result<Option<Line>, ParsingError> {
        loop {
            // remove empty spaces and lines
            self.trim_whitespace()?;
            match self.peek()? {
                Token::EOL => {
                    self.pop()?;
                }
                Token::EOF => {
                    self.pop()?;
                    return Ok(None);
                }
                _ => break,
            };
        }
        let location = self.lexer.location();
        let token = self.pop()?;
        match &token {
            Token::EOF => Ok(None),
            Token::Semicolon => self.try_parse_comment().map(Some),
            Token::Alphanumeric(_) => {
                if matches!(self.peek()?, Token::Colon) {
                    return self.try_parse_label(&token, location).map(Some);
                }
                self.try_parse_instruction(&token).map(Some)
            }
            token => Err(ParsingError::UnexpectedToken(
                "parse:line",
                token.clone(),
                location,
            )),
        }
    }

    pub fn parse(&mut self) -> Result<Assembly, ParsingError> {
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
                    return Err(ParsingError::MissingReferencedLabel(label.clone()));
                }
            }
        }

        Ok(Assembly {
            instructions,
            labels,
        })
    }
}

fn convert_to_instructions(
    lines: Vec<Line>,
) -> Result<(Vec<ParsedInstruction>, HashMap<String, usize>), ParsingError> {
    // Filter comments
    let lines: Vec<&Line> = lines.iter().filter(|l| !l.is_comment()).collect();

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
            }
            Line::Label(label) => {
                labels.insert(label.clone(), instr_cursor);
            }
            line => {
                return Err(ParsingError::Unknown(format!(
                    "Failed to create instruction or label from {:?}",
                    line
                )));
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

    use crate::assembly::lexer::StreamLexer;

    fn parse_and_assert(input: &'static str, expected: Vec<ParsedInstruction>) {
        let reader = BufReader::new(input.as_bytes());
        let lexer = StreamLexer::new(reader);
        let mut parser = Parser::new(Box::new(lexer));
        let assembly = parser.parse().unwrap();
        for (e, r) in expected.iter().zip(&assembly.instructions) {
            assert_eq!(e, r);
        }
    }

    #[test]
    fn parse_clear() {
        parse_and_assert(
            "clear",
            [Instruction::Clear]
                .iter()
                .map(|e| ParsedInstruction::new(*e))
                .collect(),
        );
    }

    #[test]
    fn parse_ldb() {
        parse_and_assert(
            "ldb r1 42",
            [Instruction::SetRegisterByte(u4::little(0x01), 42)]
                .iter()
                .map(|e| ParsedInstruction::new(*e))
                .collect(),
        );
    }

    #[test]
    fn parse_jmp() {
        parse_and_assert(
            "jmp 123",
            [Instruction::Jump(u12::from_u16(123))]
                .iter()
                .map(|e| ParsedInstruction::new(*e))
                .collect(),
        );
    }

    #[test]
    fn parse_skip_not_equal() {
        parse_and_assert(
            "sne r5 10",
            [Instruction::SkipNotEqual(u4::little(0x05), 10)]
                .iter()
                .map(|e| ParsedInstruction::new(*e))
                .collect(),
        );
    }

    #[test]
    fn parse_or() {
        parse_and_assert(
            "or r1 r2",
            [Instruction::Or(0x01.into(), 0x02.into())]
                .iter()
                .map(|e| ParsedInstruction::new(*e))
                .collect(),
        );
    }

    #[test]
    fn parse_add() {
        parse_and_assert(
            "add r14 30",
            [Instruction::Add(u4::little(14), 30)]
                .iter()
                .map(|e| ParsedInstruction::new(*e))
                .collect(),
        );
    }

    #[test]
    fn parse_abort() {
        parse_and_assert(
            "exit",
            [Instruction::Exit]
                .iter()
                .map(|e| ParsedInstruction::new(*e))
                .collect(),
        );
    }

    #[test]
    fn parse_ret() {
        parse_and_assert(
            "ret",
            [Instruction::Return]
                .iter()
                .map(|e| ParsedInstruction::new(*e))
                .collect(),
        );
    }

    #[test]
    fn parse_call() {
        parse_and_assert(
            "call 123",
            [Instruction::Call(123.into())]
                .iter()
                .map(|e| ParsedInstruction::new(*e))
                .collect(),
        );
    }

    #[test]
    fn parse_ldf() {
        parse_and_assert(
            "ldf r4",
            [Instruction::SetMemRegisterDefaultSprit(4.into())]
                .iter()
                .map(|e| ParsedInstruction::new(*e))
                .collect(),
        );
    }

    #[test]
    fn parse_draw() {
        parse_and_assert(
            "draw r3 r4 2",
            [Instruction::Draw(3.into(), 4.into(), 2.into())]
                .iter()
                .map(|e| ParsedInstruction::new(*e))
                .collect(),
        );
    }

    #[test]
    fn parse_ldd() {
        parse_and_assert(
            "ldd r7",
            [Instruction::SetRegisterDelayTimer(7.into())]
                .iter()
                .map(|e| ParsedInstruction::new(*e))
                .collect(),
        );
    }

    #[test]
    fn parse_delay() {
        parse_and_assert(
            "delay r0",
            [Instruction::SetDelayTimer(0.into())]
                .iter()
                .map(|e| ParsedInstruction::new(*e))
                .collect(),
        );
    }

    #[test]
    fn parse_skip_key_pressed() {
        parse_and_assert(
            "skp r7",
            [Instruction::SkipKeyPressed(7.into())]
                .iter()
                .map(|e| ParsedInstruction::new(*e))
                .collect(),
        );
    }

    #[test]
    fn parse_skip_key_not_pressed() {
        parse_and_assert(
            "sknp r8",
            [Instruction::SkipKeyNotPressed(8.into())]
                .iter()
                .map(|e| ParsedInstruction::new(*e))
                .collect(),
        );
    }

    #[test]
    fn parse_wait_for_key() {
        parse_and_assert(
            "input r8",
            [Instruction::WaitForKey(8.into())]
                .iter()
                .map(|e| ParsedInstruction::new(*e))
                .collect(),
        );
    }

    #[test]
    fn parse_label() {
        let input = "main:\nadd r14 30";
        let expected: Vec<ParsedInstruction> = [Instruction::Add(u4::little(14), 30)]
            .iter()
            .map(|e| ParsedInstruction::new(*e))
            .collect();
        let reader = BufReader::new(input.as_bytes());
        let lexer = StreamLexer::new(reader);
        let mut parser = Parser::new(Box::new(lexer));
        let assembly = parser.parse().unwrap();
        for (e, r) in expected.iter().zip(&assembly.instructions) {
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
            ParsedInstruction::new(Instruction::SetRegisterByte(u4::little(1), 0)),
            ParsedInstruction::new(Instruction::Add(u4::little(1), 1)),
            ParsedInstruction::new(Instruction::Clear),
            ParsedInstruction::new(Instruction::SkipNotEqual(u4::little(1), 4)),
            ParsedInstruction::new(Instruction::Jump(u12::from_u16(123))),
        ];
        let input = "; this asm contains a little bit of everything
main:
    ldb r1 0
    add r1 1
    clear
    sne r1 4 ; abort
    jmp 123


other:
    add r2 0
        ";

        let reader = BufReader::new(input.as_bytes());
        let lexer = StreamLexer::new(reader);
        let mut parser = Parser::new(Box::new(lexer));
        let assembly = parser.parse().unwrap();
        for (e, r) in expected.iter().zip(&assembly.instructions) {
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
