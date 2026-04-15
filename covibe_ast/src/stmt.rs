//! Statement AST nodes.

use super::*;
use crate::expr::Expr;
use crate::pat::Pattern;
use covibe_util::span::Span;

/// A statement in the AST.
#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    /// The node ID.
    pub id: NodeId,
    /// The kind of statement.
    pub kind: StmtKind,
    /// The span of this statement.
    pub span: Span,
}

impl Stmt {
    /// Creates a new statement.
    pub fn new(id: NodeId, kind: StmtKind, span: Span) -> Self {
        Self { id, kind, span }
    }
}

/// The kind of statement.
#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    /// Expression statement (e.g., `foo();`, `x + y;`).
    Expr(Expr),

    /// Let binding (e.g., `let x = 5`, `let (a, b) = tuple`).
    Let {
        pattern: Pattern,
        ty: Option<Type>,
        init: Option<Expr>,
        /// Whether this is mutable (`let mut x`).
        mutable: bool,
    },

    /// Variable declaration (e.g., `var x = 5`).
    Var {
        pattern: Pattern,
        ty: Option<Type>,
        init: Option<Expr>,
    },

    /// Const declaration (e.g., `const PI = 3.14159`).
    Const {
        name: Ident,
        ty: Option<Type>,
        value: Expr,
    },

    /// Assignment statement (e.g., `x = 5`, `arr[i] += 1`).
    Assign {
        target: Expr,
        value: Expr,
    },

    /// If statement.
    If {
        condition: Expr,
        then_branch: Block,
        elif_branches: Vec<(Expr, Block)>,
        else_branch: Option<Block>,
    },

    /// Match statement.
    Match {
        scrutinee: Expr,
        arms: Vec<crate::expr::MatchArm>,
    },

    /// While loop.
    While {
        condition: Expr,
        body: Block,
    },

    /// For loop (e.g., `for x in items: ...`).
    For {
        pattern: Pattern,
        iter: Expr,
        body: Block,
    },

    /// Infinite loop (e.g., `loop: ...`).
    Loop {
        body: Block,
    },

    /// Break statement (optionally with a value).
    Break(Option<Expr>),

    /// Continue statement.
    Continue,

    /// Return statement (optionally with a value).
    Return(Option<Expr>),

    /// Yield statement (for generators).
    Yield(Option<Expr>),

    /// Defer statement (e.g., `defer close(file)`).
    Defer(Box<Stmt>),

    /// Drop statement (explicit drop, e.g., `drop(value)`).
    Drop(Expr),

    /// Assert statement (e.g., `assert x > 0`, `assert cond, "message"`).
    Assert {
        condition: Expr,
        message: Option<Expr>,
    },

    /// Try/catch/finally statement.
    Try {
        body: Block,
        catch_clauses: Vec<crate::expr::CatchClause>,
        finally_block: Option<Block>,
    },

    /// Raise/throw statement (e.g., `raise ValueError("bad input")`).
    Raise(Option<Expr>),

    /// With statement (context manager, e.g., `with file = open("x"): ...`).
    With {
        items: Vec<WithItem>,
        body: Block,
    },

    /// Async block.
    Async(Block),

    /// Spawn statement.
    Spawn(Expr),

    /// Select statement (for channel operations).
    Select {
        arms: Vec<SelectArm>,
    },

    /// Unsafe block.
    Unsafe(Block),

    /// Comptime block.
    Comptime(Block),

    /// Item (nested function, struct, etc.).
    Item(Box<Item>),

    /// Empty statement (e.g., from a lone semicolon).
    Empty,
}

/// A block of statements.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// The node ID.
    pub id: NodeId,
    /// The statements in this block.
    pub stmts: Vec<Stmt>,
    /// Optional trailing expression (the value of the block).
    pub expr: Option<Box<Expr>>,
    /// The span of this block.
    pub span: Span,
}

impl Block {
    /// Creates a new block.
    pub fn new(id: NodeId, stmts: Vec<Stmt>, expr: Option<Box<Expr>>, span: Span) -> Self {
        Self {
            id,
            stmts,
            expr,
            span,
        }
    }

    /// Returns true if this block is empty.
    pub fn is_empty(&self) -> bool {
        self.stmts.is_empty() && self.expr.is_none()
    }
}

/// A with-statement item (context manager binding).
#[derive(Debug, Clone, PartialEq)]
pub struct WithItem {
    /// The context manager expression.
    pub context: Expr,
    /// Optional variable binding for the context manager's result.
    pub binding: Option<Pattern>,
    /// The span of this item.
    pub span: Span,
}

/// A select statement arm (for channel operations).
#[derive(Debug, Clone, PartialEq)]
pub struct SelectArm {
    /// The node ID.
    pub id: NodeId,
    /// The kind of select arm.
    pub kind: SelectArmKind,
    /// The body to execute if this arm is selected.
    pub body: Block,
    /// The span of this arm.
    pub span: Span,
}

/// The kind of select arm.
#[derive(Debug, Clone, PartialEq)]
pub enum SelectArmKind {
    /// Receive from a channel (e.g., `recv value from channel`).
    Recv {
        pattern: Pattern,
        channel: Expr,
    },
    /// Send to a channel (e.g., `send value to channel`).
    Send {
        value: Expr,
        channel: Expr,
    },
    /// Default case (executes if no other operation is ready).
    Default,
}
