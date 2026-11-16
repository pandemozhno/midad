use std::fmt;

#[derive(Debug, Clone)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    Lexer,
    Parser,
    Lowering,
    Checking,
    Backend,
    Io,
}

impl Error {
    pub fn new(kind: ErrorKind, message: impl Into<String>, line: usize, column: usize) -> Self {
        Self { kind, message: message.into(), line, column }
    }
    pub fn lexer(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self::new(ErrorKind::Lexer, message, line, column)
    }
    pub fn parser(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self::new(ErrorKind::Parser, message, line, column)
    }
    pub fn lowering(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self::new(ErrorKind::Lowering, message, line, column)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{:?}] {}:{}: {}", self.kind, self.line, self.column, self.message)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;