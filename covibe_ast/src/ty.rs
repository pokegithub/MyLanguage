//! Type expression AST nodes.

use super::*;
use covibe_util::span::Span;

/// A type expression.
#[derive(Debug, Clone, PartialEq)]
pub struct Type {
    /// The node ID.
    pub id: NodeId,
    /// The kind of type.
    pub kind: TypeKind,
    /// The span of this type.
    pub span: Span,
}

impl Type {
    /// Creates a new type.
    pub fn new(id: NodeId, kind: TypeKind, span: Span) -> Self {
        Self { id, kind, span }
    }

    /// Creates a dummy type for error recovery.
    pub fn error(span: Span) -> Self {
        Self {
            id: NodeId::DUMMY,
            kind: TypeKind::Error,
            span,
        }
    }
}

/// The kind of type.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    /// A type path (e.g., `int`, `Vec<T>`, `std::io::Error`).
    Path(Path),

    /// Tuple type (e.g., `(int, str)`, `()`).
    Tuple(Vec<Type>),

    /// Array type with fixed size (e.g., `[int; 10]`).
    Array {
        element: Box<Type>,
        size: Box<crate::expr::Expr>,
    },

    /// Slice type (e.g., `[int]`).
    Slice(Box<Type>),

    /// Reference type (e.g., `&int`, `&mut str`, `&'a T`).
    Ref {
        lifetime: Option<Lifetime>,
        mutable: bool,
        inner: Box<Type>,
    },

    /// Raw pointer type (e.g., `*const int`, `*mut T`).
    Pointer {
        mutable: bool,
        inner: Box<Type>,
    },

    /// Function type (e.g., `def(int, str) -> bool`).
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
        /// Whether this is an async function.
        is_async: bool,
    },

    /// Never type (`!`) - the type of expressions that never return.
    Never,

    /// Inferred type (`_`) - type to be inferred by the compiler.
    Infer,

    /// Union type (e.g., `int | str`, `Option<T> | Error`).
    Union(Vec<Type>),

    /// Intersection type (e.g., `T & Clone & Display`).
    Intersection(Vec<Type>),

    /// Trait object type (e.g., `dyn Display`, `dyn Iterator<Item=int>`).
    TraitObject {
        bounds: Vec<TraitBound>,
        lifetime: Option<Lifetime>,
    },

    /// Impl trait type (e.g., `impl Display`, `impl Iterator<Item=int>`).
    ImplTrait(Vec<TraitBound>),

    /// Parenthesized type.
    Paren(Box<Type>),

    /// Typeof type (gets the type of an expression, e.g., `typeof(x)`).
    Typeof(Box<crate::expr::Expr>),

    /// Refinement type (e.g., `int { x: x > 0 }`).
    Refinement {
        base: Box<Type>,
        predicate: RefinementPredicate,
    },

    /// Effect type (e.g., `T ! IO`, `Result<T, E> ! Async`).
    Effect {
        base: Box<Type>,
        effects: Vec<Effect>,
    },

    /// Linear type (must be used exactly once).
    Linear(Box<Type>),

    /// Opaque type (hides implementation details).
    Opaque {
        name: Ident,
        bounds: Vec<TraitBound>,
    },

    /// Associated type (e.g., `T::Item`, `Iterator::Item`).
    Associated {
        base: Box<Type>,
        ident: Ident,
    },

    /// Macro invocation in type position.
    Macro {
        path: Path,
        args: Vec<Type>,
    },

    /// Type variable (used during type inference, e.g., `'T`).
    Var(Ident),

    /// Error recovery placeholder.
    Error,
}

/// A refinement predicate for refinement types.
#[derive(Debug, Clone, PartialEq)]
pub struct RefinementPredicate {
    /// The variable name bound to the value (e.g., `x` in `int { x: x > 0 }`).
    pub var: Ident,
    /// The boolean predicate expression.
    pub condition: Box<crate::expr::Expr>,
    /// The span of this predicate.
    pub span: Span,
}

/// An effect annotation.
#[derive(Debug, Clone, PartialEq)]
pub struct Effect {
    /// The effect name (e.g., `IO`, `Async`, `Unsafe`).
    pub name: Path,
    /// The span of this effect.
    pub span: Span,
}

impl Effect {
    /// Creates a new effect.
    pub fn new(name: Path, span: Span) -> Self {
        Self { name, span }
    }
}
