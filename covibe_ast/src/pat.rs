//! Pattern AST nodes for pattern matching and destructuring.

use super::*;
use crate::literal::Literal;
use covibe_util::span::Span;

/// A pattern used in let bindings, function parameters, match arms, etc.
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    /// The node ID.
    pub id: NodeId,
    /// The kind of pattern.
    pub kind: PatternKind,
    /// The span of this pattern.
    pub span: Span,
}

impl Pattern {
    /// Creates a new pattern.
    pub fn new(id: NodeId, kind: PatternKind, span: Span) -> Self {
        Self { id, kind, span }
    }
}

/// The kind of pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum PatternKind {
    /// Wildcard pattern (`_`).
    Wildcard,

    /// Rest pattern (`...`) for capturing remaining elements.
    Rest,

    /// Identifier pattern (e.g., `x`, `mut x`).
    Ident {
        name: Ident,
        mutable: bool,
        /// Optional subpattern for `@` binding (e.g., `x @ Point { .. }`).
        subpattern: Option<Box<Pattern>>,
    },

    /// Literal pattern (e.g., `42`, `"hello"`, `true`).
    Literal(Literal),

    /// Range pattern (e.g., `1..10`, `'a'..='z'`).
    Range {
        start: Box<Pattern>,
        end: Box<Pattern>,
        inclusive: bool,
    },

    /// Tuple pattern (e.g., `(x, y)`, `(1, _, z)`).
    Tuple(Vec<Pattern>),

    /// Struct pattern (e.g., `Point { x, y }`, `Color { r: 255, .. }`).
    Struct {
        path: Path,
        fields: Vec<FieldPattern>,
        /// Whether to ignore additional fields (`..`).
        ignore_rest: bool,
    },

    /// Tuple struct pattern (e.g., `Some(x)`, `Color(r, g, b)`).
    TupleStruct {
        path: Path,
        elements: Vec<Pattern>,
    },

    /// Unit struct pattern (e.g., `None`, `MyUnit`).
    UnitStruct(Path),

    /// Array/slice pattern (e.g., `[a, b, c]`, `[head, ...tail]`).
    Array(Vec<Pattern>),

    /// Or pattern (e.g., `1 | 2 | 3`, `Some(x) | None`).
    Or(Vec<Pattern>),

    /// Parenthesized pattern.
    Paren(Box<Pattern>),

    /// Reference pattern (e.g., `&x`, `&mut y`).
    Ref {
        pattern: Box<Pattern>,
        mutable: bool,
    },

    /// Box pattern (e.g., `box x`).
    Box(Box<Pattern>),

    /// Type-annotated pattern (e.g., `x: int`, `Point { x, y }: Point`).
    Type {
        pattern: Box<Pattern>,
        ty: Type,
    },

    /// Path pattern (for enum variants without data, e.g., `Option::None`).
    Path(Path),

    /// Macro pattern (e.g., `matches!(x, ...)`).
    Macro {
        path: Path,
        args: Vec<Pattern>,
    },

    /// Guard pattern (pattern with a condition, used internally).
    Guard {
        pattern: Box<Pattern>,
        condition: Box<crate::expr::Expr>,
    },

    /// Error recovery placeholder.
    Error,
}

/// A field pattern in a struct pattern.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldPattern {
    /// The field name.
    pub name: Ident,
    /// The pattern for this field. If None, uses shorthand (e.g., `Point { x, y }`).
    pub pattern: Option<Pattern>,
    /// The span of this field pattern.
    pub span: Span,
}

impl FieldPattern {
    /// Creates a new field pattern.
    pub fn new(name: Ident, pattern: Option<Pattern>, span: Span) -> Self {
        Self {
            name,
            pattern,
            span,
        }
    }

    /// Returns true if this uses shorthand syntax.
    pub fn is_shorthand(&self) -> bool {
        self.pattern.is_none()
    }
}
