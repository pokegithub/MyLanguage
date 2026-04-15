//! Statement and control flow parsing.
//!
//! This module implements parsing for all statement forms in CoVibe, including:
//! - Expression statements
//! - Let/var/const declarations
//! - Assignment statements
//! - Control flow (if/elif/else, while, for, loop, match)
//! - Jump statements (break, continue, return, yield)
//! - Error handling (try/catch/finally, raise)
//! - Block parsing with indentation tracking
//! - Special statements (defer, drop, assert, with, spawn, select)

use crate::{ParseResult, Parser};
use covibe_ast::*;
use covibe_lexer::token::TokenKind;

impl<'a> Parser<'a> {
    /// Parses a block of statements.
    ///
    /// A block can be:
    /// 1. Indented block (Python-style):
    ///    ```
    ///    if condition:
    ///        statement1
    ///        statement2
    ///    ```
    /// 2. Brace-delimited block (optional):
    ///    ```
    ///    if condition { statement1; statement2 }
    ///    ```
    ///
    /// Returns a Block containing statements and an optional trailing expression.
    pub fn parse_block(&mut self) -> ParseResult<Block> {
        let start_pos = self.pos;
        let start_span = self.current().span;

        // Check for brace-delimited block
        if self.check(TokenKind::LeftBrace) {
            return self.parse_brace_block();
        }

        // Parse indented block
        self.parse_indented_block()
    }

    /// Parses a brace-delimited block: `{ stmt1; stmt2; ... }`
    fn parse_brace_block(&mut self) -> ParseResult<Block> {
        let start_pos = self.pos;
        self.expect(TokenKind::LeftBrace)?;
        self.skip_newlines();

        let mut stmts = Vec::new();
        let mut trailing_expr = None;

        while !self.check(TokenKind::RightBrace) && !self.is_eof() {
            self.skip_newlines();

            if self.check(TokenKind::RightBrace) {
                break;
            }

            // Try to parse a statement
            let stmt_start = self.pos;
            match self.parse_stmt() {
                Ok(stmt) => {
                    stmts.push(stmt);
                }
                Err(e) => {
                    self.report_error(&e);
                    self.synchronize();
                }
            }

            // In brace blocks, semicolons are optional but can separate statements
            self.eat(TokenKind::Semicolon);
            self.skip_newlines();
        }

        // Check if the last statement is actually an expression that serves as the block value
        if let Some(last_stmt) = stmts.last() {
            if let StmtKind::Expr(expr) = &last_stmt.kind {
                // If there's no semicolon, this is a trailing expression
                let prev_token = if self.pos > 0 {
                    &self.tokens[self.pos - 1]
                } else {
                    self.current()
                };

                if !matches!(prev_token.kind, TokenKind::Semicolon) {
                    trailing_expr = Some(Box::new(expr.clone()));
                    stmts.pop();
                }
            }
        }

        self.expect(TokenKind::RightBrace)?;

        let span = self.span_from(start_pos);
        Ok(Block::new(self.next_node_id(), stmts, trailing_expr, span))
    }

    /// Parses an indented block (Python-style).
    ///
    /// Expects:
    /// - Colon `:`
    /// - Newline
    /// - Indent token
    /// - Statements at the new indentation level
    /// - Dedent token
    fn parse_indented_block(&mut self) -> ParseResult<Block> {
        let start_pos = self.pos;

        // Expect colon
        self.expect(TokenKind::Colon)?;

        // Expect newline
        if !self.eat(TokenKind::Newline) {
            return Err(self.error("expected newline after ':'".to_string()));
        }

        // Expect indent
        if !self.eat(TokenKind::Indent) {
            return Err(self.error("expected indented block".to_string()));
        }

        let mut stmts = Vec::new();
        let mut trailing_expr = None;

        // Parse statements until we hit a dedent
        while !self.check(TokenKind::Dedent) && !self.is_eof() {
            self.skip_newlines();

            if self.check(TokenKind::Dedent) {
                break;
            }

            match self.parse_stmt() {
                Ok(stmt) => {
                    stmts.push(stmt);
                }
                Err(e) => {
                    self.report_error(&e);
                    self.synchronize();
                }
            }

            // Statements in indented blocks are separated by newlines
            if !self.eat(TokenKind::Newline) && !self.check(TokenKind::Dedent) && !self.is_eof() {
                // Allow missing newline before dedent
                if !self.check(TokenKind::Dedent) {
                    return Err(self.error("expected newline after statement".to_string()));
                }
            }
        }

        // Expect dedent
        self.expect(TokenKind::Dedent)?;

        // Check if the last statement is a trailing expression
        if let Some(last_stmt) = stmts.last() {
            if let StmtKind::Expr(expr) = &last_stmt.kind {
                trailing_expr = Some(Box::new(expr.clone()));
                stmts.pop();
            }
        }

        let span = self.span_from(start_pos);
        Ok(Block::new(self.next_node_id(), stmts, trailing_expr, span))
    }

    /// Parses a single statement.
    pub fn parse_stmt(&mut self) -> ParseResult<Stmt> {
        self.skip_newlines();

        let start_pos = self.pos;
        let start_span = self.current().span;

        // Match on the current token to determine statement type
        match self.current_kind() {
            TokenKind::Let => self.parse_let_stmt(),
            TokenKind::Var => self.parse_var_stmt(),
            TokenKind::Const => self.parse_const_stmt(),
            TokenKind::If => self.parse_if_stmt(),
            TokenKind::Match => self.parse_match_stmt(),
            TokenKind::While => self.parse_while_stmt(),
            TokenKind::For => self.parse_for_stmt(),
            TokenKind::Loop => self.parse_loop_stmt(),
            TokenKind::Break => self.parse_break_stmt(),
            TokenKind::Continue => self.parse_continue_stmt(),
            TokenKind::Return => self.parse_return_stmt(),
            TokenKind::Yield => self.parse_yield_stmt(),
            TokenKind::Defer => self.parse_defer_stmt(),
            TokenKind::Drop => self.parse_drop_stmt(),
            TokenKind::Assert => self.parse_assert_stmt(),
            TokenKind::Try => self.parse_try_stmt(),
            TokenKind::Raise => self.parse_raise_stmt(),
            TokenKind::With => self.parse_with_stmt(),
            TokenKind::Async => self.parse_async_stmt(),
            TokenKind::Spawn => self.parse_spawn_stmt(),
            TokenKind::Select => self.parse_select_stmt(),
            TokenKind::Unsafe => self.parse_unsafe_stmt(),
            TokenKind::Comptime => self.parse_comptime_stmt(),

            // Item declarations can appear in statement position (nested functions, etc.)
            TokenKind::Def | TokenKind::Struct | TokenKind::Enum | TokenKind::Trait
            | TokenKind::Impl | TokenKind::Type => {
                // These will be implemented in Part 13
                return Err(self.error("item declarations implemented in Part 13".to_string()));
            }

            // Otherwise, try to parse as expression statement or assignment
            _ => self.parse_expr_or_assign_stmt(),
        }
    }

    /// Parses a let statement: `let pattern [: type] = expr`
    fn parse_let_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::Let)?;

        // Check for `mut` modifier
        let mutable = self.eat(TokenKind::Mut);

        // Parse pattern (implemented in Part 14)
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

    /// Parses a var statement: `var pattern [: type] = expr`
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

    /// Parses a const statement: `const NAME [: type] = expr`
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

    /// Parses an if statement with optional elif and else branches.
    ///
    /// Syntax:
    /// ```
    /// if condition:
    ///     body
    /// elif condition2:
    ///     body2
    /// else:
    ///     else_body
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

        // Parse optional else branch
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
    /// ```
    /// match expr:
    ///     case pattern:
    ///         body
    ///     case pattern if guard:
    ///         body
    /// ```
    fn parse_match_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::Match)?;

        let scrutinee = self.parse_expr()?;

        // Expect colon and indented block of cases
        self.expect(TokenKind::Colon)?;
        self.expect(TokenKind::Newline)?;
        self.expect(TokenKind::Indent)?;

        let mut arms = Vec::new();

        while !self.check(TokenKind::Dedent) && !self.is_eof() {
            self.skip_newlines();

            if self.check(TokenKind::Dedent) {
                break;
            }

            // Parse match arm
            let arm = self.parse_match_arm()?;
            arms.push(arm);

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

    /// Parses a match arm: `case pattern [if guard]: body`
    fn parse_match_arm(&mut self) -> ParseResult<MatchArm> {
        let start_pos = self.pos;
        self.expect(TokenKind::Case)?;

        let pattern = self.parse_pattern()?;

        // Optional guard
        let guard = if self.eat(TokenKind::If) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let body = self.parse_block()?;

        let span = self.span_from(start_pos);
        Ok(MatchArm {
            id: self.next_node_id(),
            pattern,
            guard,
            body,
            span,
        })
    }

    /// Parses a while loop: `while condition: body`
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

    /// Parses a for loop: `for pattern in iter: body`
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
            StmtKind::For { pattern, iter, body },
            span,
        ))
    }

    /// Parses an infinite loop: `loop: body`
    fn parse_loop_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::Loop)?;

        let body = self.parse_block()?;

        let span = self.span_from(start_pos);
        Ok(Stmt::new(self.next_node_id(), StmtKind::Loop { body }, span))
    }

    /// Parses a break statement: `break [expr]`
    fn parse_break_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::Break)?;

        // Optional break value
        let value = if !self.check(TokenKind::Newline)
            && !self.check(TokenKind::Semicolon)
            && !self.is_eof()
        {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let span = self.span_from(start_pos);
        Ok(Stmt::new(self.next_node_id(), StmtKind::Break(value), span))
    }

    /// Parses a continue statement: `continue`
    fn parse_continue_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::Continue)?;

        let span = self.span_from(start_pos);
        Ok(Stmt::new(self.next_node_id(), StmtKind::Continue, span))
    }

    /// Parses a return statement: `return [expr]`
    fn parse_return_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::Return)?;

        let value = if !self.check(TokenKind::Newline)
            && !self.check(TokenKind::Semicolon)
            && !self.is_eof()
        {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let span = self.span_from(start_pos);
        Ok(Stmt::new(self.next_node_id(), StmtKind::Return(value), span))
    }

    /// Parses a yield statement: `yield [expr]`
    fn parse_yield_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::Yield)?;

        let value = if !self.check(TokenKind::Newline)
            && !self.check(TokenKind::Semicolon)
            && !self.is_eof()
        {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let span = self.span_from(start_pos);
        Ok(Stmt::new(self.next_node_id(), StmtKind::Yield(value), span))
    }

    /// Parses a defer statement: `defer stmt`
    fn parse_defer_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::Defer)?;

        let stmt = Box::new(self.parse_stmt()?);

        let span = self.span_from(start_pos);
        Ok(Stmt::new(self.next_node_id(), StmtKind::Defer(stmt), span))
    }

    /// Parses a drop statement: `drop(expr)`
    fn parse_drop_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::Drop)?;

        self.expect(TokenKind::LeftParen)?;
        let value = self.parse_expr()?;
        self.expect(TokenKind::RightParen)?;

        let span = self.span_from(start_pos);
        Ok(Stmt::new(self.next_node_id(), StmtKind::Drop(value), span))
    }

    /// Parses an assert statement: `assert condition [, message]`
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
    /// ```
    /// try:
    ///     body
    /// catch ErrorType as e:
    ///     handler
    /// finally:
    ///     cleanup
    /// ```
    fn parse_try_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::Try)?;

        let body = self.parse_block()?;

        // Parse catch clauses
        let mut catch_clauses = Vec::new();
        while self.check(TokenKind::Catch) {
            let catch_start = self.pos;
            self.advance();

            // Optional error type
            let error_type = if !self.check(TokenKind::As) && !self.check(TokenKind::Colon) {
                Some(self.parse_type()?)
            } else {
                None
            };

            // Optional binding
            let binding = if self.eat(TokenKind::As) {
                Some(self.parse_pattern()?)
            } else {
                None
            };

            let handler = self.parse_block()?;
            let catch_span = self.span_from(catch_start);

            catch_clauses.push(CatchClause {
                id: self.next_node_id(),
                error_type,
                binding,
                body: handler,
                span: catch_span,
            });
        }

        // Optional finally block
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

    /// Parses a raise statement: `raise [expr]`
    fn parse_raise_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::Raise)?;

        let value = if !self.check(TokenKind::Newline)
            && !self.check(TokenKind::Semicolon)
            && !self.is_eof()
        {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let span = self.span_from(start_pos);
        Ok(Stmt::new(self.next_node_id(), StmtKind::Raise(value), span))
    }

    /// Parses a with statement (context manager).
    ///
    /// Syntax:
    /// ```
    /// with file = open("path.txt"):
    ///     body
    /// with open("path.txt") as file, open("other.txt") as other:
    ///     body
    /// ```
    fn parse_with_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::With)?;

        let mut items = Vec::new();

        // Parse comma-separated with items
        loop {
            let item_start = self.pos;

            let context = self.parse_expr()?;

            // Optional binding
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

    /// Parses an async block: `async { ... }` or `async: ...`
    fn parse_async_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::Async)?;

        let body = self.parse_block()?;

        let span = self.span_from(start_pos);
        Ok(Stmt::new(self.next_node_id(), StmtKind::Async(body), span))
    }

    /// Parses a spawn statement: `spawn expr`
    fn parse_spawn_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::Spawn)?;

        let expr = self.parse_expr()?;

        let span = self.span_from(start_pos);
        Ok(Stmt::new(self.next_node_id(), StmtKind::Spawn(expr), span))
    }

    /// Parses a select statement (for channel operations).
    ///
    /// Syntax:
    /// ```
    /// select:
    ///     recv value from channel:
    ///         body
    ///     send value to channel:
    ///         body
    ///     default:
    ///         body
    /// ```
    fn parse_select_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::Select)?;

        self.expect(TokenKind::Colon)?;
        self.expect(TokenKind::Newline)?;
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
        let start_pos = self.pos;

        let kind = if self.eat(TokenKind::Recv) {
            // recv pattern from channel
            let pattern = self.parse_pattern()?;
            self.expect(TokenKind::From)?;
            let channel = self.parse_expr()?;
            SelectArmKind::Recv { pattern, channel }
        } else if self.eat(TokenKind::Send) {
            // send value to channel
            let value = self.parse_expr()?;
            self.expect(TokenKind::To)?;
            let channel = self.parse_expr()?;
            SelectArmKind::Send { value, channel }
        } else if self.eat(TokenKind::Default) {
            SelectArmKind::Default
        } else {
            return Err(self.error("expected 'recv', 'send', or 'default' in select arm".to_string()));
        };

        let body = self.parse_block()?;

        let span = self.span_from(start_pos);
        Ok(SelectArm {
            id: self.next_node_id(),
            kind,
            body,
            span,
        })
    }

    /// Parses an unsafe block: `unsafe { ... }` or `unsafe: ...`
    fn parse_unsafe_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::Unsafe)?;

        let body = self.parse_block()?;

        let span = self.span_from(start_pos);
        Ok(Stmt::new(self.next_node_id(), StmtKind::Unsafe(body), span))
    }

    /// Parses a comptime block: `comptime { ... }` or `comptime: ...`
    fn parse_comptime_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;
        self.expect(TokenKind::Comptime)?;

        let body = self.parse_block()?;

        let span = self.span_from(start_pos);
        Ok(Stmt::new(self.next_node_id(), StmtKind::Comptime(body), span))
    }

    /// Parses either an expression statement or an assignment statement.
    ///
    /// We need to distinguish between:
    /// - `x + y` (expression statement)
    /// - `x = y` (assignment)
    /// - `x += y` (compound assignment)
    ///
    /// Strategy: Parse an expression first, then check if it's followed by `=` or `op=`.
    fn parse_expr_or_assign_stmt(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.pos;

        let expr = self.parse_expr()?;

        // Check for assignment operators
        if self.check(TokenKind::Eq)
            || self.check(TokenKind::PlusEq)
            || self.check(TokenKind::MinusEq)
            || self.check(TokenKind::StarEq)
            || self.check(TokenKind::SlashEq)
            || self.check(TokenKind::PercentEq)
            || self.check(TokenKind::AmpEq)
            || self.check(TokenKind::PipeEq)
            || self.check(TokenKind::CaretEq)
            || self.check(TokenKind::LtLtEq)
            || self.check(TokenKind::GtGtEq)
        {
            let op_token = self.advance();

            let value = self.parse_expr()?;

            // For compound assignments (+=, -=, etc.), desugar to binary op + assignment
            let final_value = match op_token.kind {
                TokenKind::Eq => value,
                TokenKind::PlusEq => {
                    let bin_op = BinaryOp::Add;
                    Expr::new(
                        self.next_node_id(),
                        ExprKind::Binary {
                            left: Box::new(expr.clone()),
                            op: bin_op,
                            right: Box::new(value),
                        },
                        self.span_from(start_pos),
                    )
                }
                TokenKind::MinusEq => {
                    let bin_op = BinaryOp::Sub;
                    Expr::new(
                        self.next_node_id(),
                        ExprKind::Binary {
                            left: Box::new(expr.clone()),
                            op: bin_op,
                            right: Box::new(value),
                        },
                        self.span_from(start_pos),
                    )
                }
                TokenKind::StarEq => {
                    let bin_op = BinaryOp::Mul;
                    Expr::new(
                        self.next_node_id(),
                        ExprKind::Binary {
                            left: Box::new(expr.clone()),
                            op: bin_op,
                            right: Box::new(value),
                        },
                        self.span_from(start_pos),
                    )
                }
                TokenKind::SlashEq => {
                    let bin_op = BinaryOp::Div;
                    Expr::new(
                        self.next_node_id(),
                        ExprKind::Binary {
                            left: Box::new(expr.clone()),
                            op: bin_op,
                            right: Box::new(value),
                        },
                        self.span_from(start_pos),
                    )
                }
                TokenKind::PercentEq => {
                    let bin_op = BinaryOp::Rem;
                    Expr::new(
                        self.next_node_id(),
                        ExprKind::Binary {
                            left: Box::new(expr.clone()),
                            op: bin_op,
                            right: Box::new(value),
                        },
                        self.span_from(start_pos),
                    )
                }
                TokenKind::AmpEq => {
                    let bin_op = BinaryOp::BitAnd;
                    Expr::new(
                        self.next_node_id(),
                        ExprKind::Binary {
                            left: Box::new(expr.clone()),
                            op: bin_op,
                            right: Box::new(value),
                        },
                        self.span_from(start_pos),
                    )
                }
                TokenKind::PipeEq => {
                    let bin_op = BinaryOp::BitOr;
                    Expr::new(
                        self.next_node_id(),
                        ExprKind::Binary {
                            left: Box::new(expr.clone()),
                            op: bin_op,
                            right: Box::new(value),
                        },
                        self.span_from(start_pos),
                    )
                }
                TokenKind::CaretEq => {
                    let bin_op = BinaryOp::BitXor;
                    Expr::new(
                        self.next_node_id(),
                        ExprKind::Binary {
                            left: Box::new(expr.clone()),
                            op: bin_op,
                            right: Box::new(value),
                        },
                        self.span_from(start_pos),
                    )
                }
                TokenKind::LtLtEq => {
                    let bin_op = BinaryOp::Shl;
                    Expr::new(
                        self.next_node_id(),
                        ExprKind::Binary {
                            left: Box::new(expr.clone()),
                            op: bin_op,
                            right: Box::new(value),
                        },
                        self.span_from(start_pos),
                    )
                }
                TokenKind::GtGtEq => {
                    let bin_op = BinaryOp::Shr;
                    Expr::new(
                        self.next_node_id(),
                        ExprKind::Binary {
                            left: Box::new(expr.clone()),
                            op: bin_op,
                            right: Box::new(value),
                        },
                        self.span_from(start_pos),
                    )
                }
                _ => unreachable!(),
            };

            let span = self.span_from(start_pos);
            return Ok(Stmt::new(
                self.next_node_id(),
                StmtKind::Assign {
                    target: expr,
                    value: final_value,
                },
                span,
            ));
        }

        // Otherwise, it's an expression statement
        let span = self.span_from(start_pos);
        Ok(Stmt::new(
            self.next_node_id(),
            StmtKind::Expr(expr),
            span,
        ))
    }

    /// Placeholder for pattern parsing (implemented in Part 14).
    fn parse_pattern(&mut self) -> ParseResult<Pattern> {
        // For now, only support simple identifier patterns
        let ident = self.expect_ident()?;
        Ok(Pattern {
            id: self.next_node_id(),
            kind: PatternKind::Ident {
                name: ident,
                mutable: false,
            },
            span: ident.span,
        })
    }

    /// Placeholder for type parsing (implemented in Part 14).
    fn parse_type(&mut self) -> ParseResult<Type> {
        // Simple type parsing: just identifier for now
        let ident = self.expect_ident()?;
        Ok(Type {
            id: self.next_node_id(),
            kind: TypeKind::Path(Path::from_ident(ident)),
            span: ident.span,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;
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
        match stmt.kind {
            StmtKind::Let { .. } => {}
            _ => panic!("expected let statement"),
        }
    }

    #[test]
    fn test_parse_let_mut_stmt() {
        let (_, _, _, mut parser) = create_parser("let mut x = 42");
        let stmt = parser.parse_stmt().unwrap();
        match stmt.kind {
            StmtKind::Let { mutable, .. } => {
                assert!(mutable);
            }
            _ => panic!("expected let statement"),
        }
    }

    #[test]
    fn test_parse_var_stmt() {
        let (_, _, _, mut parser) = create_parser("var x = 10");
        let stmt = parser.parse_stmt().unwrap();
        match stmt.kind {
            StmtKind::Var { .. } => {}
            _ => panic!("expected var statement"),
        }
    }

    #[test]
    fn test_parse_const_stmt() {
        let (_, _, _, mut parser) = create_parser("const PI = 3.14");
        let stmt = parser.parse_stmt().unwrap();
        match stmt.kind {
            StmtKind::Const { .. } => {}
            _ => panic!("expected const statement"),
        }
    }

    #[test]
    fn test_parse_break_stmt() {
        let (_, _, _, mut parser) = create_parser("break");
        let stmt = parser.parse_stmt().unwrap();
        match stmt.kind {
            StmtKind::Break(None) => {}
            _ => panic!("expected break statement"),
        }
    }

    #[test]
    fn test_parse_break_with_value() {
        let (_, _, _, mut parser) = create_parser("break 42");
        let stmt = parser.parse_stmt().unwrap();
        match stmt.kind {
            StmtKind::Break(Some(_)) => {}
            _ => panic!("expected break statement with value"),
        }
    }

    #[test]
    fn test_parse_continue_stmt() {
        let (_, _, _, mut parser) = create_parser("continue");
        let stmt = parser.parse_stmt().unwrap();
        match stmt.kind {
            StmtKind::Continue => {}
            _ => panic!("expected continue statement"),
        }
    }

    #[test]
    fn test_parse_return_stmt() {
        let (_, _, _, mut parser) = create_parser("return");
        let stmt = parser.parse_stmt().unwrap();
        match stmt.kind {
            StmtKind::Return(None) => {}
            _ => panic!("expected return statement"),
        }
    }

    #[test]
    fn test_parse_return_with_value() {
        let (_, _, _, mut parser) = create_parser("return 42");
        let stmt = parser.parse_stmt().unwrap();
        match stmt.kind {
            StmtKind::Return(Some(_)) => {}
            _ => panic!("expected return statement with value"),
        }
    }

    #[test]
    fn test_parse_brace_block() {
        let (_, _, _, mut parser) = create_parser("{ let x = 1; let y = 2 }");
        let block = parser.parse_brace_block().unwrap();
        assert_eq!(block.stmts.len(), 2);
    }

    #[test]
    fn test_parse_expr_stmt() {
        let (_, _, _, mut parser) = create_parser("x + y");
        let stmt = parser.parse_stmt().unwrap();
        match stmt.kind {
            StmtKind::Expr(_) => {}
            _ => panic!("expected expression statement"),
        }
    }

    #[test]
    fn test_parse_assignment() {
        let (_, _, _, mut parser) = create_parser("x = 42");
        let stmt = parser.parse_stmt().unwrap();
        match stmt.kind {
            StmtKind::Assign { .. } => {}
            _ => panic!("expected assignment statement"),
        }
    }

    #[test]
    fn test_parse_compound_assignment() {
        let (_, _, _, mut parser) = create_parser("x += 10");
        let stmt = parser.parse_stmt().unwrap();
        match stmt.kind {
            StmtKind::Assign { .. } => {}
            _ => panic!("expected assignment statement"),
        }
    }

    #[test]
    fn test_parse_assert_stmt() {
        let (_, _, _, mut parser) = create_parser("assert x > 0");
        let stmt = parser.parse_stmt().unwrap();
        match stmt.kind {
            StmtKind::Assert { .. } => {}
            _ => panic!("expected assert statement"),
        }
    }

    #[test]
    fn test_parse_spawn_stmt() {
        let (_, _, _, mut parser) = create_parser("spawn foo()");
        let stmt = parser.parse_stmt().unwrap();
        match stmt.kind {
            StmtKind::Spawn(_) => {}
            _ => panic!("expected spawn statement"),
        }
    }

    #[test]
    fn test_parse_defer_stmt() {
        let (_, _, _, mut parser) = create_parser("defer close(f)");
        let stmt = parser.parse_stmt().unwrap();
        match stmt.kind {
            StmtKind::Defer(_) => {}
            _ => panic!("expected defer statement"),
        }
    }

    #[test]
    fn test_parse_drop_stmt() {
        let (_, _, _, mut parser) = create_parser("drop(x)");
        let stmt = parser.parse_stmt().unwrap();
        match stmt.kind {
            StmtKind::Drop(_) => {}
            _ => panic!("expected drop statement"),
        }
    }

    #[test]
    fn test_parse_raise_stmt() {
        let (_, _, _, mut parser) = create_parser("raise");
        let stmt = parser.parse_stmt().unwrap();
        match stmt.kind {
            StmtKind::Raise(None) => {}
            _ => panic!("expected raise statement"),
        }
    }
}
