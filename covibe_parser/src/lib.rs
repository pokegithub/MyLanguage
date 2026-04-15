//! Recursive-descent parser for the CoVibe programming language.
//!
//! This module implements a complete expression and statement parser using
//! the Pratt parsing algorithm for expressions with proper precedence handling.
//!
//! The parser transforms a stream of tokens from the lexer into an Abstract Syntax Tree (AST).

pub mod expr;
pub mod error;
pub mod stmt;

pub use expr::ExprParser;
pub use error::{ParseError, ParseResult};

use covibe_ast::{Item, Module, NodeId, NodeIdGen};
use covibe_lexer::token::{Token, TokenKind};
use covibe_lexer::Lexer;
use covibe_util::diagnostic::DiagnosticEngine;
use covibe_util::source::SourceFile;
use covibe_util::span::Span;

/// The main parser struct.
///
/// The parser maintains:
/// - A token stream (produced by the lexer)
/// - Current parsing position
/// - Node ID generator for AST nodes
/// - Diagnostic engine for error reporting
/// - Error recovery state
pub struct Parser<'a> {
    /// The source file being parsed.
    source: &'a SourceFile,
    /// Diagnostic engine for error reporting.
    diagnostics: &'a DiagnosticEngine,
    /// Token stream from the lexer.
    tokens: Vec<Token>,
    /// Current position in the token stream.
    pos: usize,
    /// Node ID generator.
    node_id_gen: NodeIdGen,
    /// Whether we're currently recovering from an error.
    recovering: bool,
}

impl<'a> Parser<'a> {
    /// Creates a new parser for the given source file.
    ///
    /// The parser will lex the entire source file upfront and store all tokens.
    /// This approach simplifies lookahead and error recovery.
    pub fn new(source: &'a SourceFile, diagnostics: &'a DiagnosticEngine) -> Self {
        let mut lexer = Lexer::new(source, diagnostics);
        let mut tokens = Vec::new();

        // Lex all tokens upfront
        loop {
            let token = lexer.next_token();
            let is_eof = matches!(token.kind, TokenKind::Eof);
            tokens.push(token);
            if is_eof {
                break;
            }
        }

        Self {
            source,
            diagnostics,
            tokens,
            pos: 0,
            node_id_gen: NodeIdGen::new(),
            recovering: false,
        }
    }

    /// Parses the source file into a module (AST root).
    ///
    /// A module is a collection of top-level items (functions, structs, etc.).
    /// For Part 12, we support parsing statements as top-level constructs.
    /// Full item parsing (functions, structs, etc.) will be added in Part 13.
    pub fn parse_module(&mut self) -> ParseResult<Module> {
        let start_pos = self.pos;
        let mut items = Vec::new();

        self.skip_newlines();

        while !self.is_eof() {
            self.skip_newlines();

            if self.is_eof() {
                break;
            }

            // Try to parse a top-level item or statement
            // For now (Part 12), we'll wrap statements as items
            // Full item parsing comes in Part 13
            match self.parse_top_level_item() {
                Ok(item) => {
                    items.push(item);
                }
                Err(e) => {
                    self.report_error(&e);
                    self.synchronize();
                }
            }

            self.skip_newlines();
        }

        let span = self.span_from(start_pos);
        Ok(Module {
            id: self.next_node_id(),
            items,
            span,
        })
    }

    /// Parses a top-level item.
    ///
    /// For Part 12, this is a placeholder that will be fully implemented in Part 13.
    /// Currently, it only recognizes item keywords and reports them as unimplemented.
    fn parse_top_level_item(&mut self) -> ParseResult<Item> {
        let start_pos = self.pos;

        // Check for item declaration keywords
        match self.current_kind() {
            TokenKind::Def
            | TokenKind::Struct
            | TokenKind::Enum
            | TokenKind::Trait
            | TokenKind::Impl
            | TokenKind::Type
            | TokenKind::Import
            | TokenKind::Export
            | TokenKind::Extern
            | TokenKind::Macro => {
                return Err(self.error(format!(
                    "item declarations will be implemented in Part 13, found {:?}",
                    self.current_kind()
                )));
            }
            _ => {}
        }

        // For now, we can't parse any items, so return an error
        Err(self.error(format!(
            "expected top-level item or declaration, found {:?}",
            self.current_kind()
        )))
    }

    // ===== Token Stream Management =====

    /// Returns the current token without consuming it.
    #[inline]
    pub(crate) fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    /// Returns the current token kind.
    #[inline]
    pub(crate) fn current_kind(&self) -> TokenKind {
        self.current().kind.clone()
    }

    /// Peeks ahead at the token at the given offset from current position.
    ///
    /// Returns TokenKind::Eof if the offset is beyond the end of the token stream.
    #[inline]
    pub(crate) fn peek(&self, offset: usize) -> &Token {
        let idx = self.pos + offset;
        if idx < self.tokens.len() {
            &self.tokens[idx]
        } else {
            self.tokens.last().unwrap() // Will always be Eof
        }
    }

    /// Peeks ahead at the token kind at the given offset.
    #[inline]
    pub(crate) fn peek_kind(&self, offset: usize) -> TokenKind {
        self.peek(offset).kind.clone()
    }

    /// Advances to the next token and returns the previous one.
    pub(crate) fn advance(&mut self) -> Token {
        let token = self.current().clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        token
    }

    /// Checks if the current token matches the given kind.
    #[inline]
    pub(crate) fn check(&self, kind: TokenKind) -> bool {
        self.current_kind() == kind
    }

    /// Checks if the current token matches any of the given kinds.
    #[inline]
    pub(crate) fn check_any(&self, kinds: &[TokenKind]) -> bool {
        kinds.iter().any(|k| self.check(k.clone()))
    }

    /// Consumes the current token if it matches the given kind, returning true if matched.
    pub(crate) fn eat(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Expects the current token to be of the given kind and consumes it.
    ///
    /// If the token doesn't match, reports an error and returns an error result.
    pub(crate) fn expect(&mut self, kind: TokenKind) -> ParseResult<Token> {
        if self.check(kind.clone()) {
            Ok(self.advance())
        } else {
            Err(self.error(format!("expected {:?}, found {:?}", kind, self.current_kind())))
        }
    }

    /// Expects an identifier and returns it.
    pub(crate) fn expect_ident(&mut self) -> ParseResult<covibe_ast::Ident> {
        if let TokenKind::Ident(name) = self.current_kind() {
            let span = self.current().span;
            self.advance();
            Ok(covibe_ast::Ident::new(name, span))
        } else {
            Err(self.error(format!("expected identifier, found {:?}", self.current_kind())))
        }
    }

    /// Checks if we're at the end of the file.
    #[inline]
    pub(crate) fn is_eof(&self) -> bool {
        matches!(self.current_kind(), TokenKind::Eof)
    }

    // ===== Error Handling and Recovery =====

    /// Creates a parse error at the current token position.
    pub(crate) fn error(&self, message: String) -> ParseError {
        ParseError {
            message,
            span: self.current().span,
        }
    }

    /// Creates a parse error at a specific span.
    pub(crate) fn error_at(&self, message: String, span: Span) -> ParseError {
        ParseError { message, span }
    }

    /// Reports a parse error to the diagnostic engine.
    pub(crate) fn report_error(&self, error: &ParseError) {
        self.diagnostics.error(&error.message, error.span);
    }

    /// Attempts to recover from an error by skipping tokens until a synchronization point.
    ///
    /// Synchronization points are typically statement/expression boundaries:
    /// - Newlines
    /// - Statement keywords (let, def, if, while, etc.)
    /// - Right braces/brackets/parens at depth 0
    pub(crate) fn synchronize(&mut self) {
        self.recovering = true;

        while !self.is_eof() {
            // Stop at newlines
            if matches!(self.current_kind(), TokenKind::Newline) {
                self.advance();
                self.recovering = false;
                return;
            }

            // Stop at statement-starting keywords
            if matches!(
                self.current_kind(),
                TokenKind::Let
                    | TokenKind::Var
                    | TokenKind::Const
                    | TokenKind::Def
                    | TokenKind::Struct
                    | TokenKind::Enum
                    | TokenKind::Trait
                    | TokenKind::Impl
                    | TokenKind::Type
                    | TokenKind::If
                    | TokenKind::While
                    | TokenKind::For
                    | TokenKind::Loop
                    | TokenKind::Match
                    | TokenKind::Return
                    | TokenKind::Break
                    | TokenKind::Continue
            ) {
                self.recovering = false;
                return;
            }

            self.advance();
        }

        self.recovering = false;
    }

    // ===== Node ID Generation =====

    /// Generates a new unique node ID.
    #[inline]
    pub(crate) fn next_node_id(&mut self) -> NodeId {
        self.node_id_gen.next()
    }

    // ===== Utility Methods =====

    /// Skips any newline tokens.
    pub(crate) fn skip_newlines(&mut self) {
        while matches!(self.current_kind(), TokenKind::Newline) {
            self.advance();
        }
    }

    /// Parses a comma-separated list of items.
    ///
    /// The `parse_item` closure is called to parse each item.
    /// The list can optionally have a trailing comma.
    ///
    /// Example: `x, y, z` or `x, y, z,`
    pub(crate) fn parse_comma_list<T, F>(
        &mut self,
        terminator: TokenKind,
        mut parse_item: F,
    ) -> ParseResult<Vec<T>>
    where
        F: FnMut(&mut Self) -> ParseResult<T>,
    {
        let mut items = Vec::new();

        while !self.check(terminator.clone()) && !self.is_eof() {
            items.push(parse_item(self)?);

            // Allow trailing comma
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }

        Ok(items)
    }

    /// Returns the span from the given start position to the current position.
    pub(crate) fn span_from(&self, start_pos: usize) -> Span {
        let start_span = self.tokens[start_pos].span;
        let end_span = if self.pos > 0 {
            self.tokens[self.pos - 1].span
        } else {
            self.current().span
        };

        start_span.to(end_span)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_parser_creation() {
        let (_, _, _, parser) = create_parser("let x = 42");
        assert!(!parser.is_eof());
        assert_eq!(parser.current_kind(), TokenKind::Let);
    }

    #[test]
    fn test_parser_advance() {
        let (_, _, _, mut parser) = create_parser("let x = 42");
        assert_eq!(parser.current_kind(), TokenKind::Let);
        parser.advance();
        assert!(matches!(parser.current_kind(), TokenKind::Ident(_)));
    }

    #[test]
    fn test_parser_peek() {
        let (_, _, _, parser) = create_parser("let x = 42");
        assert_eq!(parser.current_kind(), TokenKind::Let);
        assert!(matches!(parser.peek_kind(1), TokenKind::Ident(_)));
        assert_eq!(parser.peek_kind(2), TokenKind::Eq);
    }

    #[test]
    fn test_parser_eat() {
        let (_, _, _, mut parser) = create_parser("let x");
        assert!(parser.eat(TokenKind::Let));
        assert!(!parser.eat(TokenKind::Var));
        assert!(matches!(parser.current_kind(), TokenKind::Ident(_)));
    }

    #[test]
    fn test_parser_skip_newlines() {
        let (_, _, _, mut parser) = create_parser("let\n\n\nx");
        parser.advance(); // skip 'let'
        assert_eq!(parser.current_kind(), TokenKind::Newline);
        parser.skip_newlines();
        assert!(matches!(parser.current_kind(), TokenKind::Ident(_)));
    }
}
