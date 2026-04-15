//! Statement and block parsing.
//!
//! This module implements parsing for all statement types in CoVibe, including:
//! - Let/var/const bindings
//! - Control flow statements (if, while, for, loop, match)
//! - Blocks with indentation tracking
//! - Try/catch/finally error handling
//! - Defer, drop, assert
//! - With statements (context managers)
//! - Select statements (channel operations)
//! - Async, unsafe, comptime blocks

use super::{ParseError, ParseResult, Parser};
use covibe_ast::*;
use covibe_lexer::token::TokenKind;
use covibe_util::span::Span;

/// Statement parser trait for the main Parser.
pub trait StmtParser {
    /// Parses a single statement.
    fn parse_stmt(&mut self) -> ParseResult<Stmt>;

    /// Parses a block of statements.
    fn parse_block(&mut self) -> ParseResult<Block>;

    /// Parses a block without requiring explicit indentation tokens.
    fn parse_block_expr(&mut self) -> ParseResult<Block>;
}

impl<'a> StmtParser for Parser<'a> {
    /// Parses a statement.
    ///
    /// This is the main entry point for statement parsing.
    /// It dispatches to specific statement parsers based on the current token.
    fn parse_stmt(&mut self) -> ParseResult<Stmt> {
        self.skip_newlines();

        let start_pos = self.pos;
        let start_span = self.current().span;

        match self.current_kind() {
            // Let binding
            TokenKind::Let => self.parse_let_stmt(),

            // Var declaration
            TokenKind::Var => self.parse_var_stmt(),

            // Const declaration
            TokenKind::Const => self.parse_const_stmt(),

            // If statement
            TokenKind::If => self.parse_if_stmt(),

            // Match statement
            TokenKind::Match => self.parse_match_stmt(),

            // While loop
            TokenKind::While => self.parse_while_stmt(),

            // For loop
            TokenKind::For => self.parse_for_stmt(),

            // Infinite loop
            TokenKind::Loop => self.parse_loop_stmt(),

            // Break statement
            TokenKind::Break => {
                self.advance();
                let value = if is_stmt_terminator(&self.current_kind()) {
                    None
                } else {
                    Some(self.parse_expr()?)
                };
                let span = self.span_from(start_pos);
                Ok(Stmt::new(self.next_node_id(), StmtKind::Break(value), span))
            }

            // Continue statement
            TokenKind::Continue => {
                self.advance();
                let span = self.span_from(start_pos);
                Ok(Stmt::new(
                    self.next_node_id(),
                    StmtKind::Continue,
                    span,
                ))
            }

            // Return statement
            TokenKind::Return => {
                self.advance();
                let value = if is_stmt_terminator(&self.current_kind()) {
                    None
                } else {
                    Some(self.parse_expr()?)
                };
                let span = self.span_from(start_pos);
                Ok(Stmt::new(
                    self.next_node_id(),
                    StmtKind::Return(value),
                    span,
                ))
            }

            // Yield statement
            TokenKind::Yield => {
                self.advance();
                let value = if is_stmt_terminator(&self.current_kind()) {
                    None
                } else {
                    Some(self.parse_expr()?)
                };
                let span = self.span_from(start_pos);
                Ok(Stmt::new(self.next_node_id(), StmtKind::Yield(value), span))
            }

            // Defer statement
            TokenKind::Defer => self.parse_defer_stmt(),

            // Drop statement
            TokenKind::Drop => {
                self.advance();
                let expr = self.parse_expr()?;
                let span = self.span_from(start_pos);
                Ok(Stmt::new(self.next_node_id(), StmtKind::Drop(expr), span))
            }

            // Assert statement
            TokenKind::Assert => self.parse_assert_stmt(),

            // Try/catch/finally
            TokenKind::Try => self.parse_try_stmt(),

            // Raise/throw
            TokenKind::Raise | TokenKind::Throw => {
                self.advance();
                let value = if is_stmt_terminator(&self.current_kind()) {
                    None
                } else {
                    Some(self.parse_expr()?)
                };
                let span = self.span_from(start_pos);
                Ok(Stmt::new(self.next_node_id(), StmtKind::Raise(value), span))
            }

            // With statement
            TokenKind::With => self.parse_with_stmt(),

            // Async block
            TokenKind::Async => {
                self.advance();
                let block = self.parse_block()?;
                let span = self.span_from(start_pos);
                Ok(Stmt::new(
                    self.next_node_id(),
                    StmtKind::Async(block),
                    span,
                ))
            }

            // Spawn statement
            TokenKind::Spawn => {
                self.advance();
                let expr = self.parse_expr()?;
                let span = self.span_from(start_pos);
                Ok(Stmt::new(self.next_node_id(), StmtKind::Spawn(expr), span))
            }

            // Select statement
            TokenKind::Select => self.parse_select_stmt(),

            // Unsafe block
            TokenKind::Unsafe => {
                self.advance();
                let block = self.parse_block()?;
                let span = self.span_from(start_pos);
                Ok(Stmt::new(
                    self.next_node_id(),
                    StmtKind::Unsafe(block),
                    span,
                ))
            }

            // Comptime block
            TokenKind::Comptime => {
                self.advance();
                let block = self.parse_block()?;
                let span = self.span_from(start_pos);
                Ok(Stmt::new(
                    self.next_node_id(),
                    StmtKind::Comptime(block),
                    span,
                ))
            }

            // Empty statement (lone semicolon or newline)
            TokenKind::Semicolon => {
                self.advance();
                Ok(Stmt::new(
                    self.next_node_id(),
                    StmtKind::Empty,
                    start_span,
                ))
            }

            // Expression statement or assignment
            _ => self.parse_expr_or_assign_stmt(),
        }
    }

    /// Parses a block of statements with explicit indentation tracking.
    ///
    /// Expected syntax:
    /// ```covibe
    /// :
    ///     statement1
    ///     statement2
    ///     optional_trailing_expr
    /// ```
    fn parse_block(&mut self) -> ParseResult<Block> {
        let start_pos = self.pos;
        let start_span = self.current().span;

        // Expect colon
        self.expect(TokenKind::Colon)?;
        self.skip_newlines();

        // Expect indent
        self.expect(TokenKind::Indent)?;

        let mut stmts = Vec::new();
        let mut trailing_expr = None;

        while !self.check(TokenKind::Dedent) && !self.is_eof() {
            self.skip_newlines();

            if self.check(TokenKind::Dedent) {
                break;
            }

            // Try to parse a statement
            let stmt = self.parse_stmt()?;

            // Check if this is an expression statement that could be a trailing expression
            if let StmtKind::Expr(expr) = stmt.kind {
                // Look ahead to see if we're at the end of the block
                self.skip_newlines();
                if self.check(TokenKind::Dedent) || self.is_eof() {
                    // This is the trailing expression
                    trailing_expr = Some(Box::new(expr));
                    break;
                } else {
                    // Not at the end, so it's a regular expression statement
                    stmts.push(Stmt::new(stmt.id, StmtKind::Expr(expr), stmt.span));
                }
            } else {
                stmts.push(stmt);
            }

            // Skip trailing semicolons and newlines
            while self.eat(TokenKind::Semicolon) || self.eat(TokenKind::Newline) {
                // Continue
            }
        }

        self.expect(TokenKind::Dedent)?;

        let span = self.span_from(start_pos);

        Ok(Block::new(
            self.next_node_id(),
            stmts,
            trailing_expr,
            span,
        ))
    }

    /// Parses a block expression without requiring indentation tokens.
    ///
    /// This is used for single-line blocks or expression contexts.
    fn parse_block_expr(&mut self) -> ParseResult<Block> {
        let start_pos = self.pos;

        // Parse a single expression as the block body
        let expr = self.parse_expr()?;
        let span = self.span_from(start_pos);

        Ok(Block::new(
            self.next_node_id(),
            Vec::new(),
            Some(Box::new(expr)),
            span,
        ))
    }
}

// ===== Statement Parsing Implementations =====

impl<'a> Parser<'a> {
    /// Parses a let binding statement.
    ///
    /// Syntax:
    /// ```covibe
    /// let x = 5
    /// let mut x: Int = 5
    /// let (a, b) = tuple
    /// ```
    fn parse_let_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::Let)?;

        // Check for 'mut'
        let mutable = self.eat(TokenKind::Mut);

        // Parse pattern
        let pattern = self.parse_pattern()?;

        // Optional type annotation
        let ty = if self.eat(TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        // Optional initializer
        let init = if self.eat(TokenKind::Eq) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let span = self.span_from(start_pos);

        Ok(Stmt::new(
            self.next_node_id(),
            StmtKind::Let {
                pattern,
                ty,
                init,
                mutable,
            },
            span,
        ))
    }

    /// Parses a var declaration statement.
    ///
    /// Syntax: `var x: Int = 5`
    fn parse_var_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::Var)?;

        let pattern = self.parse_pattern()?;

        let ty = if self.eat(TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let init = if self.eat(TokenKind::Eq) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let span = self.span_from(start_pos);

        Ok(Stmt::new(
            self.next_node_id(),
            StmtKind::Var { pattern, ty, init },
            span,
        ))
    }

    /// Parses a const declaration statement.
    ///
    /// Syntax: `const PI = 3.14159`
    fn parse_const_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::Const)?;

        let name = self.expect_ident()?;

        let ty = if self.eat(TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(TokenKind::Eq)?;
        let value = self.parse_expr()?;

        let span = self.span_from(start_pos);

        Ok(Stmt::new(
            self.next_node_id(),
            StmtKind::Const { name, ty, value },
            span,
        ))
    }

    /// Parses an if statement.
    ///
    /// Syntax:
    /// ```covibe
    /// if condition:
    ///     body
    /// elif condition2:
    ///     body2
    /// else:
    ///     body3
    /// ```
    fn parse_if_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::If)?;

        let condition = self.parse_expr()?;
        let then_branch = self.parse_block()?;

        // Parse elif branches
        let mut elif_branches = Vec::new();
        while self.check(TokenKind::Elif) {
            self.advance();
            let elif_cond = self.parse_expr()?;
            let elif_body = self.parse_block()?;
            elif_branches.push((elif_cond, elif_body));
        }

        // Parse else branch
        let else_branch = if self.eat(TokenKind::Else) {
            Some(self.parse_block()?)
        } else {
            None
        };

        let span = self.span_from(start_pos);

        Ok(Stmt::new(
            self.next_node_id(),
            StmtKind::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
            },
            span,
        ))
    }

    /// Parses a match statement.
    ///
    /// Syntax:
    /// ```covibe
    /// match value:
    ///     case Pattern1:
    ///         body1
    ///     case Pattern2 if guard:
    ///         body2
    /// ```
    fn parse_match_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::Match)?;

        let scrutinee = self.parse_expr()?;

        self.expect(TokenKind::Colon)?;
        self.skip_newlines();
        self.expect(TokenKind::Indent)?;

        let mut arms = Vec::new();

        while !self.check(TokenKind::Dedent) && !self.is_eof() {
            self.skip_newlines();
            if self.check(TokenKind::Dedent) {
                break;
            }

            arms.push(self.parse_match_arm_stmt()?);
            self.skip_newlines();
        }

        self.expect(TokenKind::Dedent)?;

        let span = self.span_from(start_pos);

        Ok(Stmt::new(
            self.next_node_id(),
            StmtKind::Match { scrutinee, arms },
            span,
        ))
    }

    /// Parses a match arm for match statements.
    fn parse_match_arm_stmt(&mut self) -> ParseResult<MatchArm> {
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

        // Parse the body - could be a block or a single expression
        let body = if self.check(TokenKind::Newline) {
            self.skip_newlines();
            if self.check(TokenKind::Indent) {
                // Multi-line block
                self.advance(); // consume INDENT
                let mut stmts = Vec::new();
                let mut trailing_expr = None;

                while !self.check(TokenKind::Dedent) && !self.is_eof() {
                    self.skip_newlines();
                    if self.check(TokenKind::Dedent) {
                        break;
                    }

                    let stmt = self.parse_stmt()?;

                    if let StmtKind::Expr(expr) = stmt.kind {
                        self.skip_newlines();
                        if self.check(TokenKind::Dedent) || self.is_eof() {
                            trailing_expr = Some(Box::new(expr));
                            break;
                        } else {
                            stmts.push(Stmt::new(stmt.id, StmtKind::Expr(expr), stmt.span));
                        }
                    } else {
                        stmts.push(stmt);
                    }

                    while self.eat(TokenKind::Semicolon) || self.eat(TokenKind::Newline) {}
                }

                self.expect(TokenKind::Dedent)?;

                // Convert to expression
                if stmts.is_empty() && trailing_expr.is_some() {
                    *trailing_expr.unwrap()
                } else {
                    // Create a block expression
                    let span = self.span_from(self.pos - 1);
                    Expr::new(
                        self.next_node_id(),
                        ExprKind::Block(Block::new(
                            self.next_node_id(),
                            stmts,
                            trailing_expr,
                            span,
                        )),
                        span,
                    )
                }
            } else {
                // Single-line expression after newline
                self.parse_expr()?
            }
        } else {
            // Inline expression
            self.parse_expr()?
        };

        Ok(MatchArm {
            pattern,
            guard,
            body,
        })
    }

    /// Parses a while loop statement.
    ///
    /// Syntax:
    /// ```covibe
    /// while condition:
    ///     body
    /// ```
    fn parse_while_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::While)?;

        let condition = self.parse_expr()?;
        let body = self.parse_block()?;

        let span = self.span_from(start_pos);

        Ok(Stmt::new(
            self.next_node_id(),
            StmtKind::While { condition, body },
            span,
        ))
    }

    /// Parses a for loop statement.
    ///
    /// Syntax:
    /// ```covibe
    /// for x in items:
    ///     body
    /// ```
    fn parse_for_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::For)?;

        let pattern = self.parse_pattern()?;

        self.expect(TokenKind::In)?;

        let iter = self.parse_expr()?;
        let body = self.parse_block()?;

        let span = self.span_from(start_pos);

        Ok(Stmt::new(
            self.next_node_id(),
            StmtKind::For {
                pattern,
                iter,
                body,
            },
            span,
        ))
    }

    /// Parses an infinite loop statement.
    ///
    /// Syntax:
    /// ```covibe
    /// loop:
    ///     body
    /// ```
    fn parse_loop_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::Loop)?;

        let body = self.parse_block()?;

        let span = self.span_from(start_pos);

        Ok(Stmt::new(self.next_node_id(), StmtKind::Loop { body }, span))
    }

    /// Parses a defer statement.
    ///
    /// Syntax: `defer close(file)`
    fn parse_defer_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::Defer)?;

        let deferred_stmt = Box::new(self.parse_stmt()?);

        let span = self.span_from(start_pos);

        Ok(Stmt::new(
            self.next_node_id(),
            StmtKind::Defer(deferred_stmt),
            span,
        ))
    }

    /// Parses an assert statement.
    ///
    /// Syntax:
    /// ```covibe
    /// assert condition
    /// assert condition, "error message"
    /// ```
    fn parse_assert_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::Assert)?;

        let condition = self.parse_expr()?;

        let message = if self.eat(TokenKind::Comma) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let span = self.span_from(start_pos);

        Ok(Stmt::new(
            self.next_node_id(),
            StmtKind::Assert { condition, message },
            span,
        ))
    }

    /// Parses a try/catch/finally statement.
    ///
    /// Syntax:
    /// ```covibe
    /// try:
    ///     body
    /// catch Error as e:
    ///     handler
    /// finally:
    ///     cleanup
    /// ```
    fn parse_try_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::Try)?;

        let body = self.parse_block()?;

        let mut catch_clauses = Vec::new();
        while self.check(TokenKind::Catch) {
            self.advance();

            // Parse error type (optional)
            let error_type = if is_stmt_terminator(&self.current_kind()) {
                None
            } else {
                Some(self.parse_type()?)
            };

            // Parse binding (e.g., 'as e')
            let binding = if self.eat(TokenKind::As) {
                Some(self.expect_ident()?)
            } else {
                None
            };

            let handler = self.parse_block()?;

            catch_clauses.push(CatchClause {
                error_type,
                binding,
                body: handler,
            });
        }

        let finally_block = if self.eat(TokenKind::Finally) {
            Some(self.parse_block()?)
        } else {
            None
        };

        let span = self.span_from(start_pos);

        Ok(Stmt::new(
            self.next_node_id(),
            StmtKind::Try {
                body,
                catch_clauses,
                finally_block,
            },
            span,
        ))
    }

    /// Parses a with statement (context manager).
    ///
    /// Syntax:
    /// ```covibe
    /// with file = open("data.txt"):
    ///     body
    /// with open("a.txt") as f1, open("b.txt") as f2:
    ///     body
    /// ```
    fn parse_with_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::With)?;

        let mut items = Vec::new();

        loop {
            let item_start = self.pos;
            let context = self.parse_expr()?;

            let binding = if self.eat(TokenKind::As) {
                Some(self.parse_pattern()?)
            } else {
                None
            };

            let item_span = self.span_from(item_start);

            items.push(WithItem {
                context,
                binding,
                span: item_span,
            });

            if !self.eat(TokenKind::Comma) {
                break;
            }
        }

        let body = self.parse_block()?;

        let span = self.span_from(start_pos);

        Ok(Stmt::new(
            self.next_node_id(),
            StmtKind::With { items, body },
            span,
        ))
    }

    /// Parses a select statement for channel operations.
    ///
    /// Syntax:
    /// ```covibe
    /// select:
    ///     recv value from channel1:
    ///         body1
    ///     send data to channel2:
    ///         body2
    ///     default:
    ///         body3
    /// ```
    fn parse_select_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::Select)?;

        self.expect(TokenKind::Colon)?;
        self.skip_newlines();
        self.expect(TokenKind::Indent)?;

        let mut arms = Vec::new();

        while !self.check(TokenKind::Dedent) && !self.is_eof() {
            self.skip_newlines();
            if self.check(TokenKind::Dedent) {
                break;
            }

            let arm = self.parse_select_arm()?;
            arms.push(arm);
            self.skip_newlines();
        }

        self.expect(TokenKind::Dedent)?;

        let span = self.span_from(start_pos);

        Ok(Stmt::new(
            self.next_node_id(),
            StmtKind::Select { arms },
            span,
        ))
    }

    /// Parses a select arm.
    fn parse_select_arm(&mut self) -> ParseResult<SelectArm> {
        let arm_start = self.pos;

        let kind = if self.check(TokenKind::Recv) {
            // recv value from channel
            self.advance();
            let pattern = self.parse_pattern()?;
            self.expect(TokenKind::From)?;
            let channel = self.parse_expr()?;
            SelectArmKind::Recv { pattern, channel }
        } else if self.check(TokenKind::Send) {
            // send value to channel
            self.advance();
            let value = self.parse_expr()?;
            self.expect(TokenKind::To)?;
            let channel = self.parse_expr()?;
            SelectArmKind::Send { value, channel }
        } else if self.check(TokenKind::Default) {
            // default case
            self.advance();
            SelectArmKind::Default
        } else {
            return Err(self.error("expected 'recv', 'send', or 'default' in select arm".to_string()));
        };

        self.expect(TokenKind::Colon)?;

        // Parse body block
        self.skip_newlines();
        if !self.check(TokenKind::Indent) {
            // Single expression on same line
            let expr = self.parse_expr()?;
            let span = self.span_from(arm_start);
            let block = Block::new(
                self.next_node_id(),
                Vec::new(),
                Some(Box::new(expr)),
                span,
            );
            return Ok(SelectArm {
                id: self.next_node_id(),
                kind,
                body: block,
                span,
            });
        }

        self.expect(TokenKind::Indent)?;

        let mut stmts = Vec::new();
        let mut trailing_expr = None;

        while !self.check(TokenKind::Dedent) && !self.is_eof() {
            self.skip_newlines();
            if self.check(TokenKind::Dedent) {
                break;
            }

            let stmt = self.parse_stmt()?;

            if let StmtKind::Expr(expr) = stmt.kind {
                self.skip_newlines();
                if self.check(TokenKind::Dedent) || self.is_eof() {
                    trailing_expr = Some(Box::new(expr));
                    break;
                } else {
                    stmts.push(Stmt::new(stmt.id, StmtKind::Expr(expr), stmt.span));
                }
            } else {
                stmts.push(stmt);
            }

            while self.eat(TokenKind::Semicolon) || self.eat(TokenKind::Newline) {}
        }

        self.expect(TokenKind::Dedent)?;

        let span = self.span_from(arm_start);
        let block = Block::new(self.next_node_id(), stmts, trailing_expr, span);

        Ok(SelectArm {
            id: self.next_node_id(),
            kind,
            body: block,
            span,
        })
    }

    /// Parses an expression statement or assignment.
    ///
    /// This distinguishes between:
    /// - Expression statements (e.g., `foo()`)
    /// - Assignment statements (handled by expression parser, wrapped here)
    fn parse_expr_or_assign_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;

        let expr = self.parse_expr()?;

        // Check if this is actually an assignment expression
        // Assignment expressions should be converted to assignment statements
        if let ExprKind::Assign { op, target, value } = expr.kind {
            // For walrus operator (:=), keep it as an expression
            if matches!(op, AssignOp::Walrus) {
                let span = self.span_from(start_pos);
                return Ok(Stmt::new(
                    self.next_node_id(),
                    StmtKind::Expr(Expr::new(
                        expr.id,
                        ExprKind::Assign {
                            op,
                            target,
                            value,
                        },
                        expr.span,
                    )),
                    span,
                ));
            }

            // Regular assignment - convert to assignment statement
            let span = self.span_from(start_pos);
            return Ok(Stmt::new(
                self.next_node_id(),
                StmtKind::Assign {
                    target: *target,
                    value: *value,
                },
                span,
            ));
        }

        let span = self.span_from(start_pos);
        Ok(Stmt::new(
            self.next_node_id(),
            StmtKind::Expr(expr),
            span,
        ))
    }
}

// ===== Helper Functions =====

/// Returns true if the token terminates a statement.
fn is_stmt_terminator(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Newline
            | TokenKind::Semicolon
            | TokenKind::Eof
            | TokenKind::Dedent
            | TokenKind::RBrace
            | TokenKind::RParen
            | TokenKind::RBracket
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ExprParser;
    use covibe_util::diagnostic::DiagnosticEngine;
    use covibe_util::source::{SourceFile, SourceMap};

    fn create_parser(input: &str) -> (SourceMap, DiagnosticEngine, SourceFile, Parser) {
        let source_map = SourceMap::new();
        let diagnostics = DiagnosticEngine::new(&source_map);
        let file = source_map.add_file("test.cv".to_string(), input.to_string());
        let source_file = source_map.get_file(file).unwrap();
        let parser = Parser::new(source_file, &diagnostics);
        (source_map, diagnostics, source_file.clone(), parser)
    }

    #[test]
    fn test_parse_let_stmt() {
        let (_, _, _, mut parser) = create_parser("let x = 42");
        let stmt = parser.parse_stmt().unwrap();
        assert!(matches!(stmt.kind, StmtKind::Let { .. }));
    }

    #[test]
    fn test_parse_let_mut_stmt() {
        let (_, _, _, mut parser) = create_parser("let mut x = 42");
        let stmt = parser.parse_stmt().unwrap();
        if let StmtKind::Let { mutable, .. } = stmt.kind {
            assert!(mutable);
        } else {
            panic!("Expected Let statement");
        }
    }

    #[test]
    fn test_parse_var_stmt() {
        let (_, _, _, mut parser) = create_parser("var x = 10");
        let stmt = parser.parse_stmt().unwrap();
        assert!(matches!(stmt.kind, StmtKind::Var { .. }));
    }

    #[test]
    fn test_parse_const_stmt() {
        let (_, _, _, mut parser) = create_parser("const PI = 3.14");
        let stmt = parser.parse_stmt().unwrap();
        assert!(matches!(stmt.kind, StmtKind::Const { .. }));
    }

    #[test]
    fn test_parse_break_stmt() {
        let (_, _, _, mut parser) = create_parser("break");
        let stmt = parser.parse_stmt().unwrap();
        assert!(matches!(stmt.kind, StmtKind::Break(None)));
    }

    #[test]
    fn test_parse_break_with_value() {
        let (_, _, _, mut parser) = create_parser("break 42");
        let stmt = parser.parse_stmt().unwrap();
        assert!(matches!(stmt.kind, StmtKind::Break(Some(_))));
    }

    #[test]
    fn test_parse_continue_stmt() {
        let (_, _, _, mut parser) = create_parser("continue");
        let stmt = parser.parse_stmt().unwrap();
        assert!(matches!(stmt.kind, StmtKind::Continue));
    }

    #[test]
    fn test_parse_return_stmt() {
        let (_, _, _, mut parser) = create_parser("return 42");
        let stmt = parser.parse_stmt().unwrap();
        assert!(matches!(stmt.kind, StmtKind::Return(Some(_))));
    }

    #[test]
    fn test_parse_expr_stmt() {
        let (_, _, _, mut parser) = create_parser("foo()");
        let stmt = parser.parse_stmt().unwrap();
        assert!(matches!(stmt.kind, StmtKind::Expr(_)));
    }

    #[test]
    fn test_parse_assign_stmt() {
        let (_, _, _, mut parser) = create_parser("x = 42");
        let stmt = parser.parse_stmt().unwrap();
        assert!(matches!(stmt.kind, StmtKind::Assign { .. }));
    }

    #[test]
    fn test_parse_assert_stmt() {
        let (_, _, _, mut parser) = create_parser("assert x > 0");
        let stmt = parser.parse_stmt().unwrap();
        assert!(matches!(stmt.kind, StmtKind::Assert { .. }));
    }

    #[test]
    fn test_parse_assert_with_message() {
        let (_, _, _, mut parser) = create_parser("assert x > 0, \"x must be positive\"");
        let stmt = parser.parse_stmt().unwrap();
        if let StmtKind::Assert { message, .. } = stmt.kind {
            assert!(message.is_some());
        } else {
            panic!("Expected Assert statement");
        }
    }
}
