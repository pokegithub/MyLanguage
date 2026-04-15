//! Lexical analysis for the CoVibe programming language.
//!
//! This module provides the lexer (also known as tokenizer or scanner) which
//! transforms a stream of UTF-8 encoded characters into a sequence of tokens.
//! The lexer handles:
//! - Whitespace and indentation tracking (significant indentation like Python)
//! - Comments (line, block, and documentation comments)
//! - Identifiers and keywords (including Unicode support)
//! - Numeric literals (integers and floats in multiple bases)
//! - String literals (plain, raw, f-strings, heredoc, byte strings)
//! - Operators and punctuation
//! - Error recovery and reporting

pub mod token;
pub mod lexer;

pub use token::{Token, TokenKind, Literal};
pub use lexer::Lexer;
