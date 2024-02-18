//!
//! # Chip-8 lexer
//!

use std::fmt;
use std::error::Error;
use std::io::{Read, self};
use std::num::ParseIntError;

const BUFFER_SIZE: usize = 256;

#[derive(Debug)]
pub enum LexerError {
    IO(io::Error),
    IntegerParsing(ParseIntError),
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            LexerError::IO(ref err) => err.fmt(f),
            LexerError::IntegerParsing(ref err) => err.fmt(f),
        }
    }
}

impl Error for LexerError {
}

impl From<io::Error> for LexerError {
    fn from(err: io::Error) -> LexerError {
        LexerError::IO(err)
    }
}

impl From<ParseIntError> for LexerError {
    fn from(err: ParseIntError) -> LexerError {
        LexerError::IntegerParsing(err)
    }
}

/// Allows for reading and iterating tokens from a stream of bytes
pub trait Lexer {
    fn next(&mut self) -> Result<Token, LexerError>;
    fn location(&self) -> (usize, usize);

    fn line(&self) -> usize {
        let (line, _) = self.location();
        line
    }

    fn column(&self) -> usize {
        let (_, column) = self.location();
        column
    }
}


/// Simple stream lexer, which only maintains a set buffer of bytes
/// at a time. Can be used to lex over any stream which has implemented
/// the Read trait.
pub struct StreamLexer<T: Read> {
    reader: T,
    buffer: [u8; BUFFER_SIZE],
    cursor: usize,
    buffer_size: usize,
    line: usize,
    column: usize,
}

impl<T: Read> StreamLexer<T> {
    pub fn new(reader: T) -> Self { 
        Self {
            reader,
            buffer: [0; BUFFER_SIZE],
            cursor: BUFFER_SIZE,
            buffer_size: BUFFER_SIZE,
            line: 0,
            column: 0,
        }
    }

    fn is_buffer_end(&self) -> bool {
        self.cursor >= self.buffer_size
    }

    fn is_stream_end(&self) -> bool {
        self.is_buffer_end() && self.buffer_size < BUFFER_SIZE
    }

    fn load(&mut self) -> Result<(), LexerError> {
        self.buffer_size = self.reader.read(&mut self.buffer)?;
        self.cursor = 0;
        Ok(())
    }

    fn peek(&self) -> char {
        self.buffer[self.cursor] as char
    }

    fn pop(&mut self) -> Result<char, LexerError> {
        if self.is_buffer_end() {
            self.load()?;
        }
        let ret = self.peek();
        self.cursor += 1;
        self.column += 1;

        Ok(ret as char)
    }

    fn collect(&mut self, first: char, pred: fn(char) -> bool) -> Result<Vec<char>, LexerError> {
        let mut chars: Vec<char> = vec![first];
        loop {
            if self.is_stream_end() {
                break;
            }

            if pred(self.peek()) {
                chars.push(self.pop()?);
            } else {
                break;
            }
        }
        Ok(chars)
    }


    pub fn all(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = vec![];
        loop {
            let token = self.next()?;
            if matches!(token, Token::EOF) {
                tokens.push(Token::EOF);
                break;
            }
            tokens.push(token);
        }
        Ok(tokens)
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn column(&self) -> usize {
        self.column
    }

    pub fn location(&self) -> (usize, usize) {
        (self.line, self.column)
    }
}

impl<T: Read> Lexer for StreamLexer<T> {
    fn location(&self) -> (usize, usize) {
        (self.line, self.column)
    }

    fn next(&mut self) -> Result<Token, LexerError> {
        if self.is_stream_end() {
            return Ok(Token::EOF);
        }

        let byte = self.pop()?;
        let token = match byte {
            b if b == ',' => Token::Comma,
            b if b == ':' => Token::Colon,
            b if b == ';' => Token::Semicolon,
            b if is_whitespace(b) => {
                self.collect(b, is_whitespace)?;
                Token::Whitespace
            },
            b if b == '\n' => {
                self.line += 1;
                self.column = 0;
                Token::EOL
            },
            b if b == '\r' => {
                if self.peek() as char == '\n' {
                    self.pop()?;
                    self.line += 1;
                    self.column = 0;
                    Token::EOL
                } else {
                    Token::Symbol(b)
                }
            },
            b if b.is_ascii_punctuation() => Token::Symbol(b),
            b if b.is_ascii_digit() => {
                let number: String = self.collect(b, |e| e.is_ascii_digit())?
                    .into_iter()
                    .collect();
                let integer: usize = number.parse()?;
                Token::Integer(integer)
            },
            b if b.is_ascii_alphanumeric() => {
                let literal: String = self.collect(b, |e| e.is_ascii_alphanumeric())?
                    .into_iter()
                    .collect();
                Token::Alphanumeric(literal)
            },
            b => Token::Unknown(b as u8),
        };
        Ok(token)
    }
}

fn is_whitespace(c: char) -> bool {
    c == ' ' || c == '\t'
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Comma,
    Colon,
    Semicolon,
    Symbol(char),
    Integer(usize),
    Alphanumeric(String),
    Whitespace,
    Unknown(u8),
    EOL,
    EOF,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    fn lex_and_assert(input: &str, expected: Vec<Token>) {
        let mut lexer = StreamLexer::new(BufReader::new(input.as_bytes()));
        let result = lexer.all().unwrap();
        for (e, r) in (&expected).into_iter().zip(&result) {
            assert_eq!(e, r, "Expected '{:?}', got '{:?}'", expected, result);
        }
    }

    #[test]
    fn line_counter() {
        let input = "1\n2\r\n3";
        let mut lexer = StreamLexer::new(BufReader::new(input.as_bytes()));
        assert_eq!(lexer.line(), 0);
        assert_eq!(lexer.next().unwrap(), Token::Integer(1));
        assert_eq!(lexer.line(), 0);
        assert_eq!(lexer.next().unwrap(), Token::EOL);
        assert_eq!(lexer.line(), 1);
        assert_eq!(lexer.next().unwrap(), Token::Integer(2));
        assert_eq!(lexer.line(), 1);
        assert_eq!(lexer.next().unwrap(), Token::EOL);
        assert_eq!(lexer.line(), 2);
        assert_eq!(lexer.next().unwrap(), Token::Integer(3));
        assert_eq!(lexer.line(), 2);
        assert_eq!(lexer.next().unwrap(), Token::EOF);
    }

    #[test]
    fn column_counter() {
        let input = "1 abc 32";
        let mut lexer = StreamLexer::new(BufReader::new(input.as_bytes()));
        assert_eq!(lexer.column(), 0);
        assert_eq!(lexer.next().unwrap(), Token::Integer(1));
        assert_eq!(lexer.column(), 1);
        assert_eq!(lexer.next().unwrap(), Token::Whitespace);
        assert_eq!(lexer.column(), 2);
        assert_eq!(lexer.next().unwrap(), Token::Alphanumeric("abc".to_string()));
        assert_eq!(lexer.column(), 5);
        assert_eq!(lexer.next().unwrap(), Token::Whitespace);
        assert_eq!(lexer.column(), 6);
        assert_eq!(lexer.next().unwrap(), Token::Integer(32));
    }

    #[test]
    fn line_and_column_counter8() {
        let input = "1 abc\n32 ewq";
        let mut lexer = StreamLexer::new(BufReader::new(input.as_bytes()));
        assert_eq!(lexer.location(), (0,0));
        assert_eq!(lexer.next().unwrap(), Token::Integer(1));
        assert_eq!(lexer.location(), (0,1));
        assert_eq!(lexer.next().unwrap(), Token::Whitespace);
        assert_eq!(lexer.location(), (0,2));
        assert_eq!(lexer.next().unwrap(), Token::Alphanumeric("abc".to_string()));
        assert_eq!(lexer.location(), (0,5));
        assert_eq!(lexer.next().unwrap(), Token::EOL);
        assert_eq!(lexer.location(), (1,0));
        assert_eq!(lexer.next().unwrap(), Token::Integer(32));
        assert_eq!(lexer.location(), (1,2));
        assert_eq!(lexer.next().unwrap(), Token::Whitespace);
        assert_eq!(lexer.location(), (1,3));
        assert_eq!(lexer.next().unwrap(), Token::Alphanumeric("ewq".to_string()));
        assert_eq!(lexer.location(), (1,6));
        assert_eq!(lexer.next().unwrap(), Token::EOF);
    }

    #[test]
    fn whitespace() {
        lex_and_assert(
            "\t \t",
            vec![Token::Whitespace, Token::EOF],
        );
    }

    #[test]
    fn comma() {
        lex_and_assert(
            ",",
            vec![Token::Comma, Token::EOF],
        );
    }

    #[test]
    fn colon() {
        lex_and_assert(
            ":",
            vec![Token::Colon, Token::EOF],
        );
    }

    #[test]
    fn semicolon() {
        lex_and_assert(
            ";",
            vec![Token::Semicolon, Token::EOF],
        );
    }

    #[test]
    fn integer() {
        lex_and_assert(
            "321",
            vec![Token::Integer(321), Token::EOF],
        );
    }

    #[test]
    fn alphanumeric() {
        lex_and_assert(
            "tJKo32Ii",
            vec![Token::Alphanumeric("tJKo32Ii".to_string()), Token::EOF],
        );
    }

    #[test]
    fn symbol() {
        lex_and_assert(
            "(#'",
            vec![Token::Symbol('('), Token::Symbol('#'), Token::Symbol('\''), Token::EOF],
        );
    }

    #[test]
    fn unknown() {
        lex_and_assert(
            "\x02",
            vec![Token::Unknown(0x02), Token::EOF],
        );
    }

    #[test]
    fn newline() {
        lex_and_assert(
            "\n\r\n",
            vec![
                Token::EOL,
                Token::EOL,
                Token::EOF,
            ],
        );
    }

    #[test]
    fn instruction() {
        lex_and_assert(
            "mov r1, 24",
            vec![
                Token::Alphanumeric("mov".to_string()),
                Token::Whitespace,
                Token::Alphanumeric("r1".to_string()),
                Token::Comma,
                Token::Whitespace,
                Token::Integer(24),
                Token::EOF,
            ],
        );
    }

    #[test]
    fn label_instruction() {
        lex_and_assert(
            "label:\n\tmov r1, 24",
            vec![
                Token::Alphanumeric("label".to_string()),
                Token::Colon,
                Token::EOL,
                Token::Whitespace,
                Token::Alphanumeric("mov".to_string()),
                Token::Whitespace,
                Token::Alphanumeric("r1".to_string()),
                Token::Comma,
                Token::Whitespace,
                Token::Integer(24),
                Token::EOF,
            ],
        );
    }

    #[test]
    fn instruction_comment() {
        lex_and_assert(
            "mov r1, 24 ; comment",
            vec![
                Token::Alphanumeric("mov".to_string()),
                Token::Whitespace,
                Token::Alphanumeric("r1".to_string()),
                Token::Comma,
                Token::Whitespace,
                Token::Integer(24),
                Token::Whitespace,
                Token::Semicolon,
                Token::Whitespace,
                Token::Alphanumeric("comment".to_string()),
                Token::EOF,
            ],
        );
    }

    #[test]
    fn comment() {
        lex_and_assert(
            "; something else",
            vec![
                Token::Semicolon,
                Token::Whitespace,
                Token::Alphanumeric("something".to_string()),
                Token::Whitespace,
                Token::Alphanumeric("else".to_string()),
                Token::EOF,
            ],
        );
    }

    #[test]
    fn multiline() {
        lex_and_assert(
            "; comment\njmp 321",
            vec![
                Token::Semicolon,
                Token::Whitespace,
                Token::Alphanumeric("comment".to_string()),
                Token::EOL,
                Token::Alphanumeric("jmp".to_string()),
                Token::Whitespace,
                Token::Integer(321),
                Token::EOF,
            ],
        );
    }
}

