//! Error types for the parser.

use covibe_util::span::Span;
use std::fmt;

/// A parse error.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    /// The error message.
    pub message: String,
    /// The span where the error occurred.
    pub span: Span,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}

/// Result type for parsing operations.
pub type ParseResult<T> = Result<T, ParseError>;
