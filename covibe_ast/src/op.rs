//! Operator definitions for expressions.

use std::fmt;

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
    // Arithmetic
    /// Addition (`+`)
    Add,
    /// Subtraction (`-`)
    Sub,
    /// Multiplication (`*`)
    Mul,
    /// Division (`/`)
    Div,
    /// Integer division (`//`)
    FloorDiv,
    /// Modulo (`%`)
    Mod,
    /// Exponentiation (`**`)
    Pow,

    // Bitwise
    /// Bitwise AND (`&`)
    BitAnd,
    /// Bitwise OR (`|`)
    BitOr,
    /// Bitwise XOR (`^`)
    BitXor,
    /// Left shift (`<<`)
    Shl,
    /// Right shift (`>>`)
    Shr,
    /// Unsigned right shift (`>>>`)
    UShr,

    // Comparison
    /// Equal (`==`)
    Eq,
    /// Not equal (`!=`)
    Ne,
    /// Less than (`<`)
    Lt,
    /// Less than or equal (`<=`)
    Le,
    /// Greater than (`>`)
    Gt,
    /// Greater than or equal (`>=`)
    Ge,
    /// Three-way comparison / spaceship (`<=>`)
    Spaceship,

    // Logical
    /// Logical AND (`and` or `&&`)
    And,
    /// Logical OR (`or` or `||`)
    Or,

    // Other
    /// Range (inclusive) (`..=`)
    RangeInclusive,
    /// Range (exclusive) (`..`)
    Range,
    /// Pipe operator (`|>`)
    Pipe,
    /// Optional chaining (`?.`)
    OptionalChaining,
    /// Null coalescing (`??`)
    NullCoalesce,
    /// Type check (`is`)
    Is,
    /// Membership test (`in`)
    In,
}

impl BinOp {
    /// Returns true if this is an arithmetic operator.
    pub fn is_arithmetic(&self) -> bool {
        matches!(
            self,
            BinOp::Add
                | BinOp::Sub
                | BinOp::Mul
                | BinOp::Div
                | BinOp::FloorDiv
                | BinOp::Mod
                | BinOp::Pow
        )
    }

    /// Returns true if this is a comparison operator.
    pub fn is_comparison(&self) -> bool {
        matches!(
            self,
            BinOp::Eq
                | BinOp::Ne
                | BinOp::Lt
                | BinOp::Le
                | BinOp::Gt
                | BinOp::Ge
                | BinOp::Spaceship
        )
    }

    /// Returns true if this is a logical operator.
    pub fn is_logical(&self) -> bool {
        matches!(self, BinOp::And | BinOp::Or)
    }

    /// Returns true if this is a bitwise operator.
    pub fn is_bitwise(&self) -> bool {
        matches!(
            self,
            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr | BinOp::UShr
        )
    }

    /// Returns true if this operator short-circuits (doesn't evaluate right side if unnecessary).
    pub fn is_short_circuit(&self) -> bool {
        matches!(self, BinOp::And | BinOp::Or | BinOp::OptionalChaining)
    }
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::FloorDiv => "//",
            BinOp::Mod => "%",
            BinOp::Pow => "**",
            BinOp::BitAnd => "&",
            BinOp::BitOr => "|",
            BinOp::BitXor => "^",
            BinOp::Shl => "<<",
            BinOp::Shr => ">>",
            BinOp::UShr => ">>>",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::Spaceship => "<=>",
            BinOp::And => "and",
            BinOp::Or => "or",
            BinOp::Range => "..",
            BinOp::RangeInclusive => "..=",
            BinOp::Pipe => "|>",
            BinOp::OptionalChaining => "?.",
            BinOp::NullCoalesce => "??",
            BinOp::Is => "is",
            BinOp::In => "in",
        };
        write!(f, "{}", s)
    }
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnOp {
    /// Negation (`-`)
    Neg,
    /// Logical NOT (`not` or `!`)
    Not,
    /// Bitwise NOT (`~`)
    BitNot,
    /// Dereference (`*`)
    Deref,
    /// Address-of / reference (`&`)
    Ref,
    /// Mutable reference (`&mut`)
    RefMut,
    /// Spread operator (`...`)
    Spread,
    /// Try operator (`?`)
    Try,
}

impl fmt::Display for UnOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            UnOp::Neg => "-",
            UnOp::Not => "not",
            UnOp::BitNot => "~",
            UnOp::Deref => "*",
            UnOp::Ref => "&",
            UnOp::RefMut => "&mut",
            UnOp::Spread => "...",
            UnOp::Try => "?",
        };
        write!(f, "{}", s)
    }
}

/// Assignment operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssignOp {
    /// Simple assignment (`=`)
    Assign,
    /// Addition assignment (`+=`)
    AddAssign,
    /// Subtraction assignment (`-=`)
    SubAssign,
    /// Multiplication assignment (`*=`)
    MulAssign,
    /// Division assignment (`/=`)
    DivAssign,
    /// Floor division assignment (`//=`)
    FloorDivAssign,
    /// Modulo assignment (`%=`)
    ModAssign,
    /// Exponentiation assignment (`**=`)
    PowAssign,
    /// Bitwise AND assignment (`&=`)
    BitAndAssign,
    /// Bitwise OR assignment (`|=`)
    BitOrAssign,
    /// Bitwise XOR assignment (`^=`)
    BitXorAssign,
    /// Left shift assignment (`<<=`)
    ShlAssign,
    /// Right shift assignment (`>>=`)
    ShrAssign,
    /// Unsigned right shift assignment (`>>>=`)
    UShrAssign,
    /// Walrus operator (`:=`) - inline assignment
    Walrus,
}

impl AssignOp {
    /// Converts this assignment operator to its corresponding binary operator, if applicable.
    pub fn to_binop(&self) -> Option<BinOp> {
        match self {
            AssignOp::AddAssign => Some(BinOp::Add),
            AssignOp::SubAssign => Some(BinOp::Sub),
            AssignOp::MulAssign => Some(BinOp::Mul),
            AssignOp::DivAssign => Some(BinOp::Div),
            AssignOp::FloorDivAssign => Some(BinOp::FloorDiv),
            AssignOp::ModAssign => Some(BinOp::Mod),
            AssignOp::PowAssign => Some(BinOp::Pow),
            AssignOp::BitAndAssign => Some(BinOp::BitAnd),
            AssignOp::BitOrAssign => Some(BinOp::BitOr),
            AssignOp::BitXorAssign => Some(BinOp::BitXor),
            AssignOp::ShlAssign => Some(BinOp::Shl),
            AssignOp::ShrAssign => Some(BinOp::Shr),
            AssignOp::UShrAssign => Some(BinOp::UShr),
            AssignOp::Assign | AssignOp::Walrus => None,
        }
    }
}

impl fmt::Display for AssignOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            AssignOp::Assign => "=",
            AssignOp::AddAssign => "+=",
            AssignOp::SubAssign => "-=",
            AssignOp::MulAssign => "*=",
            AssignOp::DivAssign => "/=",
            AssignOp::FloorDivAssign => "//=",
            AssignOp::ModAssign => "%=",
            AssignOp::PowAssign => "**=",
            AssignOp::BitAndAssign => "&=",
            AssignOp::BitOrAssign => "|=",
            AssignOp::BitXorAssign => "^=",
            AssignOp::ShlAssign => "<<=",
            AssignOp::ShrAssign => ">>=",
            AssignOp::UShrAssign => ">>>=",
            AssignOp::Walrus => ":=",
        };
        write!(f, "{}", s)
    }
}
