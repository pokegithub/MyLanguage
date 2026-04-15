//! Expression parsing using the Pratt parsing algorithm.
//!
//! This module implements a complete Pratt parser for CoVibe expressions
//! with proper operator precedence and associativity handling.
//!
//! The parser supports:
//! - Primary expressions (literals, identifiers, parenthesized expressions)
//! - Unary operators (prefix: -, !, ~, &, *, not)
//! - Binary operators (all arithmetic, comparison, logical, bitwise operators)
//! - Postfix operators (function calls, field access, indexing, method calls)
//! - Collection literals (arrays, tuples, dicts, sets)
//! - Comprehensions (list, set, dict, generator)
//! - Control flow expressions (if, match)
//! - Lambda expressions
//! - Range expressions
//! - Type casts and ascriptions

use super::{ParseError, ParseResult, Parser};
use covibe_ast::*;
use covibe_lexer::token::{Literal as TokenLiteral, TokenKind};
use covibe_util::span::Span;

/// Operator precedence levels (higher number = higher precedence).
///
/// Based on the CoVibe language specification Part 1, Section 11.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum Precedence {
    None = 0,
    Assignment = 1,      // = += -= *= /= etc., |>, <|
    Ternary = 2,         // ?: (not implemented yet, reserved)
    NullCoalesce = 3,    // ??
    Or = 4,              // or, ||
    And = 5,             // and, &&
    Comparison = 6,      // == != < <= > >= <=> is in
    Range = 7,           // .. ..=
    BitOr = 8,           // |
    BitXor = 9,          // ^
    BitAnd = 10,         // &
    Shift = 11,          // << >> >>>
    Additive = 12,       // + -
    Multiplicative = 13, // * / // %
    Cast = 14,           // as
    Unary = 15,          // - ! ~ not & * (prefix)
    Power = 16,          // **
    OptionalChain = 17,  // ?.
    Postfix = 18,        // () [] . (postfix)
    Path = 19,           // ::
}

impl Precedence {
    /// Returns the precedence for a given binary operator token.
    fn for_binop(kind: &TokenKind) -> Precedence {
        match kind {
            // Assignment
            TokenKind::Eq
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
            | TokenKind::PipeGt
            | TokenKind::ColonEq => Precedence::Assignment,

            // Null coalescing
            TokenKind::QuestionQuestion => Precedence::NullCoalesce,

            // Logical OR
            TokenKind::Or | TokenKind::OrOr => Precedence::Or,

            // Logical AND
            TokenKind::And | TokenKind::AndAnd => Precedence::And,

            // Comparison
            TokenKind::EqEq
            | TokenKind::BangEq
            | TokenKind::Lt
            | TokenKind::LtEq
            | TokenKind::Gt
            | TokenKind::GtEq
            | TokenKind::Spaceship
            | TokenKind::Is
            | TokenKind::In => Precedence::Comparison,

            // Range
            TokenKind::DotDot | TokenKind::DotDotEq => Precedence::Range,

            // Bitwise OR
            TokenKind::Pipe => Precedence::BitOr,

            // Bitwise XOR
            TokenKind::Caret => Precedence::BitXor,

            // Bitwise AND
            TokenKind::Ampersand => Precedence::BitAnd,

            // Shift
            TokenKind::LtLt | TokenKind::GtGt | TokenKind::GtGtGt => Precedence::Shift,

            // Additive
            TokenKind::Plus | TokenKind::Minus => Precedence::Additive,

            // Multiplicative
            TokenKind::Star | TokenKind::Slash | TokenKind::SlashSlash | TokenKind::Percent => {
                Precedence::Multiplicative
            }

            // Cast
            TokenKind::As => Precedence::Cast,

            // Power
            TokenKind::StarStar => Precedence::Power,

            // Optional chaining (Note: not yet implemented in lexer)
            // TokenKind::QuestionDot => Precedence::OptionalChain,

            // Path separator
            TokenKind::ColonColon => Precedence::Path,

            _ => Precedence::None,
        }
    }

    /// Returns whether the operator is right-associative.
    fn is_right_associative(kind: &TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Eq
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
                | TokenKind::ColonEq
                | TokenKind::StarStar
                | TokenKind::QuestionQuestion
        )
    }
}

/// Expression parser trait for the main Parser.
pub trait ExprParser {
    /// Parses an expression.
    fn parse_expr(&mut self) -> ParseResult<Expr>;

    /// Parses an expression with a minimum precedence level (Pratt parsing).
    fn parse_expr_with_precedence(&mut self, min_prec: Precedence) -> ParseResult<Expr>;

    /// Parses a primary expression.
    fn parse_primary_expr(&mut self) -> ParseResult<Expr>;

    /// Parses a prefix unary expression.
    fn parse_prefix_expr(&mut self) -> ParseResult<Expr>;

    /// Parses an infix binary expression.
    fn parse_infix_expr(&mut self, left: Expr) -> ParseResult<Expr>;

    /// Parses a postfix expression.
    fn parse_postfix_expr(&mut self, left: Expr) -> ParseResult<Expr>;
}

impl<'a> ExprParser for Parser<'a> {
    /// Parses an expression.
    ///
    /// This is the main entry point for expression parsing.
    fn parse_expr(&mut self) -> ParseResult<Expr> {
        self.parse_expr_with_precedence(Precedence::None)
    }

    /// Parses an expression with the Pratt parsing algorithm.
    ///
    /// This handles operator precedence and associativity correctly.
    fn parse_expr_with_precedence(&mut self, min_prec: Precedence) -> ParseResult<Expr> {
        // Parse the left side (prefix or primary expression)
        let mut left = self.parse_prefix_expr()?;

        // Parse infix and postfix operators
        loop {
            // Skip newlines between operators and operands
            if matches!(self.current_kind(), TokenKind::Newline) {
                // Only skip if the next token is an operator or postfix token
                if is_infix_or_postfix(&self.peek_kind(1)) {
                    self.skip_newlines();
                } else {
                    break;
                }
            }

            let current_kind = self.current_kind();

            // Check for postfix operators (highest precedence)
            if is_postfix_start(&current_kind) {
                left = self.parse_postfix_expr(left)?;
                continue;
            }

            // Check for infix operators
            let prec = Precedence::for_binop(&current_kind);
            if prec == Precedence::None || prec < min_prec {
                break;
            }

            left = self.parse_infix_expr(left)?;
        }

        Ok(left)
    }

    /// Parses a prefix unary expression or a primary expression.
    fn parse_prefix_expr(&mut self) -> ParseResult<Expr> {
        let start_pos = self.pos;
        let start_span = self.current().span;

        match self.current_kind() {
            // Unary minus
            TokenKind::Minus => {
                self.advance();
                let operand = self.parse_expr_with_precedence(Precedence::Unary)?;
                let span = start_span.to(operand.span.end());
                Ok(Expr::new(
                    self.next_node_id(),
                    ExprKind::Unary {
                        op: UnOp::Neg,
                        operand: Box::new(operand),
                    },
                    span,
                ))
            }

            // Logical NOT
            TokenKind::Bang | TokenKind::Not => {
                self.advance();
                let operand = self.parse_expr_with_precedence(Precedence::Unary)?;
                let span = start_span.to(operand.span.end());
                Ok(Expr::new(
                    self.next_node_id(),
                    ExprKind::Unary {
                        op: UnOp::Not,
                        operand: Box::new(operand),
                    },
                    span,
                ))
            }

            // Bitwise NOT
            TokenKind::Tilde => {
                self.advance();
                let operand = self.parse_expr_with_precedence(Precedence::Unary)?;
                let span = start_span.to(operand.span.end());
                Ok(Expr::new(
                    self.next_node_id(),
                    ExprKind::Unary {
                        op: UnOp::BitNot,
                        operand: Box::new(operand),
                    },
                    span,
                ))
            }

            // Reference/address-of
            TokenKind::Ampersand => {
                self.advance();
                // Check for &mut
                let op = if self.eat(TokenKind::Mut) {
                    UnOp::RefMut
                } else {
                    UnOp::Ref
                };
                let operand = self.parse_expr_with_precedence(Precedence::Unary)?;
                let span = start_span.to(operand.span.end());
                Ok(Expr::new(
                    self.next_node_id(),
                    ExprKind::Unary {
                        op,
                        operand: Box::new(operand),
                    },
                    span,
                ))
            }

            // Dereference
            TokenKind::Star => {
                self.advance();
                let operand = self.parse_expr_with_precedence(Precedence::Unary)?;
                let span = start_span.to(operand.span.end());
                Ok(Expr::new(
                    self.next_node_id(),
                    ExprKind::Unary {
                        op: UnOp::Deref,
                        operand: Box::new(operand),
                    },
                    span,
                ))
            }

            // Spread operator
            TokenKind::DotDotDot => {
                self.advance();
                let operand = self.parse_expr_with_precedence(Precedence::Unary)?;
                let span = start_span.to(operand.span.end());
                Ok(Expr::new(
                    self.next_node_id(),
                    ExprKind::Unary {
                        op: UnOp::Spread,
                        operand: Box::new(operand),
                    },
                    span,
                ))
            }

            _ => self.parse_primary_expr(),
        }
    }

    /// Parses an infix binary expression or assignment.
    fn parse_infix_expr(&mut self, left: Expr) -> ParseResult<Expr> {
        let start_span = left.span;
        let op_token = self.advance();
        let op_kind = op_token.kind.clone();

        let prec = Precedence::for_binop(&op_kind);

        // Adjust precedence for right-associative operators
        let next_prec = if Precedence::is_right_associative(&op_kind) {
            prec
        } else {
            // For left-associative, we need strictly greater precedence
            unsafe { std::mem::transmute::<u8, Precedence>(prec as u8 + 1) }
        };

        let right = self.parse_expr_with_precedence(next_prec)?;
        let span = start_span.to(right.span.end());

        // Handle assignment operators
        if is_assignment_op(&op_kind) {
            let op = token_to_assign_op(&op_kind);
            return Ok(Expr::new(
                self.next_node_id(),
                ExprKind::Assign {
                    op,
                    target: Box::new(left),
                    value: Box::new(right),
                },
                span,
            ));
        }

        // Handle pipe operator (transforms to call)
        if matches!(op_kind, TokenKind::PipeGt) {
            // a |> b becomes b(a)
            return Ok(Expr::new(
                self.next_node_id(),
                ExprKind::Call {
                    func: Box::new(right),
                    args: vec![Arg {
                        name: None,
                        value: left,
                    }],
                },
                span,
            ));
        }

        // Regular binary operator
        let op = token_to_binop(&op_kind)?;
        Ok(Expr::new(
            self.next_node_id(),
            ExprKind::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            },
            span,
        ))
    }

    /// Parses a postfix expression (call, index, field access, etc.).
    fn parse_postfix_expr(&mut self, mut expr: Expr) -> ParseResult<Expr> {
        loop {
            let start_span = expr.span;

            match self.current_kind() {
                // Function call: expr(args)
                TokenKind::LParen => {
                    self.advance();
                    let args = self.parse_call_args()?;
                    self.expect(TokenKind::RParen)?;
                    let span = start_span.to(self.tokens[self.pos - 1].span.end());
                    expr = Expr::new(
                        self.next_node_id(),
                        ExprKind::Call {
                            func: Box::new(expr),
                            args,
                        },
                        span,
                    );
                }

                // Array/slice indexing: expr[index]
                TokenKind::LBracket => {
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(TokenKind::RBracket)?;
                    let span = start_span.to(self.tokens[self.pos - 1].span.end());
                    expr = Expr::new(
                        self.next_node_id(),
                        ExprKind::Index {
                            object: Box::new(expr),
                            index: Box::new(index),
                        },
                        span,
                    );
                }

                // Field access or method call: expr.field or expr.method(args)
                TokenKind::Dot => {
                    self.advance();

                    // Check for tuple index: expr.0, expr.1, etc.
                    if let TokenKind::IntLiteral(TokenLiteral::Int(n, _, _)) = self.current_kind()
                    {
                        let index = n.parse::<usize>().map_err(|_| {
                            self.error("tuple index out of range".to_string())
                        })?;
                        let span = start_span.to(self.current().span.end());
                        self.advance();
                        expr = Expr::new(
                            self.next_node_id(),
                            ExprKind::TupleIndex {
                                object: Box::new(expr),
                                index,
                            },
                            span,
                        );
                    } else {
                        // Field access or method call
                        let field = self.expect_ident()?;

                        // Check if it's a method call
                        if self.check(TokenKind::LParen) {
                            // Method call
                            self.advance();
                            let args = self.parse_call_args()?;
                            self.expect(TokenKind::RParen)?;
                            let span = start_span.to(self.tokens[self.pos - 1].span.end());
                            expr = Expr::new(
                                self.next_node_id(),
                                ExprKind::MethodCall {
                                    receiver: Box::new(expr),
                                    method: field,
                                    args,
                                    generics: None, // TODO: parse turbofish ::<T>
                                },
                                span,
                            );
                        } else {
                            // Field access
                            let span = start_span.to(field.span.end());
                            expr = Expr::new(
                                self.next_node_id(),
                                ExprKind::Field {
                                    object: Box::new(expr),
                                    field,
                                },
                                span,
                            );
                        }
                    }
                }

                // Try operator: expr?
                TokenKind::Question => {
                    self.advance();
                    let span = start_span.to(self.tokens[self.pos - 1].span.end());
                    expr = Expr::new(
                        self.next_node_id(),
                        ExprKind::Unary {
                            op: UnOp::Try,
                            operand: Box::new(expr),
                        },
                        span,
                    );
                }

                // Path separator: expr::segment
                TokenKind::ColonColon => {
                    // Convert expr to path and continue parsing path segments
                    // This is complex and will be fully implemented in statement/type parsing
                    // For now, we handle simple cases
                    break;
                }

                _ => break,
            }
        }

        Ok(expr)
    }

    /// Parses a primary expression.
    fn parse_primary_expr(&mut self) -> ParseResult<Expr> {
        let start_pos = self.pos;
        let start_span = self.current().span;

        match self.current_kind() {
            // Integer literal
            TokenKind::IntLiteral(TokenLiteral::Int(value, base, suffix)) => {
                self.advance();
                Ok(Expr::new(
                    self.next_node_id(),
                    ExprKind::Literal(Literal::Int(value, base, suffix)),
                    start_span,
                ))
            }

            // Float literal
            TokenKind::FloatLiteral(TokenLiteral::Float(value, suffix)) => {
                self.advance();
                Ok(Expr::new(
                    self.next_node_id(),
                    ExprKind::Literal(Literal::Float(value, suffix)),
                    start_span,
                ))
            }

            // String literal
            TokenKind::StringLiteral(TokenLiteral::Str(value, kind)) => {
                self.advance();
                Ok(Expr::new(
                    self.next_node_id(),
                    ExprKind::Literal(Literal::Str(value, kind)),
                    start_span,
                ))
            }

            // Char literal
            TokenKind::CharLiteral(TokenLiteral::Char(value)) => {
                self.advance();
                Ok(Expr::new(
                    self.next_node_id(),
                    ExprKind::Literal(Literal::Char(value)),
                    start_span,
                ))
            }

            // Boolean literals
            TokenKind::True => {
                self.advance();
                Ok(Expr::new(
                    self.next_node_id(),
                    ExprKind::Literal(Literal::Bool(true)),
                    start_span,
                ))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::new(
                    self.next_node_id(),
                    ExprKind::Literal(Literal::Bool(false)),
                    start_span,
                ))
            }

            // None literal
            TokenKind::None | TokenKind::Null => {
                self.advance();
                Ok(Expr::new(
                    self.next_node_id(),
                    ExprKind::Literal(Literal::None),
                    start_span,
                ))
            }

            // Identifier or path
            TokenKind::Ident(name) => {
                let ident = Ident::new(name, start_span);
                self.advance();

                // Check if it's the start of a path
                if self.check(TokenKind::ColonColon) {
                    // Parse as path
                    let path = self.parse_path_from_ident(ident)?;
                    let span = path.span;
                    Ok(Expr::new(self.next_node_id(), ExprKind::Path(path), span))
                } else {
                    // Simple identifier reference
                    let path = Path::from_ident(ident);
                    Ok(Expr::new(self.next_node_id(), ExprKind::Path(path), start_span))
                }
            }

            // self keyword
            TokenKind::SelfLower => {
                let span = self.current().span;
                self.advance();
                let path = Path::from_ident(Ident::new(
                    covibe_util::interner::Symbol::INVALID, // Will be properly interned
                    span,
                ));
                Ok(Expr::new(self.next_node_id(), ExprKind::Path(path), span))
            }

            // Self keyword
            TokenKind::SelfUpper => {
                let span = self.current().span;
                self.advance();
                let path = Path::from_ident(Ident::new(
                    covibe_util::interner::Symbol::INVALID,
                    span,
                ));
                Ok(Expr::new(self.next_node_id(), ExprKind::Path(path), span))
            }

            // super keyword
            TokenKind::Super => {
                let span = self.current().span;
                self.advance();
                let path = Path::from_ident(Ident::new(
                    covibe_util::interner::Symbol::INVALID,
                    span,
                ));
                Ok(Expr::new(self.next_node_id(), ExprKind::Path(path), span))
            }

            // Parenthesized expression or tuple
            TokenKind::LParen => self.parse_paren_or_tuple_expr(),

            // Array literal
            TokenKind::LBracket => self.parse_array_expr(),

            // Dict or set literal
            TokenKind::LBrace => self.parse_dict_or_set_expr(),

            // Lambda expression
            TokenKind::Lambda => self.parse_lambda_expr(),

            // If expression
            TokenKind::If => self.parse_if_expr(),

            // Match expression
            TokenKind::Match => self.parse_match_expr(),

            // Return expression
            TokenKind::Return => {
                self.advance();
                let value = if is_expr_terminator(&self.current_kind()) {
                    None
                } else {
                    Some(Box::new(self.parse_expr()?))
                };
                let span = self.span_from(start_pos);
                Ok(Expr::new(
                    self.next_node_id(),
                    ExprKind::Return(value),
                    span,
                ))
            }

            // Break expression
            TokenKind::Break => {
                self.advance();
                let value = if is_expr_terminator(&self.current_kind()) {
                    None
                } else {
                    Some(Box::new(self.parse_expr()?))
                };
                let span = self.span_from(start_pos);
                Ok(Expr::new(self.next_node_id(), ExprKind::Break(value), span))
            }

            // Continue expression
            TokenKind::Continue => {
                self.advance();
                Ok(Expr::new(
                    self.next_node_id(),
                    ExprKind::Continue,
                    start_span,
                ))
            }

            // Yield expression
            TokenKind::Yield => {
                self.advance();
                let value = if is_expr_terminator(&self.current_kind()) {
                    None
                } else {
                    Some(Box::new(self.parse_expr()?))
                };
                let span = self.span_from(start_pos);
                Ok(Expr::new(self.next_node_id(), ExprKind::Yield(value), span))
            }

            // Await expression
            TokenKind::Await => {
                self.advance();
                let expr = self.parse_expr_with_precedence(Precedence::Unary)?;
                let span = start_span.to(expr.span.end());
                Ok(Expr::new(
                    self.next_node_id(),
                    ExprKind::Await(Box::new(expr)),
                    span,
                ))
            }

            // Spawn expression
            TokenKind::Spawn => {
                self.advance();
                let expr = self.parse_expr_with_precedence(Precedence::Unary)?;
                let span = start_span.to(expr.span.end());
                Ok(Expr::new(
                    self.next_node_id(),
                    ExprKind::Spawn(Box::new(expr)),
                    span,
                ))
            }

            _ => Err(self.error(format!(
                "unexpected token in expression: {:?}",
                self.current_kind()
            ))),
        }
    }
}

// ===== Helper Functions =====

/// Returns true if the token can start a postfix expression.
fn is_postfix_start(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::LParen
            | TokenKind::LBracket
            | TokenKind::Dot
            | TokenKind::Question
            | TokenKind::ColonColon
    )
}

/// Returns true if the token is an infix or postfix operator.
fn is_infix_or_postfix(kind: &TokenKind) -> bool {
    Precedence::for_binop(kind) != Precedence::None || is_postfix_start(kind)
}

/// Returns true if the token terminates an expression.
fn is_expr_terminator(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Newline
            | TokenKind::Semicolon
            | TokenKind::Comma
            | TokenKind::RParen
            | TokenKind::RBracket
            | TokenKind::RBrace
            | TokenKind::Eof
            | TokenKind::Colon
            | TokenKind::Then
            | TokenKind::Else
            | TokenKind::Elif
    )
}

/// Returns true if the token is an assignment operator.
fn is_assignment_op(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Eq
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
            | TokenKind::ColonEq
    )
}

/// Converts a token kind to an assignment operator.
fn token_to_assign_op(kind: &TokenKind) -> AssignOp {
    match kind {
        TokenKind::Eq => AssignOp::Assign,
        TokenKind::PlusEq => AssignOp::AddAssign,
        TokenKind::MinusEq => AssignOp::SubAssign,
        TokenKind::StarEq => AssignOp::MulAssign,
        TokenKind::SlashEq => AssignOp::DivAssign,
        TokenKind::SlashSlashEq => AssignOp::FloorDivAssign,
        TokenKind::PercentEq => AssignOp::ModAssign,
        TokenKind::StarStarEq => AssignOp::PowAssign,
        TokenKind::AmpersandEq => AssignOp::BitAndAssign,
        TokenKind::PipeEq => AssignOp::BitOrAssign,
        TokenKind::CaretEq => AssignOp::BitXorAssign,
        TokenKind::LtLtEq => AssignOp::ShlAssign,
        TokenKind::GtGtEq => AssignOp::ShrAssign,
        TokenKind::GtGtGtEq => AssignOp::UShrAssign,
        TokenKind::ColonEq => AssignOp::Walrus,
        _ => unreachable!("not an assignment operator: {:?}", kind),
    }
}

/// Converts a token kind to a binary operator.
fn token_to_binop(kind: &TokenKind) -> ParseResult<BinOp> {
    match kind {
        TokenKind::Plus => Ok(BinOp::Add),
        TokenKind::Minus => Ok(BinOp::Sub),
        TokenKind::Star => Ok(BinOp::Mul),
        TokenKind::Slash => Ok(BinOp::Div),
        TokenKind::SlashSlash => Ok(BinOp::FloorDiv),
        TokenKind::Percent => Ok(BinOp::Mod),
        TokenKind::StarStar => Ok(BinOp::Pow),
        TokenKind::Ampersand => Ok(BinOp::BitAnd),
        TokenKind::Pipe => Ok(BinOp::BitOr),
        TokenKind::Caret => Ok(BinOp::BitXor),
        TokenKind::LtLt => Ok(BinOp::Shl),
        TokenKind::GtGt => Ok(BinOp::Shr),
        TokenKind::GtGtGt => Ok(BinOp::UShr),
        TokenKind::EqEq => Ok(BinOp::Eq),
        TokenKind::BangEq => Ok(BinOp::Ne),
        TokenKind::Lt => Ok(BinOp::Lt),
        TokenKind::LtEq => Ok(BinOp::Le),
        TokenKind::Gt => Ok(BinOp::Gt),
        TokenKind::GtEq => Ok(BinOp::Ge),
        TokenKind::Spaceship => Ok(BinOp::Spaceship),
        TokenKind::And | TokenKind::AndAnd => Ok(BinOp::And),
        TokenKind::Or | TokenKind::OrOr => Ok(BinOp::Or),
        TokenKind::DotDot => Ok(BinOp::Range),
        TokenKind::DotDotEq => Ok(BinOp::RangeInclusive),
        TokenKind::QuestionQuestion => Ok(BinOp::NullCoalesce),
        TokenKind::Is => Ok(BinOp::Is),
        TokenKind::In => Ok(BinOp::In),
        _ => Err(ParseError {
            message: format!("not a binary operator: {:?}", kind),
            span: Span::default(),
        }),
    }
}

// Implementations for complex expression parsing will be added next
impl<'a> Parser<'a> {
    /// Parses function call arguments.
    fn parse_call_args(&mut self) -> ParseResult<Vec<Arg>> {
        if self.check(TokenKind::RParen) {
            return Ok(Vec::new());
        }

        self.parse_comma_list(TokenKind::RParen, |parser| {
            let start_pos = parser.pos;

            // Check for named argument: name = value
            if let TokenKind::Ident(_) = parser.current_kind() {
                if parser.peek_kind(1) == TokenKind::Eq {
                    let name = parser.expect_ident()?;
                    parser.expect(TokenKind::Eq)?;
                    let value = parser.parse_expr()?;
                    return Ok(Arg {
                        name: Some(name),
                        value,
                    });
                }
            }

            // Positional argument
            let value = parser.parse_expr()?;
            Ok(Arg { name: None, value })
        })
    }

    /// Parses a path starting from an identifier.
    fn parse_path_from_ident(&mut self, start_ident: Ident) -> ParseResult<Path> {
        let mut segments = vec![PathSegment::new(start_ident, None)];
        let start_span = start_ident.span;

        while self.eat(TokenKind::ColonColon) {
            let ident = self.expect_ident()?;
            // TODO: parse generic arguments after ident
            segments.push(PathSegment::new(ident, None));
        }

        let span = start_span.to(segments.last().unwrap().ident.span.end());
        Ok(Path::new(segments, span))
    }

    /// Parses a parenthesized expression or tuple.
    fn parse_paren_or_tuple_expr(&mut self) -> ParseResult<Expr> {
        let start_pos = self.pos;
        let start_span = self.current().span;
        self.expect(TokenKind::LParen)?;

        // Empty tuple: ()
        if self.check(TokenKind::RParen) {
            self.advance();
            let span = start_span.to(self.tokens[self.pos - 1].span.end());
            return Ok(Expr::new(
                self.next_node_id(),
                ExprKind::Tuple(Vec::new()),
                span,
            ));
        }

        let first_expr = self.parse_expr()?;

        // Check for comma (indicates tuple)
        if self.eat(TokenKind::Comma) {
            let mut elements = vec![first_expr];

            // Parse remaining elements
            while !self.check(TokenKind::RParen) && !self.is_eof() {
                elements.push(self.parse_expr()?);
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }

            self.expect(TokenKind::RParen)?;
            let span = start_span.to(self.tokens[self.pos - 1].span.end());
            Ok(Expr::new(
                self.next_node_id(),
                ExprKind::Tuple(elements),
                span,
            ))
        } else {
            // Single parenthesized expression
            self.expect(TokenKind::RParen)?;
            Ok(first_expr)
        }
    }

    /// Parses an array literal or array comprehension.
    fn parse_array_expr(&mut self) -> ParseResult<Expr> {
        let start_pos = self.pos;
        let start_span = self.current().span;
        self.expect(TokenKind::LBracket)?;

        // Empty array: []
        if self.check(TokenKind::RBracket) {
            self.advance();
            let span = start_span.to(self.tokens[self.pos - 1].span.end());
            return Ok(Expr::new(
                self.next_node_id(),
                ExprKind::Array(Vec::new()),
                span,
            ));
        }

        let first_expr = self.parse_expr()?;

        // Check for semicolon (array repeat: [value; count])
        if self.eat(TokenKind::Semicolon) {
            let count = self.parse_expr()?;
            self.expect(TokenKind::RBracket)?;
            let span = start_span.to(self.tokens[self.pos - 1].span.end());
            return Ok(Expr::new(
                self.next_node_id(),
                ExprKind::ArrayRepeat {
                    value: Box::new(first_expr),
                    count: Box::new(count),
                },
                span,
            ));
        }

        // Check for comprehension: [expr for ...]
        if self.check(TokenKind::For) {
            let comprehensions = self.parse_comprehensions()?;
            self.expect(TokenKind::RBracket)?;
            let span = start_span.to(self.tokens[self.pos - 1].span.end());
            return Ok(Expr::new(
                self.next_node_id(),
                ExprKind::ListComp {
                    element: Box::new(first_expr),
                    comprehensions,
                },
                span,
            ));
        }

        // Regular array literal
        let mut elements = vec![first_expr];
        if self.eat(TokenKind::Comma) {
            while !self.check(TokenKind::RBracket) && !self.is_eof() {
                elements.push(self.parse_expr()?);
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
        }

        self.expect(TokenKind::RBracket)?;
        let span = start_span.to(self.tokens[self.pos - 1].span.end());
        Ok(Expr::new(
            self.next_node_id(),
            ExprKind::Array(elements),
            span,
        ))
    }

    /// Parses a dict or set literal.
    fn parse_dict_or_set_expr(&mut self) -> ParseResult<Expr> {
        let start_pos = self.pos;
        let start_span = self.current().span;
        self.expect(TokenKind::LBrace)?;

        // Empty dict: {}
        if self.check(TokenKind::RBrace) {
            self.advance();
            let span = start_span.to(self.tokens[self.pos - 1].span.end());
            return Ok(Expr::new(
                self.next_node_id(),
                ExprKind::Dict(Vec::new()),
                span,
            ));
        }

        let first_expr = self.parse_expr()?;

        // Check for colon (dict) or comma/for (set)
        if self.eat(TokenKind::Colon) {
            // Dict literal or comprehension
            let first_value = self.parse_expr()?;

            // Check for comprehension
            if self.check(TokenKind::For) {
                let comprehensions = self.parse_comprehensions()?;
                self.expect(TokenKind::RBrace)?;
                let span = start_span.to(self.tokens[self.pos - 1].span.end());
                return Ok(Expr::new(
                    self.next_node_id(),
                    ExprKind::DictComp {
                        key: Box::new(first_expr),
                        value: Box::new(first_value),
                        comprehensions,
                    },
                    span,
                ));
            }

            // Regular dict literal
            let mut entries = vec![(first_expr, first_value)];
            if self.eat(TokenKind::Comma) {
                while !self.check(TokenKind::RBrace) && !self.is_eof() {
                    let key = self.parse_expr()?;
                    self.expect(TokenKind::Colon)?;
                    let value = self.parse_expr()?;
                    entries.push((key, value));
                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                }
            }

            self.expect(TokenKind::RBrace)?;
            let span = start_span.to(self.tokens[self.pos - 1].span.end());
            Ok(Expr::new(
                self.next_node_id(),
                ExprKind::Dict(entries),
                span,
            ))
        } else {
            // Set literal or comprehension
            // Check for comprehension
            if self.check(TokenKind::For) {
                let comprehensions = self.parse_comprehensions()?;
                self.expect(TokenKind::RBrace)?;
                let span = start_span.to(self.tokens[self.pos - 1].span.end());
                return Ok(Expr::new(
                    self.next_node_id(),
                    ExprKind::SetComp {
                        element: Box::new(first_expr),
                        comprehensions,
                    },
                    span,
                ));
            }

            // Regular set literal
            let mut elements = vec![first_expr];
            if self.eat(TokenKind::Comma) {
                while !self.check(TokenKind::RBrace) && !self.is_eof() {
                    elements.push(self.parse_expr()?);
                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                }
            }

            self.expect(TokenKind::RBrace)?;
            let span = start_span.to(self.tokens[self.pos - 1].span.end());
            Ok(Expr::new(self.next_node_id(), ExprKind::Set(elements), span))
        }
    }

    /// Parses comprehension clauses.
    fn parse_comprehensions(&mut self) -> ParseResult<Vec<Comprehension>> {
        let mut comprehensions = Vec::new();

        while self.check(TokenKind::For) {
            self.advance();
            let pattern = self.parse_pattern()?;
            self.expect(TokenKind::In)?;
            let iter = self.parse_expr()?;

            let mut filters = Vec::new();
            while self.check(TokenKind::If) {
                self.advance();
                filters.push(self.parse_expr()?);
            }

            comprehensions.push(Comprehension {
                pattern,
                iter,
                filters,
            });
        }

        Ok(comprehensions)
    }

    /// Parses a lambda expression.
    fn parse_lambda_expr(&mut self) -> ParseResult<Expr> {
        let start_pos = self.pos;
        let start_span = self.current().span;
        self.expect(TokenKind::Lambda)?;

        // Parse parameters
        let params = if self.check(TokenKind::Colon) {
            // No parameters
            Vec::new()
        } else {
            self.parse_lambda_params()?
        };

        self.expect(TokenKind::Colon)?;

        // Parse body
        let body = Box::new(self.parse_expr()?);
        let span = start_span.to(body.span.end());

        Ok(Expr::new(
            self.next_node_id(),
            ExprKind::Lambda {
                params,
                return_type: None,
                body,
                captures: Vec::new(), // Capture analysis done in semantic pass
            },
            span,
        ))
    }

    /// Parses lambda parameters.
    fn parse_lambda_params(&mut self) -> ParseResult<Vec<FunctionParam>> {
        let mut params = Vec::new();

        loop {
            if self.check(TokenKind::Colon) {
                break;
            }

            let name = self.expect_ident()?;
            let ty = if self.eat(TokenKind::Colon) {
                Some(self.parse_type()?)
            } else {
                None
            };

            params.push(FunctionParam {
                id: self.next_node_id(),
                pattern: Pattern::new(
                    self.next_node_id(),
                    PatternKind::Ident {
                        name: name.clone(),
                        mutable: false,
                        subpattern: None,
                    },
                    name.span,
                ),
                ty,
                default: None,
            });

            if !self.eat(TokenKind::Comma) {
                break;
            }
        }

        Ok(params)
    }

    /// Parses an if expression.
    fn parse_if_expr(&mut self) -> ParseResult<Expr> {
        let start_pos = self.pos;
        let start_span = self.current().span;
        self.expect(TokenKind::If)?;

        let condition = Box::new(self.parse_expr()?);

        // Expect colon or 'then'
        if !self.eat(TokenKind::Colon) {
            self.eat(TokenKind::Then);
        }

        let then_branch = Box::new(self.parse_expr()?);

        // Parse elif branches
        let mut elif_branches = Vec::new();
        while self.check(TokenKind::Elif) {
            self.advance();
            let elif_cond = self.parse_expr()?;
            if !self.eat(TokenKind::Colon) {
                self.eat(TokenKind::Then);
            }
            let elif_body = self.parse_expr()?;
            elif_branches.push((elif_cond, elif_body));
        }

        // Parse else branch
        let else_branch = if self.eat(TokenKind::Else) {
            if !self.eat(TokenKind::Colon) {
                // No colon needed after else in some cases
            }
            Some(Box::new(self.parse_expr()?))
        } else {
            None
        };

        let span = if let Some(ref else_expr) = else_branch {
            start_span.to(else_expr.span)
        } else if let Some((_, ref last_elif)) = elif_branches.last() {
            start_span.to(last_elif.span)
        } else {
            start_span.to(then_branch.span)
        };

        Ok(Expr::new(
            self.next_node_id(),
            ExprKind::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
            },
            span,
        ))
    }

    /// Parses a match expression.
    fn parse_match_expr(&mut self) -> ParseResult<Expr> {
        let start_pos = self.pos;
        let start_span = self.current().span;
        self.expect(TokenKind::Match)?;

        let scrutinee = Box::new(self.parse_expr()?);

        self.expect(TokenKind::Colon)?;
        self.skip_newlines();
        self.expect(TokenKind::Indent)?;

        let mut arms = Vec::new();

        while !self.check(TokenKind::Dedent) && !self.is_eof() {
            self.skip_newlines();
            if self.check(TokenKind::Dedent) {
                break;
            }

            arms.push(self.parse_match_arm()?);
            self.skip_newlines();
        }

        self.expect(TokenKind::Dedent)?;

        let span = self.span_from(start_pos);

        Ok(Expr::new(
            self.next_node_id(),
            ExprKind::Match { scrutinee, arms },
            span,
        ))
    }

    /// Parses a match arm.
    fn parse_match_arm(&mut self) -> ParseResult<MatchArm> {
        self.expect(TokenKind::Case)?;

        let pattern = self.parse_pattern()?;

        // Optional guard
        let guard = if self.check(TokenKind::If) {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };

        self.expect(TokenKind::Colon)?;

        let body = self.parse_expr()?;

        Ok(MatchArm {
            pattern,
            guard,
            body,
        })
    }

    /// Parses a pattern (placeholder - will be fully implemented in Part 14).
    fn parse_pattern(&mut self) -> ParseResult<Pattern> {
        let start_span = self.current().span;

        // For now, just parse identifiers
        if let TokenKind::Ident(name) = self.current_kind() {
            let ident = Ident::new(name, start_span);
            self.advance();

            Ok(Pattern::new(
                self.next_node_id(),
                PatternKind::Ident {
                    name: ident,
                    mutable: false,
                    subpattern: None,
                },
                start_span,
            ))
        } else {
            Err(self.error("expected pattern".to_string()))
        }
    }

    /// Parses a type expression (placeholder - will be fully implemented in Part 14).
    fn parse_type(&mut self) -> ParseResult<Type> {
        let start_span = self.current().span;

        // For now, just parse simple type paths
        if let TokenKind::Ident(name) = self.current_kind() {
            let ident = Ident::new(name, start_span);
            self.advance();
            let path = Path::from_ident(ident);

            Ok(Type::new(
                self.next_node_id(),
                TypeKind::Path(path),
                start_span,
            ))
        } else {
            Err(self.error("expected type".to_string()))
        }
    }
}
