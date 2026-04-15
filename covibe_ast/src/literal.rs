//! Literal value representations in the AST.

use covibe_util::interner::Symbol;
use covibe_util::span::Span;

/// A literal value in the source code.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    /// Integer literal (e.g., `42`, `0xFF`, `0b1010`, `0o777`).
    Int(IntLit),
    /// Floating-point literal (e.g., `3.14`, `2.5e-3`).
    Float(FloatLit),
    /// String literal (e.g., `"hello"`, `f"x = {x}"`).
    Str(StrLit),
    /// Character literal (e.g., `'a'`, `'\n'`).
    Char(CharLit),
    /// Boolean literal (`true` or `false`).
    Bool(BoolLit),
    /// Byte literal (e.g., `b'A'`).
    Byte(ByteLit),
    /// Byte string literal (e.g., `b"hello"`).
    ByteStr(ByteStrLit),
}

impl Literal {
    /// Returns the span of this literal.
    pub fn span(&self) -> Span {
        match self {
            Literal::Int(lit) => lit.span,
            Literal::Float(lit) => lit.span,
            Literal::Str(lit) => lit.span,
            Literal::Char(lit) => lit.span,
            Literal::Bool(lit) => lit.span,
            Literal::Byte(lit) => lit.span,
            Literal::ByteStr(lit) => lit.span,
        }
    }
}

/// An integer literal.
#[derive(Debug, Clone, PartialEq)]
pub struct IntLit {
    /// The raw string representation of the integer (as it appears in source).
    pub raw: Symbol,
    /// The base of the integer (2, 8, 10, or 16).
    pub base: IntBase,
    /// Optional type suffix (e.g., `i32`, `u64`).
    pub suffix: Option<IntSuffix>,
    /// The span of this literal.
    pub span: Span,
}

/// The base of an integer literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntBase {
    /// Binary (0b...)
    Binary,
    /// Octal (0o...)
    Octal,
    /// Decimal (default)
    Decimal,
    /// Hexadecimal (0x...)
    Hexadecimal,
}

/// Type suffix for integer literals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntSuffix {
    I8,
    I16,
    I32,
    I64,
    I128,
    ISize,
    U8,
    U16,
    U32,
    U64,
    U128,
    USize,
}

/// A floating-point literal.
#[derive(Debug, Clone, PartialEq)]
pub struct FloatLit {
    /// The raw string representation of the float.
    pub raw: Symbol,
    /// Optional type suffix (e.g., `f32`, `f64`).
    pub suffix: Option<FloatSuffix>,
    /// The span of this literal.
    pub span: Span,
}

/// Type suffix for floating-point literals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatSuffix {
    F32,
    F64,
}

/// A string literal.
#[derive(Debug, Clone, PartialEq)]
pub struct StrLit {
    /// The kind of string literal.
    pub kind: StrKind,
    /// The string content (after escape processing, if applicable).
    pub value: Symbol,
    /// For f-strings, the interpolation parts.
    pub parts: Vec<StrPart>,
    /// The span of this literal.
    pub span: Span,
}

/// The kind of string literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrKind {
    /// Normal string with escape sequences processed.
    Normal,
    /// Raw string (no escape processing).
    Raw,
    /// Format string (f-string) with interpolation.
    Format,
    /// Heredoc (multi-line string).
    Heredoc,
}

/// A part of an f-string (format string).
#[derive(Debug, Clone, PartialEq)]
pub enum StrPart {
    /// A literal string portion.
    Literal(Symbol, Span),
    /// An interpolated expression: `{expr}` or `{expr:format_spec}`.
    Interpolation {
        /// The expression to interpolate.
        expr: Box<crate::expr::Expr>,
        /// Optional format specification.
        format_spec: Option<Symbol>,
        /// Span of the entire interpolation (including braces).
        span: Span,
    },
}

/// A character literal.
#[derive(Debug, Clone, PartialEq)]
pub struct CharLit {
    /// The character value.
    pub value: char,
    /// The span of this literal.
    pub span: Span,
}

/// A boolean literal.
#[derive(Debug, Clone, PartialEq)]
pub struct BoolLit {
    /// The boolean value.
    pub value: bool,
    /// The span of this literal.
    pub span: Span,
}

/// A byte literal.
#[derive(Debug, Clone, PartialEq)]
pub struct ByteLit {
    /// The byte value.
    pub value: u8,
    /// The span of this literal.
    pub span: Span,
}

/// A byte string literal.
#[derive(Debug, Clone, PartialEq)]
pub struct ByteStrLit {
    /// The byte string value.
    pub value: Vec<u8>,
    /// The span of this literal.
    pub span: Span,
}
