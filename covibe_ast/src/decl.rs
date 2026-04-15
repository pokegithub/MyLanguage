//! Declaration AST nodes.

use super::*;
use crate::expr::{Expr, FunctionParam};
use crate::stmt::Block;
use covibe_util::span::Span;

/// A function declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// The node ID.
    pub id: NodeId,
    /// The function name.
    pub name: Ident,
    /// Generic parameters.
    pub generics: Vec<GenericParam>,
    /// Function parameters.
    pub params: Vec<FunctionParam>,
    /// Return type (optional if inferred or returns unit).
    pub return_type: Option<Type>,
    /// Where clause.
    pub where_clause: Option<WhereClause>,
    /// Function body (None for extern/trait functions).
    pub body: Option<Block>,
    /// Whether this is an async function.
    pub is_async: bool,
    /// Whether this is unsafe.
    pub is_unsafe: bool,
    /// Whether this is a const function (can be evaluated at compile time).
    pub is_const: bool,
    /// Whether this is an extern function.
    pub is_extern: bool,
    /// Optional ABI for extern functions (e.g., "C", "system").
    pub abi: Option<Symbol>,
    /// The span of this function.
    pub span: Span,
}

/// A struct declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct StructDecl {
    /// The node ID.
    pub id: NodeId,
    /// The struct name.
    pub name: Ident,
    /// Generic parameters.
    pub generics: Vec<GenericParam>,
    /// Where clause.
    pub where_clause: Option<WhereClause>,
    /// The kind of struct (named fields, tuple, or unit).
    pub kind: StructKind,
    /// The span of this struct.
    pub span: Span,
}

/// The kind of struct.
#[derive(Debug, Clone, PartialEq)]
pub enum StructKind {
    /// Struct with named fields (e.g., `struct Point { x: int, y: int }`).
    Named(Vec<FieldDecl>),
    /// Tuple struct (e.g., `struct Color(u8, u8, u8)`).
    Tuple(Vec<TupleFieldDecl>),
    /// Unit struct (e.g., `struct Marker`).
    Unit,
}

/// A named field in a struct.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDecl {
    /// The node ID.
    pub id: NodeId,
    /// Doc comments.
    pub docs: Vec<DocComment>,
    /// Attributes.
    pub attrs: Vec<Attribute>,
    /// Visibility.
    pub vis: Visibility,
    /// Field name.
    pub name: Ident,
    /// Field type.
    pub ty: Type,
    /// Default value (for struct initialization).
    pub default: Option<Expr>,
    /// The span of this field.
    pub span: Span,
}

/// A tuple field in a tuple struct.
#[derive(Debug, Clone, PartialEq)]
pub struct TupleFieldDecl {
    /// The node ID.
    pub id: NodeId,
    /// Doc comments.
    pub docs: Vec<DocComment>,
    /// Attributes.
    pub attrs: Vec<Attribute>,
    /// Visibility.
    pub vis: Visibility,
    /// Field type.
    pub ty: Type,
    /// The span of this field.
    pub span: Span,
}

/// An enum declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    /// The node ID.
    pub id: NodeId,
    /// The enum name.
    pub name: Ident,
    /// Generic parameters.
    pub generics: Vec<GenericParam>,
    /// Where clause.
    pub where_clause: Option<WhereClause>,
    /// Enum variants.
    pub variants: Vec<VariantDecl>,
    /// The span of this enum.
    pub span: Span,
}

/// An enum variant.
#[derive(Debug, Clone, PartialEq)]
pub struct VariantDecl {
    /// The node ID.
    pub id: NodeId,
    /// Doc comments.
    pub docs: Vec<DocComment>,
    /// Attributes.
    pub attrs: Vec<Attribute>,
    /// Variant name.
    pub name: Ident,
    /// The kind of variant.
    pub kind: VariantKind,
    /// Optional discriminant value (for C-like enums).
    pub discriminant: Option<Expr>,
    /// The span of this variant.
    pub span: Span,
}

/// The kind of enum variant.
#[derive(Debug, Clone, PartialEq)]
pub enum VariantKind {
    /// Unit variant (e.g., `None`).
    Unit,
    /// Tuple variant (e.g., `Some(T)`).
    Tuple(Vec<TupleFieldDecl>),
    /// Struct variant (e.g., `Error { code: int, message: str }`).
    Struct(Vec<FieldDecl>),
}

/// A trait declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct TraitDecl {
    /// The node ID.
    pub id: NodeId,
    /// The trait name.
    pub name: Ident,
    /// Generic parameters.
    pub generics: Vec<GenericParam>,
    /// Supertraits (e.g., `trait Foo: Bar + Baz`).
    pub supertraits: Vec<TraitBound>,
    /// Where clause.
    pub where_clause: Option<WhereClause>,
    /// Trait items (methods, associated types, constants).
    pub items: Vec<TraitItem>,
    /// Whether this is an unsafe trait.
    pub is_unsafe: bool,
    /// Whether this is an auto trait.
    pub is_auto: bool,
    /// The span of this trait.
    pub span: Span,
}

/// An item in a trait.
#[derive(Debug, Clone, PartialEq)]
pub struct TraitItem {
    /// The node ID.
    pub id: NodeId,
    /// Doc comments.
    pub docs: Vec<DocComment>,
    /// Attributes.
    pub attrs: Vec<Attribute>,
    /// The kind of trait item.
    pub kind: TraitItemKind,
    /// The span of this item.
    pub span: Span,
}

/// The kind of trait item.
#[derive(Debug, Clone, PartialEq)]
pub enum TraitItemKind {
    /// A method declaration (may have a default implementation).
    Method {
        sig: FunctionSignature,
        body: Option<Block>,
    },
    /// An associated type (e.g., `type Item;` or `type Item = T;`).
    Type {
        name: Ident,
        bounds: Vec<TraitBound>,
        default: Option<Type>,
    },
    /// An associated constant (e.g., `const MAX: int;` or `const MAX: int = 100;`).
    Const {
        name: Ident,
        ty: Type,
        default: Option<Expr>,
    },
}

/// A function signature (used in traits and function pointers).
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionSignature {
    /// The function name.
    pub name: Ident,
    /// Generic parameters.
    pub generics: Vec<GenericParam>,
    /// Function parameters.
    pub params: Vec<FunctionParam>,
    /// Return type.
    pub return_type: Option<Type>,
    /// Where clause.
    pub where_clause: Option<WhereClause>,
    /// Whether this is async.
    pub is_async: bool,
    /// Whether this is unsafe.
    pub is_unsafe: bool,
    /// Whether this is const.
    pub is_const: bool,
    /// The span of this signature.
    pub span: Span,
}

/// An impl block.
#[derive(Debug, Clone, PartialEq)]
pub struct ImplDecl {
    /// The node ID.
    pub id: NodeId,
    /// Generic parameters.
    pub generics: Vec<GenericParam>,
    /// The trait being implemented (None for inherent impls).
    pub trait_ref: Option<Path>,
    /// The type being implemented for.
    pub self_ty: Type,
    /// Where clause.
    pub where_clause: Option<WhereClause>,
    /// Items in this impl block.
    pub items: Vec<ImplItem>,
    /// Whether this is an unsafe impl.
    pub is_unsafe: bool,
    /// The span of this impl.
    pub span: Span,
}

/// An item in an impl block.
#[derive(Debug, Clone, PartialEq)]
pub struct ImplItem {
    /// The node ID.
    pub id: NodeId,
    /// Doc comments.
    pub docs: Vec<DocComment>,
    /// Attributes.
    pub attrs: Vec<Attribute>,
    /// Visibility (for inherent impls).
    pub vis: Visibility,
    /// The kind of impl item.
    pub kind: ImplItemKind,
    /// The span of this item.
    pub span: Span,
}

/// The kind of impl item.
#[derive(Debug, Clone, PartialEq)]
pub enum ImplItemKind {
    /// A method.
    Method(Function),
    /// An associated type.
    Type {
        name: Ident,
        ty: Type,
    },
    /// An associated constant.
    Const {
        name: Ident,
        ty: Type,
        value: Expr,
    },
}

/// A type alias declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeAlias {
    /// The node ID.
    pub id: NodeId,
    /// The alias name.
    pub name: Ident,
    /// Generic parameters.
    pub generics: Vec<GenericParam>,
    /// Where clause.
    pub where_clause: Option<WhereClause>,
    /// The type being aliased.
    pub ty: Type,
    /// The span of this alias.
    pub span: Span,
}

/// A const declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstDecl {
    /// The node ID.
    pub id: NodeId,
    /// The const name.
    pub name: Ident,
    /// The type (optional if inferred).
    pub ty: Option<Type>,
    /// The value.
    pub value: Expr,
    /// The span of this declaration.
    pub span: Span,
}

/// A static declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct StaticDecl {
    /// The node ID.
    pub id: NodeId,
    /// The static name.
    pub name: Ident,
    /// The type.
    pub ty: Type,
    /// The initial value (optional for extern statics).
    pub value: Option<Expr>,
    /// Whether this is mutable.
    pub mutable: bool,
    /// The span of this declaration.
    pub span: Span,
}

/// An import declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportDecl {
    /// The node ID.
    pub id: NodeId,
    /// The import tree.
    pub tree: ImportTree,
    /// The span of this import.
    pub span: Span,
}

/// An import tree (for use/import statements).
#[derive(Debug, Clone, PartialEq)]
pub enum ImportTree {
    /// Simple import (e.g., `import foo`).
    Simple {
        path: Path,
        alias: Option<Ident>,
    },
    /// Glob import (e.g., `from foo import *`).
    Glob(Path),
    /// Nested imports (e.g., `from foo import {bar, baz}`).
    Nested {
        prefix: Path,
        trees: Vec<ImportTree>,
    },
}

/// An export declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportDecl {
    /// The node ID.
    pub id: NodeId,
    /// The export tree.
    pub tree: ExportTree,
    /// The span of this export.
    pub span: Span,
}

/// An export tree.
#[derive(Debug, Clone, PartialEq)]
pub enum ExportTree {
    /// Export an item by name.
    Name {
        name: Ident,
        alias: Option<Ident>,
    },
    /// Re-export from another module.
    Reexport(ImportTree),
    /// Export all.
    All,
}

/// An extern block.
#[derive(Debug, Clone, PartialEq)]
pub struct ExternBlock {
    /// The node ID.
    pub id: NodeId,
    /// The ABI (e.g., "C", "system", "Rust").
    pub abi: Option<Symbol>,
    /// Items in this extern block.
    pub items: Vec<ExternItem>,
    /// The span of this block.
    pub span: Span,
}

/// An item in an extern block.
#[derive(Debug, Clone, PartialEq)]
pub struct ExternItem {
    /// The node ID.
    pub id: NodeId,
    /// Doc comments.
    pub docs: Vec<DocComment>,
    /// Attributes.
    pub attrs: Vec<Attribute>,
    /// Visibility.
    pub vis: Visibility,
    /// The kind of extern item.
    pub kind: ExternItemKind,
    /// The span of this item.
    pub span: Span,
}

/// The kind of extern item.
#[derive(Debug, Clone, PartialEq)]
pub enum ExternItemKind {
    /// An extern function.
    Function(FunctionSignature),
    /// An extern static.
    Static {
        name: Ident,
        ty: Type,
        mutable: bool,
    },
    /// An extern type (opaque type from foreign code).
    Type(Ident),
}

/// A module declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDecl {
    /// The node ID.
    pub id: NodeId,
    /// The module name.
    pub name: Ident,
    /// The module content (None for external modules).
    pub content: Option<Vec<Item>>,
    /// The span of this module.
    pub span: Span,
}

/// A macro declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct MacroDecl {
    /// The node ID.
    pub id: NodeId,
    /// The macro name.
    pub name: Ident,
    /// Macro rules.
    pub rules: Vec<MacroRule>,
    /// The span of this macro.
    pub span: Span,
}

/// A macro rule.
#[derive(Debug, Clone, PartialEq)]
pub struct MacroRule {
    /// The matcher (input pattern).
    pub matcher: Vec<MacroToken>,
    /// The transcriber (output template).
    pub transcriber: Vec<MacroToken>,
    /// The span of this rule.
    pub span: Span,
}

/// A token in a macro definition.
#[derive(Debug, Clone, PartialEq)]
pub enum MacroToken {
    /// A literal token.
    Token(covibe_util::interner::Symbol),
    /// A metavariable (e.g., `$x:expr`).
    Metavar {
        name: Ident,
        kind: MacroFragmentKind,
    },
    /// A repetition (e.g., `$(...)*`, `$(...)+`).
    Repeat {
        tokens: Vec<MacroToken>,
        separator: Option<Symbol>,
        kind: MacroRepeatKind,
    },
}

/// The kind of macro fragment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroFragmentKind {
    Expr,
    Stmt,
    Pat,
    Ty,
    Ident,
    Path,
    Block,
    Item,
    Meta,
    Literal,
}

/// The kind of macro repetition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroRepeatKind {
    /// Zero or more (`*`).
    ZeroOrMore,
    /// One or more (`+`).
    OneOrMore,
    /// Zero or one (`?`).
    ZeroOrOne,
}
