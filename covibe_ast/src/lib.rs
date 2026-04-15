//! Abstract Syntax Tree (AST) definitions for the CoVibe programming language.
//!
//! This module defines the complete AST node hierarchy for CoVibe, including:
//! - Expressions: All expression forms from literals to complex operations
//! - Statements: Control flow, assignments, and declarations
//! - Declarations: Functions, types, structs, enums, traits, impls
//! - Types: Type expressions and annotations
//! - Patterns: Pattern matching constructs
//!
//! Each AST node carries a `Span` indicating its source location for error reporting.

pub mod decl;
pub mod expr;
pub mod literal;
pub mod op;
pub mod pat;
pub mod stmt;
pub mod ty;
pub mod visitor;

use covibe_util::interner::Symbol;
use covibe_util::span::{BytePos, Span};

// Re-export commonly used types
pub use decl::*;
pub use expr::*;
pub use literal::*;
pub use op::*;
pub use pat::*;
pub use stmt::*;
pub use ty::*;
pub use visitor::*;

/// A unique identifier for an AST node.
///
/// Node IDs are assigned during parsing and used for:
/// - Mapping AST nodes to their resolved types
/// - Tracking def/use relationships in name resolution
/// - Associating diagnostics with specific nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(u32);

impl NodeId {
    /// A dummy node ID used as a placeholder.
    pub const DUMMY: NodeId = NodeId(0);

    /// Creates a new NodeId.
    pub const fn new(id: u32) -> Self {
        NodeId(id)
    }

    /// Returns the underlying u32 value.
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

impl From<u32> for NodeId {
    fn from(id: u32) -> Self {
        NodeId(id)
    }
}

impl From<usize> for NodeId {
    fn from(id: usize) -> Self {
        NodeId(id as u32)
    }
}

/// Generator for unique NodeIds.
#[derive(Debug, Default)]
pub struct NodeIdGen {
    next_id: u32,
}

impl NodeIdGen {
    /// Creates a new NodeIdGen starting from 1 (0 is reserved for DUMMY).
    pub fn new() -> Self {
        Self { next_id: 1 }
    }

    /// Generates the next unique NodeId.
    pub fn next(&mut self) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id = self.next_id.checked_add(1).expect("NodeId overflow");
        id
    }
}

/// An identifier in the AST.
///
/// Identifiers are represented as interned symbols for efficient
/// comparison and reduced memory usage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ident {
    /// The interned symbol representing this identifier.
    pub symbol: Symbol,
    /// The source span of this identifier.
    pub span: Span,
}

impl Ident {
    /// Creates a new identifier.
    pub fn new(symbol: Symbol, span: Span) -> Self {
        Self { symbol, span }
    }

    /// Creates a dummy identifier (for testing or placeholder purposes).
    pub fn dummy() -> Self {
        Self {
            symbol: Symbol::INVALID,
            span: Span::new(BytePos::ZERO, BytePos::ZERO),
        }
    }
}

/// A path for referencing types, modules, or values.
///
/// Examples: `std::collections::Vec`, `MyModule::MyType`, `value`
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    /// The segments of the path.
    pub segments: Vec<PathSegment>,
    /// The span of the entire path.
    pub span: Span,
}

impl Path {
    /// Creates a new path.
    pub fn new(segments: Vec<PathSegment>, span: Span) -> Self {
        Self { segments, span }
    }

    /// Creates a path from a single identifier.
    pub fn from_ident(ident: Ident) -> Self {
        Self {
            segments: vec![PathSegment::new(ident, None)],
            span: ident.span,
        }
    }

    /// Returns true if this path has only one segment (simple name).
    pub fn is_simple(&self) -> bool {
        self.segments.len() == 1
    }

    /// Returns the last segment of the path.
    pub fn last_segment(&self) -> Option<&PathSegment> {
        self.segments.last()
    }
}

/// A segment in a path, potentially with generic arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct PathSegment {
    /// The identifier for this segment.
    pub ident: Ident,
    /// Optional generic arguments.
    pub args: Option<GenericArgs>,
}

impl PathSegment {
    /// Creates a new path segment.
    pub fn new(ident: Ident, args: Option<GenericArgs>) -> Self {
        Self { ident, args }
    }
}

/// Generic arguments in a path segment.
///
/// Example: `Vec<int>`, `Result<T, Error>`, `Array<int, 10>`
#[derive(Debug, Clone, PartialEq)]
pub struct GenericArgs {
    /// Type arguments.
    pub types: Vec<Type>,
    /// Const arguments (for const generics).
    pub consts: Vec<Expr>,
    /// The span of the generic arguments (including angle brackets).
    pub span: Span,
}

impl GenericArgs {
    /// Creates new generic arguments.
    pub fn new(types: Vec<Type>, consts: Vec<Expr>, span: Span) -> Self {
        Self {
            types,
            consts,
            span,
        }
    }
}

/// A lifetime parameter.
///
/// Example: `'a`, `'static`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Lifetime {
    /// The name of the lifetime (without the leading apostrophe).
    pub name: Symbol,
    /// The span of the lifetime.
    pub span: Span,
}

impl Lifetime {
    /// Creates a new lifetime.
    pub fn new(name: Symbol, span: Span) -> Self {
        Self { name, span }
    }

    /// The special 'static lifetime.
    pub fn static_lifetime(span: Span) -> Self {
        // Note: The actual "static" symbol should be interned by the interner
        // This uses INVALID as a placeholder; real usage requires proper interning
        Self {
            name: Symbol::INVALID,
            span,
        }
    }
}

/// Visibility modifier for items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Public (visible everywhere).
    Public,
    /// Private (visible only in current module).
    Private,
    /// Protected (visible in current module and submodules).
    Protected,
    /// Visible within a specific path (e.g., `pub(crate)`, `pub(super)`).
    Restricted(Span), // Span points to the path restriction
}

impl Visibility {
    /// Returns true if this is public visibility.
    pub fn is_public(&self) -> bool {
        matches!(self, Visibility::Public)
    }

    /// Returns true if this is private visibility.
    pub fn is_private(&self) -> bool {
        matches!(self, Visibility::Private)
    }
}

impl Default for Visibility {
    fn default() -> Self {
        Visibility::Private
    }
}

/// Mutability modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mutability {
    /// Immutable (default).
    Immutable,
    /// Mutable (requires explicit `mut` keyword).
    Mutable,
}

impl Mutability {
    /// Returns true if this is mutable.
    pub fn is_mutable(&self) -> bool {
        matches!(self, Mutability::Mutable)
    }
}

impl Default for Mutability {
    fn default() -> Self {
        Mutability::Immutable
    }
}

/// A generic parameter in a type or function declaration.
#[derive(Debug, Clone, PartialEq)]
pub enum GenericParam {
    /// A type parameter (e.g., `T`, `T: Display`).
    Type(TypeParam),
    /// A const parameter (e.g., `N: usize`).
    Const(ConstParam),
    /// A lifetime parameter (e.g., `'a`).
    Lifetime(LifetimeParam),
}

/// A type parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeParam {
    /// The node ID.
    pub id: NodeId,
    /// The name of the type parameter.
    pub name: Ident,
    /// Trait bounds (e.g., `T: Display + Clone`).
    pub bounds: Vec<TraitBound>,
    /// Default type (e.g., `T = int`).
    pub default: Option<Type>,
    /// The span of this type parameter.
    pub span: Span,
}

/// A const parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstParam {
    /// The node ID.
    pub id: NodeId,
    /// The name of the const parameter.
    pub name: Ident,
    /// The type of the const parameter.
    pub ty: Type,
    /// Default value.
    pub default: Option<Expr>,
    /// The span of this const parameter.
    pub span: Span,
}

/// A lifetime parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct LifetimeParam {
    /// The node ID.
    pub id: NodeId,
    /// The lifetime.
    pub lifetime: Lifetime,
    /// Lifetime bounds (e.g., `'a: 'b`).
    pub bounds: Vec<Lifetime>,
    /// The span of this lifetime parameter.
    pub span: Span,
}

/// A trait bound on a type parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct TraitBound {
    /// The trait path.
    pub path: Path,
    /// The span of this bound.
    pub span: Span,
}

/// A where clause predicate.
#[derive(Debug, Clone, PartialEq)]
pub enum WherePredicate {
    /// Type bound (e.g., `T: Display`).
    BoundPredicate {
        /// The type being constrained.
        ty: Type,
        /// The trait bounds.
        bounds: Vec<TraitBound>,
        /// The span of this predicate.
        span: Span,
    },
    /// Lifetime bound (e.g., `'a: 'b`).
    LifetimePredicate {
        /// The lifetime being constrained.
        lifetime: Lifetime,
        /// The bounds on the lifetime.
        bounds: Vec<Lifetime>,
        /// The span of this predicate.
        span: Span,
    },
}

/// A where clause.
#[derive(Debug, Clone, PartialEq)]
pub struct WhereClause {
    /// The predicates in the where clause.
    pub predicates: Vec<WherePredicate>,
    /// The span of the entire where clause.
    pub span: Span,
}

/// An attribute (annotation/decorator).
///
/// Examples: `@inline`, `@deprecated("use new_function instead")`, `@test`
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    /// The path of the attribute.
    pub path: Path,
    /// Arguments to the attribute.
    pub args: Vec<Expr>,
    /// The span of the attribute.
    pub span: Span,
}

/// A doc comment.
///
/// Doc comments are treated specially and attached to the following item.
#[derive(Debug, Clone, PartialEq)]
pub struct DocComment {
    /// The content of the doc comment (without leading markers).
    pub content: String,
    /// The span of the doc comment.
    pub span: Span,
}

/// The root of an AST: a module.
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    /// The node ID.
    pub id: NodeId,
    /// Items (top-level declarations) in this module.
    pub items: Vec<Item>,
    /// The span of the entire module.
    pub span: Span,
}

/// A top-level item in a module.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// The node ID.
    pub id: NodeId,
    /// Doc comments.
    pub docs: Vec<DocComment>,
    /// Attributes.
    pub attrs: Vec<Attribute>,
    /// Visibility.
    pub vis: Visibility,
    /// The kind of item.
    pub kind: ItemKind,
    /// The span of this item.
    pub span: Span,
}

/// The kind of a top-level item.
#[derive(Debug, Clone, PartialEq)]
pub enum ItemKind {
    /// A function declaration.
    Function(Function),
    /// A struct declaration.
    Struct(StructDecl),
    /// An enum declaration.
    Enum(EnumDecl),
    /// A trait declaration.
    Trait(TraitDecl),
    /// An impl block.
    Impl(ImplDecl),
    /// A type alias.
    TypeAlias(TypeAlias),
    /// A const declaration.
    Const(ConstDecl),
    /// A static declaration.
    Static(StaticDecl),
    /// An import declaration.
    Import(ImportDecl),
    /// An export declaration.
    Export(ExportDecl),
    /// An extern block.
    Extern(ExternBlock),
    /// A module declaration.
    Module(ModuleDecl),
    /// A macro declaration.
    Macro(MacroDecl),
}
