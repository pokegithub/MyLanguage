//! Expression AST nodes.

use super::*;
use crate::literal::Literal;
use crate::op::{AssignOp, BinOp, UnOp};
use crate::pat::Pattern;
use crate::stmt::Block;
use covibe_util::span::Span;

/// An expression in the AST.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    /// The node ID.
    pub id: NodeId,
    /// The kind of expression.
    pub kind: ExprKind,
    /// The span of this expression.
    pub span: Span,
}

impl Expr {
    /// Creates a new expression.
    pub fn new(id: NodeId, kind: ExprKind, span: Span) -> Self {
        Self { id, kind, span }
    }
}

/// The kind of expression.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// A literal value (e.g., `42`, `"hello"`, `true`).
    Literal(Literal),

    /// A variable or path reference (e.g., `x`, `std::io::read`).
    Path(Path),

    /// Binary operation (e.g., `a + b`, `x == y`).
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },

    /// Unary operation (e.g., `-x`, `!flag`, `*ptr`).
    Unary { op: UnOp, operand: Box<Expr> },

    /// Assignment (e.g., `x = 5`, `y += 3`).
    Assign {
        op: AssignOp,
        target: Box<Expr>,
        value: Box<Expr>,
    },

    /// Function call (e.g., `foo(a, b)`, `obj.method(x)`).
    Call {
        func: Box<Expr>,
        args: Vec<Arg>,
    },

    /// Method call (e.g., `x.to_string()`, `vec.push(item)`).
    MethodCall {
        receiver: Box<Expr>,
        method: Ident,
        args: Vec<Arg>,
        /// Generic arguments (e.g., `vec.into::<String>()`).
        generics: Option<GenericArgs>,
    },

    /// Field access (e.g., `obj.field`, `point.x`).
    Field {
        object: Box<Expr>,
        field: Ident,
    },

    /// Tuple field access (e.g., `tuple.0`, `pair.1`).
    TupleIndex {
        object: Box<Expr>,
        index: usize,
    },

    /// Array/slice indexing (e.g., `arr[i]`, `matrix[x][y]`).
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },

    /// Range expression (e.g., `1..10`, `0..=5`, `..`, `a..`).
    Range {
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        inclusive: bool,
    },

    /// Tuple expression (e.g., `(1, 2, 3)`, `(x, y)`).
    Tuple(Vec<Expr>),

    /// Array literal (e.g., `[1, 2, 3]`).
    Array(Vec<Expr>),

    /// Array repeat expression (e.g., `[0; 10]`).
    ArrayRepeat {
        value: Box<Expr>,
        count: Box<Expr>,
    },

    /// Dictionary/map literal (e.g., `{x: 1, y: 2}`).
    Dict(Vec<(Expr, Expr)>),

    /// Set literal (e.g., `{1, 2, 3}`).
    Set(Vec<Expr>),

    /// List comprehension (e.g., `[x * 2 for x in range(10) if x % 2 == 0]`).
    ListComp {
        element: Box<Expr>,
        comprehensions: Vec<Comprehension>,
    },

    /// Set comprehension (e.g., `{x * 2 for x in range(10)}`).
    SetComp {
        element: Box<Expr>,
        comprehensions: Vec<Comprehension>,
    },

    /// Dict comprehension (e.g., `{k: v * 2 for k, v in items}`).
    DictComp {
        key: Box<Expr>,
        value: Box<Expr>,
        comprehensions: Vec<Comprehension>,
    },

    /// Generator expression (e.g., `(x * 2 for x in range(10))`).
    Generator {
        element: Box<Expr>,
        comprehensions: Vec<Comprehension>,
    },

    /// If expression (e.g., `if cond: a else: b`).
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        elif_branches: Vec<(Expr, Expr)>,
        else_branch: Option<Box<Expr>>,
    },

    /// Match expression.
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },

    /// Block expression (e.g., `{ stmt1; stmt2; expr }`).
    Block(Block),

    /// Lambda/closure expression (e.g., `lambda x: x + 1`, `|a, b| a + b`).
    Lambda {
        params: Vec<FunctionParam>,
        return_type: Option<Type>,
        body: Box<Expr>,
        /// Whether this captures variables from the environment.
        captures: Vec<Capture>,
    },

    /// Return expression (e.g., `return 42`, `return`).
    Return(Option<Box<Expr>>),

    /// Break expression (e.g., `break`, `break value`).
    Break(Option<Box<Expr>>),

    /// Continue expression.
    Continue,

    /// Yield expression (for generators).
    Yield(Option<Box<Expr>>),

    /// Await expression (e.g., `await future`, `future.await`).
    Await(Box<Expr>),

    /// Async block (e.g., `async { ... }`).
    Async(Block),

    /// Spawn expression (e.g., `spawn task()`).
    Spawn(Box<Expr>),

    /// Try block (e.g., `try { ... } catch e { ... }`).
    Try {
        body: Block,
        catch_clauses: Vec<CatchClause>,
        finally_block: Option<Block>,
    },

    /// Type cast (e.g., `x as int`, `value as f64`).
    Cast {
        expr: Box<Expr>,
        ty: Type,
    },

    /// Type ascription (e.g., `x: int`).
    Type {
        expr: Box<Expr>,
        ty: Type,
    },

    /// Struct initialization (e.g., `Point { x: 1, y: 2 }`).
    Struct {
        path: Path,
        fields: Vec<FieldInit>,
        /// Struct update syntax (e.g., `Point { x: 5, ..old_point }`).
        base: Option<Box<Expr>>,
    },

    /// Tuple struct initialization (e.g., `Color(255, 0, 0)`).
    TupleStruct {
        path: Path,
        fields: Vec<Expr>,
    },

    /// Unit struct (e.g., `MyStruct`).
    UnitStruct(Path),

    /// Parenthesized expression (e.g., `(x + y)`).
    Paren(Box<Expr>),

    /// Comptime expression (e.g., `comptime { ... }`).
    Comptime(Block),

    /// Macro invocation (e.g., `println!("hello")`, `vec![1, 2, 3]`).
    Macro {
        path: Path,
        args: Vec<Expr>,
    },

    /// Unsafe block (e.g., `unsafe { ... }`).
    Unsafe(Block),

    /// Move expression (e.g., `move |x| x + 1`).
    Move(Box<Expr>),

    /// Clone expression (e.g., `clone obj`).
    Clone(Box<Expr>),

    /// Copy expression (e.g., `copy value`).
    Copy(Box<Expr>),

    /// Box expression (heap allocation, e.g., `box value`).
    Box(Box<Expr>),

    /// Placeholder for error recovery.
    Error,
}

/// A function argument.
#[derive(Debug, Clone, PartialEq)]
pub struct Arg {
    /// Optional argument name (for named arguments).
    pub name: Option<Ident>,
    /// The argument value.
    pub value: Expr,
    /// Whether this is a spread argument (`...args`).
    pub spread: bool,
}

/// A comprehension clause (for list/set/dict comprehensions and generators).
#[derive(Debug, Clone, PartialEq)]
pub struct Comprehension {
    /// The pattern to bind (e.g., `x` in `for x in items`).
    pub pattern: Pattern,
    /// The iterator expression.
    pub iter: Expr,
    /// Optional filter conditions (e.g., `if x > 0`).
    pub filters: Vec<Expr>,
    /// Whether this is an async comprehension.
    pub is_async: bool,
}

/// A match arm.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    /// The node ID.
    pub id: NodeId,
    /// The pattern to match.
    pub pattern: Pattern,
    /// Optional guard condition (e.g., `if x > 0`).
    pub guard: Option<Expr>,
    /// The expression to evaluate if this arm matches.
    pub body: Expr,
    /// The span of this arm.
    pub span: Span,
}

/// A catch clause in a try block.
#[derive(Debug, Clone, PartialEq)]
pub struct CatchClause {
    /// The pattern for the caught exception.
    pub pattern: Option<Pattern>,
    /// The body of the catch clause.
    pub body: Block,
    /// The span of this clause.
    pub span: Span,
}

/// A field initializer in a struct expression.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldInit {
    /// The field name.
    pub name: Ident,
    /// The field value. If None, uses shorthand syntax (e.g., `Point { x, y }`).
    pub value: Option<Expr>,
    /// The span of this field initializer.
    pub span: Span,
}

/// A captured variable in a closure.
#[derive(Debug, Clone, PartialEq)]
pub struct Capture {
    /// The variable being captured.
    pub var: Ident,
    /// How the variable is captured.
    pub kind: CaptureKind,
}

/// The kind of variable capture in a closure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureKind {
    /// By immutable reference.
    ByRef,
    /// By mutable reference.
    ByRefMut,
    /// By move (takes ownership).
    ByMove,
}

/// A function parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionParam {
    /// The node ID.
    pub id: NodeId,
    /// The parameter pattern (usually an identifier, but can be destructuring).
    pub pattern: Pattern,
    /// The type annotation (optional if inferred).
    pub ty: Option<Type>,
    /// Default value for the parameter.
    pub default: Option<Expr>,
    /// The span of this parameter.
    pub span: Span,
}
