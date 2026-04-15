//! Token definitions for the CoVibe lexer.

use covibe_util::span::Span;
use std::fmt;

/// A token produced by the lexer.
///
/// Each token consists of a kind (what type of token it is), a span indicating
/// its location in the source file, and optionally some associated data.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// The kind of token.
    pub kind: TokenKind,
    /// The location of the token in the source file.
    pub span: Span,
}

impl Token {
    /// Creates a new token with the given kind and span.
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }

    /// Returns true if this token is a keyword.
    pub fn is_keyword(&self) -> bool {
        matches!(
            self.kind,
            TokenKind::Def
                | TokenKind::Let
                | TokenKind::Var
                | TokenKind::Const
                | TokenKind::Struct
                | TokenKind::Enum
                | TokenKind::Trait
                | TokenKind::Impl
                | TokenKind::Type
                | TokenKind::Class
                | TokenKind::Interface
                | TokenKind::If
                | TokenKind::Elif
                | TokenKind::Else
                | TokenKind::Match
                | TokenKind::Case
                | TokenKind::For
                | TokenKind::While
                | TokenKind::Loop
                | TokenKind::Break
                | TokenKind::Continue
                | TokenKind::Return
                | TokenKind::Yield
                | TokenKind::Await
                | TokenKind::Int
                | TokenKind::Float
                | TokenKind::Bool
                | TokenKind::Str
                | TokenKind::Char
                | TokenKind::I8
                | TokenKind::I16
                | TokenKind::I32
                | TokenKind::I64
                | TokenKind::I128
                | TokenKind::ISize
                | TokenKind::U8
                | TokenKind::U16
                | TokenKind::U32
                | TokenKind::U64
                | TokenKind::U128
                | TokenKind::USize
                | TokenKind::F32
                | TokenKind::F64
                | TokenKind::Import
                | TokenKind::From
                | TokenKind::As
                | TokenKind::Export
                | TokenKind::Pub
                | TokenKind::Priv
                | TokenKind::Protected
                | TokenKind::Ref
                | TokenKind::Mut
                | TokenKind::Move
                | TokenKind::Copy
                | TokenKind::Clone
                | TokenKind::Box
                | TokenKind::Alloc
                | TokenKind::Defer
                | TokenKind::Drop
                | TokenKind::Static
                | TokenKind::Unsafe
                | TokenKind::Async
                | TokenKind::Spawn
                | TokenKind::Send
                | TokenKind::Recv
                | TokenKind::Select
                | TokenKind::To
                | TokenKind::Default
                | TokenKind::True
                | TokenKind::False
                | TokenKind::None
                | TokenKind::Null
                | TokenKind::And
                | TokenKind::Or
                | TokenKind::Not
                | TokenKind::In
                | TokenKind::Is
                | TokenKind::SelfLower
                | TokenKind::SelfUpper
                | TokenKind::Super
                | TokenKind::Where
                | TokenKind::With
                | TokenKind::Try
                | TokenKind::Catch
                | TokenKind::Finally
                | TokenKind::Raise
                | TokenKind::Throw
                | TokenKind::Assert
                | TokenKind::Lambda
                | TokenKind::Comptime
                | TokenKind::Macro
                | TokenKind::Extern
        )
    }

    /// Returns true if this token is an operator.
    pub fn is_operator(&self) -> bool {
        matches!(
            self.kind,
            TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::SlashSlash
                | TokenKind::Percent
                | TokenKind::StarStar
                | TokenKind::EqEq
                | TokenKind::BangEq
                | TokenKind::Lt
                | TokenKind::LtEq
                | TokenKind::Gt
                | TokenKind::GtEq
                | TokenKind::Spaceship
                | TokenKind::AndAnd
                | TokenKind::OrOr
                | TokenKind::Bang
                | TokenKind::Ampersand
                | TokenKind::Pipe
                | TokenKind::Caret
                | TokenKind::Tilde
                | TokenKind::LtLt
                | TokenKind::GtGt
                | TokenKind::GtGtGt
                | TokenKind::Eq
                | TokenKind::PlusEq
                | TokenKind::MinusEq
                | TokenKind::StarEq
                | TokenKind::SlashEq
                | TokenKind::SlashSlashEq
                | TokenKind::PercentEq
                | TokenKind::StarStarEq
                | TokenKind::AmpersandEq
                | TokenKind::PipeEq
                | TokenKind::CaretEq
                | TokenKind::LtLtEq
                | TokenKind::GtGtEq
                | TokenKind::GtGtGtEq
        )
    }

    /// Returns true if this token is a literal.
    pub fn is_literal(&self) -> bool {
        matches!(
            self.kind,
            TokenKind::Literal(_) | TokenKind::True | TokenKind::False | TokenKind::None
        )
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

/// The kind of token.
///
/// This enum represents all possible token types in the CoVibe language.
/// It includes keywords, operators, punctuation, literals, and special tokens
/// for controlling indentation-based syntax.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Special tokens
    /// End of file
    Eof,
    /// Increase in indentation level
    Indent,
    /// Decrease in indentation level
    Dedent,
    /// Newline (significant for statement separation)
    Newline,

    // Identifiers and literals
    /// Identifier (variable name, function name, etc.)
    Ident(String),
    /// Literal value (number, string, etc.)
    Literal(Literal),

    // Keywords - Declaration
    /// `def` keyword
    Def,
    /// `let` keyword
    Let,
    /// `var` keyword
    Var,
    /// `const` keyword
    Const,
    /// `struct` keyword
    Struct,
    /// `enum` keyword
    Enum,
    /// `trait` keyword
    Trait,
    /// `impl` keyword
    Impl,
    /// `type` keyword
    Type,
    /// `class` keyword
    Class,
    /// `interface` keyword
    Interface,

    // Keywords - Control flow
    /// `if` keyword
    If,
    /// `elif` keyword
    Elif,
    /// `else` keyword
    Else,
    /// `match` keyword
    Match,
    /// `case` keyword
    Case,
    /// `for` keyword
    For,
    /// `while` keyword
    While,
    /// `loop` keyword
    Loop,
    /// `break` keyword
    Break,
    /// `continue` keyword
    Continue,
    /// `return` keyword
    Return,
    /// `yield` keyword
    Yield,
    /// `await` keyword
    Await,

    // Keywords - Type keywords
    /// `int` keyword
    Int,
    /// `float` keyword
    Float,
    /// `bool` keyword
    Bool,
    /// `str` keyword
    Str,
    /// `char` keyword
    Char,
    /// `i8` keyword
    I8,
    /// `i16` keyword
    I16,
    /// `i32` keyword
    I32,
    /// `i64` keyword
    I64,
    /// `i128` keyword
    I128,
    /// `isize` keyword
    ISize,
    /// `u8` keyword
    U8,
    /// `u16` keyword
    U16,
    /// `u32` keyword
    U32,
    /// `u64` keyword
    U64,
    /// `u128` keyword
    U128,
    /// `usize` keyword
    USize,
    /// `f32` keyword
    F32,
    /// `f64` keyword
    F64,

    // Keywords - Module and visibility
    /// `import` keyword
    Import,
    /// `from` keyword
    From,
    /// `as` keyword
    As,
    /// `export` keyword
    Export,
    /// `pub` keyword
    Pub,
    /// `priv` keyword
    Priv,
    /// `protected` keyword
    Protected,

    // Keywords - Memory and ownership
    /// `ref` keyword
    Ref,
    /// `mut` keyword
    Mut,
    /// `move` keyword
    Move,
    /// `copy` keyword
    Copy,
    /// `clone` keyword
    Clone,
    /// `box` keyword
    Box,
    /// `alloc` keyword
    Alloc,
    /// `defer` keyword
    Defer,
    /// `drop` keyword
    Drop,
    /// `static` keyword
    Static,
    /// `unsafe` keyword
    Unsafe,

    // Keywords - Concurrency
    /// `async` keyword
    Async,
    /// `spawn` keyword
    Spawn,
    /// `send` keyword
    Send,
    /// `recv` keyword
    Recv,
    /// `select` keyword
    Select,
    /// `to` keyword (for select send operations)
    To,
    /// `default` keyword (for select default case)
    Default,

    // Keywords - Boolean and special literals
    /// `true` keyword
    True,
    /// `false` keyword
    False,
    /// `none` keyword
    None,
    /// `null` keyword
    Null,

    // Keywords - Operator keywords
    /// `and` keyword
    And,
    /// `or` keyword
    Or,
    /// `not` keyword
    Not,
    /// `in` keyword
    In,
    /// `is` keyword
    Is,

    // Keywords - Other
    /// `self` keyword
    SelfLower,
    /// `Self` keyword
    SelfUpper,
    /// `super` keyword
    Super,
    /// `where` keyword
    Where,
    /// `with` keyword
    With,
    /// `try` keyword
    Try,
    /// `catch` keyword
    Catch,
    /// `finally` keyword
    Finally,
    /// `raise` keyword
    Raise,
    /// `throw` keyword (alias for raise)
    Throw,
    /// `assert` keyword
    Assert,
    /// `lambda` keyword
    Lambda,
    /// `comptime` keyword
    Comptime,
    /// `macro` keyword
    Macro,
    /// `extern` keyword
    Extern,

    // Arithmetic operators
    /// `+` operator
    Plus,
    /// `-` operator
    Minus,
    /// `*` operator
    Star,
    /// `/` operator
    Slash,
    /// `//` operator (integer division)
    SlashSlash,
    /// `%` operator
    Percent,
    /// `**` operator (exponentiation)
    StarStar,

    // Comparison operators
    /// `==` operator
    EqEq,
    /// `!=` operator
    BangEq,
    /// `<` operator
    Lt,
    /// `<=` operator
    LtEq,
    /// `>` operator
    Gt,
    /// `>=` operator
    GtEq,
    /// `<=>` operator (spaceship/three-way comparison)
    Spaceship,

    // Logical operators
    /// `&&` operator
    AndAnd,
    /// `||` operator
    OrOr,
    /// `!` operator
    Bang,

    // Bitwise operators
    /// `&` operator
    Ampersand,
    /// `|` operator
    Pipe,
    /// `^` operator
    Caret,
    /// `~` operator
    Tilde,
    /// `<<` operator
    LtLt,
    /// `>>` operator
    GtGt,
    /// `>>>` operator (unsigned right shift)
    GtGtGt,

    // Assignment operators
    /// `=` operator
    Eq,
    /// `+=` operator
    PlusEq,
    /// `-=` operator
    MinusEq,
    /// `*=` operator
    StarEq,
    /// `/=` operator
    SlashEq,
    /// `//=` operator
    SlashSlashEq,
    /// `%=` operator
    PercentEq,
    /// `**=` operator
    StarStarEq,
    /// `&=` operator
    AmpersandEq,
    /// `|=` operator
    PipeEq,
    /// `^=` operator
    CaretEq,
    /// `<<=` operator
    LtLtEq,
    /// `>>=` operator
    GtGtEq,
    /// `>>>=` operator
    GtGtGtEq,

    // Other operators
    /// `->` operator (function return type)
    Arrow,
    /// `=>` operator (match arm)
    FatArrow,
    /// `..` operator (exclusive range)
    DotDot,
    /// `..=` operator (inclusive range)
    DotDotEq,
    /// `...` operator (variadic, spread)
    DotDotDot,
    /// `?` operator (optional chaining, error propagation)
    Question,
    /// `??` operator (null coalescing)
    QuestionQuestion,
    /// `?:` operator (ternary)
    QuestionColon,
    /// `::` operator (path separator)
    ColonColon,
    /// `@` symbol (decorator, raw identifier)
    At,
    /// `$` symbol (macro variable)
    Dollar,
    /// `|>` operator (pipe)
    PipeGt,
    /// `<|` operator (reverse pipe)
    LtPipe,

    // Punctuation
    /// `(` left parenthesis
    LParen,
    /// `)` right parenthesis
    RParen,
    /// `[` left square bracket
    LBracket,
    /// `]` right square bracket
    RBracket,
    /// `{` left curly brace
    LBrace,
    /// `}` right curly brace
    RBrace,
    /// `,` comma
    Comma,
    /// `.` dot
    Dot,
    /// `:` colon
    Colon,
    /// `;` semicolon
    Semicolon,
    /// `#` hash
    Hash,

    // Walrus operator
    /// `:=` operator (walrus/inline assignment)
    ColonEq,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Eof => write!(f, "end of file"),
            TokenKind::Indent => write!(f, "INDENT"),
            TokenKind::Dedent => write!(f, "DEDENT"),
            TokenKind::Newline => write!(f, "NEWLINE"),
            TokenKind::Ident(name) => write!(f, "identifier '{}'", name),
            TokenKind::Literal(lit) => write!(f, "{}", lit),
            TokenKind::Def => write!(f, "'def'"),
            TokenKind::Let => write!(f, "'let'"),
            TokenKind::Var => write!(f, "'var'"),
            TokenKind::Const => write!(f, "'const'"),
            TokenKind::Struct => write!(f, "'struct'"),
            TokenKind::Enum => write!(f, "'enum'"),
            TokenKind::Trait => write!(f, "'trait'"),
            TokenKind::Impl => write!(f, "'impl'"),
            TokenKind::Type => write!(f, "'type'"),
            TokenKind::Class => write!(f, "'class'"),
            TokenKind::Interface => write!(f, "'interface'"),
            TokenKind::If => write!(f, "'if'"),
            TokenKind::Elif => write!(f, "'elif'"),
            TokenKind::Else => write!(f, "'else'"),
            TokenKind::Match => write!(f, "'match'"),
            TokenKind::Case => write!(f, "'case'"),
            TokenKind::For => write!(f, "'for'"),
            TokenKind::While => write!(f, "'while'"),
            TokenKind::Loop => write!(f, "'loop'"),
            TokenKind::Break => write!(f, "'break'"),
            TokenKind::Continue => write!(f, "'continue'"),
            TokenKind::Return => write!(f, "'return'"),
            TokenKind::Yield => write!(f, "'yield'"),
            TokenKind::Await => write!(f, "'await'"),
            TokenKind::Int => write!(f, "'int'"),
            TokenKind::Float => write!(f, "'float'"),
            TokenKind::Bool => write!(f, "'bool'"),
            TokenKind::Str => write!(f, "'str'"),
            TokenKind::Char => write!(f, "'char'"),
            TokenKind::I8 => write!(f, "'i8'"),
            TokenKind::I16 => write!(f, "'i16'"),
            TokenKind::I32 => write!(f, "'i32'"),
            TokenKind::I64 => write!(f, "'i64'"),
            TokenKind::I128 => write!(f, "'i128'"),
            TokenKind::ISize => write!(f, "'isize'"),
            TokenKind::U8 => write!(f, "'u8'"),
            TokenKind::U16 => write!(f, "'u16'"),
            TokenKind::U32 => write!(f, "'u32'"),
            TokenKind::U64 => write!(f, "'u64'"),
            TokenKind::U128 => write!(f, "'u128'"),
            TokenKind::USize => write!(f, "'usize'"),
            TokenKind::F32 => write!(f, "'f32'"),
            TokenKind::F64 => write!(f, "'f64'"),
            TokenKind::Import => write!(f, "'import'"),
            TokenKind::From => write!(f, "'from'"),
            TokenKind::As => write!(f, "'as'"),
            TokenKind::Export => write!(f, "'export'"),
            TokenKind::Pub => write!(f, "'pub'"),
            TokenKind::Priv => write!(f, "'priv'"),
            TokenKind::Protected => write!(f, "'protected'"),
            TokenKind::Ref => write!(f, "'ref'"),
            TokenKind::Mut => write!(f, "'mut'"),
            TokenKind::Move => write!(f, "'move'"),
            TokenKind::Copy => write!(f, "'copy'"),
            TokenKind::Clone => write!(f, "'clone'"),
            TokenKind::Box => write!(f, "'box'"),
            TokenKind::Alloc => write!(f, "'alloc'"),
            TokenKind::Defer => write!(f, "'defer'"),
            TokenKind::Drop => write!(f, "'drop'"),
            TokenKind::Static => write!(f, "'static'"),
            TokenKind::Unsafe => write!(f, "'unsafe'"),
            TokenKind::Async => write!(f, "'async'"),
            TokenKind::Spawn => write!(f, "'spawn'"),
            TokenKind::Send => write!(f, "'send'"),
            TokenKind::Recv => write!(f, "'recv'"),
            TokenKind::Select => write!(f, "'select'"),
            TokenKind::To => write!(f, "'to'"),
            TokenKind::Default => write!(f, "'default'"),
            TokenKind::True => write!(f, "'true'"),
            TokenKind::False => write!(f, "'false'"),
            TokenKind::None => write!(f, "'none'"),
            TokenKind::Null => write!(f, "'null'"),
            TokenKind::And => write!(f, "'and'"),
            TokenKind::Or => write!(f, "'or'"),
            TokenKind::Not => write!(f, "'not'"),
            TokenKind::In => write!(f, "'in'"),
            TokenKind::Is => write!(f, "'is'"),
            TokenKind::SelfLower => write!(f, "'self'"),
            TokenKind::SelfUpper => write!(f, "'Self'"),
            TokenKind::Super => write!(f, "'super'"),
            TokenKind::Where => write!(f, "'where'"),
            TokenKind::With => write!(f, "'with'"),
            TokenKind::Try => write!(f, "'try'"),
            TokenKind::Catch => write!(f, "'catch'"),
            TokenKind::Finally => write!(f, "'finally'"),
            TokenKind::Raise => write!(f, "'raise'"),
            TokenKind::Throw => write!(f, "'throw'"),
            TokenKind::Assert => write!(f, "'assert'"),
            TokenKind::Lambda => write!(f, "'lambda'"),
            TokenKind::Comptime => write!(f, "'comptime'"),
            TokenKind::Macro => write!(f, "'macro'"),
            TokenKind::Extern => write!(f, "'extern'"),
            TokenKind::Plus => write!(f, "'+'"),
            TokenKind::Minus => write!(f, "'-'"),
            TokenKind::Star => write!(f, "'*'"),
            TokenKind::Slash => write!(f, "'/'"),
            TokenKind::SlashSlash => write!(f, "'//'"),
            TokenKind::Percent => write!(f, "'%'"),
            TokenKind::StarStar => write!(f, "'**'"),
            TokenKind::EqEq => write!(f, "'=='"),
            TokenKind::BangEq => write!(f, "'!='"),
            TokenKind::Lt => write!(f, "'<'"),
            TokenKind::LtEq => write!(f, "'<='"),
            TokenKind::Gt => write!(f, "'>'"),
            TokenKind::GtEq => write!(f, "'>='"),
            TokenKind::Spaceship => write!(f, "'<=>'"),
            TokenKind::AndAnd => write!(f, "'&&'"),
            TokenKind::OrOr => write!(f, "'||'"),
            TokenKind::Bang => write!(f, "'!'"),
            TokenKind::Ampersand => write!(f, "'&'"),
            TokenKind::Pipe => write!(f, "'|'"),
            TokenKind::Caret => write!(f, "'^'"),
            TokenKind::Tilde => write!(f, "'~'"),
            TokenKind::LtLt => write!(f, "'<<'"),
            TokenKind::GtGt => write!(f, "'>>'"),
            TokenKind::GtGtGt => write!(f, "'>>>'"),
            TokenKind::Eq => write!(f, "'='"),
            TokenKind::PlusEq => write!(f, "'+='"),
            TokenKind::MinusEq => write!(f, "'-='"),
            TokenKind::StarEq => write!(f, "'*='"),
            TokenKind::SlashEq => write!(f, "'/='"),
            TokenKind::SlashSlashEq => write!(f, "'//='"),
            TokenKind::PercentEq => write!(f, "'%='"),
            TokenKind::StarStarEq => write!(f, "'**='"),
            TokenKind::AmpersandEq => write!(f, "'&='"),
            TokenKind::PipeEq => write!(f, "'|='"),
            TokenKind::CaretEq => write!(f, "'^='"),
            TokenKind::LtLtEq => write!(f, "'<<='"),
            TokenKind::GtGtEq => write!(f, "'>>='"),
            TokenKind::GtGtGtEq => write!(f, "'>>>='"),
            TokenKind::Arrow => write!(f, "'->'"),
            TokenKind::FatArrow => write!(f, "'=>'"),
            TokenKind::DotDot => write!(f, "'..'"),
            TokenKind::DotDotEq => write!(f, "'..='"),
            TokenKind::DotDotDot => write!(f, "'...'"),
            TokenKind::Question => write!(f, "'?'"),
            TokenKind::QuestionQuestion => write!(f, "'??'"),
            TokenKind::QuestionColon => write!(f, "'?:'"),
            TokenKind::ColonColon => write!(f, "'::'"),
            TokenKind::At => write!(f, "'@'"),
            TokenKind::Dollar => write!(f, "'$'"),
            TokenKind::PipeGt => write!(f, "'|>'"),
            TokenKind::LtPipe => write!(f, "'<|'"),
            TokenKind::LParen => write!(f, "'('"),
            TokenKind::RParen => write!(f, "')'"),
            TokenKind::LBracket => write!(f, "'['"),
            TokenKind::RBracket => write!(f, "']'"),
            TokenKind::LBrace => write!(f, "'{{'"),
            TokenKind::RBrace => write!(f, "'}}'"),
            TokenKind::Comma => write!(f, "','"),
            TokenKind::Dot => write!(f, "'.'"),
            TokenKind::Colon => write!(f, "':'"),
            TokenKind::Semicolon => write!(f, "';'"),
            TokenKind::Hash => write!(f, "'#'"),
            TokenKind::ColonEq => write!(f, "':='"),
        }
    }
}

/// Represents different types of literal values.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    /// Integer literal (value, suffix)
    Integer(String, Option<IntSuffix>),
    /// Float literal (value, suffix)
    Float(String, Option<FloatSuffix>),
    /// String literal
    String(String),
    /// Raw string literal (hash count, content)
    RawString(usize, String),
    /// Format string literal with interpolation positions
    FormatString(String),
    /// Heredoc string literal (triple-quoted multi-line string)
    Heredoc(String),
    /// Byte string literal
    ByteString(Vec<u8>),
    /// Character literal
    Char(char),
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::Integer(val, suffix) => {
                if let Some(s) = suffix {
                    write!(f, "integer literal {}{:?}", val, s)
                } else {
                    write!(f, "integer literal {}", val)
                }
            }
            Literal::Float(val, suffix) => {
                if let Some(s) = suffix {
                    write!(f, "float literal {}{:?}", val, s)
                } else {
                    write!(f, "float literal {}", val)
                }
            }
            Literal::String(s) => write!(f, "string literal \"{}\"", s),
            Literal::RawString(count, s) => write!(f, "raw string r{}\"{}\"", "#".repeat(*count), s),
            Literal::FormatString(s) => write!(f, "f-string f\"{}\"", s),
            Literal::Heredoc(s) => write!(f, "heredoc \"\"\"{}\"\"\"", s),
            Literal::ByteString(_) => write!(f, "byte string literal"),
            Literal::Char(c) => write!(f, "character literal '{}'", c),
        }
    }
}

/// Integer type suffix.
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

/// Float type suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatSuffix {
    F32,
    F64,
}
