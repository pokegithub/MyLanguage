//! Lexer implementation for the CoVibe programming language.

use crate::token::{FloatSuffix, IntSuffix, Literal, Token, TokenKind};
use covibe_util::diagnostic::{Diagnostic, DiagnosticEngine};
use covibe_util::source::SourceFile;
use covibe_util::span::Span;
use std::str::Chars;

/// The lexer state machine.
///
/// The lexer transforms a stream of UTF-8 characters into a sequence of tokens.
/// It maintains state for:
/// - Current position in the source file
/// - Indentation tracking (stack of indentation levels)
/// - Bracket/brace/parenthesis nesting depth (for ignoring indentation inside delimiters)
/// - Pending DEDENT tokens (emitted when indentation decreases)
pub struct Lexer<'a> {
    /// The source file being lexed
    source: &'a SourceFile,
    /// Character iterator
    chars: Chars<'a>,
    /// Current byte position in the source
    pos: usize,
    /// Current line number (0-indexed)
    line: usize,
    /// Current column number (0-indexed, in bytes)
    column: usize,
    /// Diagnostic engine for error reporting
    diagnostics: &'a DiagnosticEngine,
    /// Stack of indentation levels (in spaces/tabs)
    indent_stack: Vec<usize>,
    /// Whether we're at the beginning of a line
    at_line_start: bool,
    /// Nesting depth of (), [], {} (indentation is ignored when depth > 0)
    nesting_depth: usize,
    /// Pending DEDENT tokens to emit
    pending_dedents: usize,
    /// Character used for indentation (None = not determined, Some(' ') = spaces, Some('\t') = tabs)
    indent_char: Option<char>,
    /// Whether the last token was a newline
    last_was_newline: bool,
    /// Lookahead character (peeked but not consumed)
    peek_char: Option<char>,
}

impl<'a> Lexer<'a> {
    /// Creates a new lexer for the given source file.
    pub fn new(source: &'a SourceFile, diagnostics: &'a DiagnosticEngine) -> Self {
        let mut chars = source.source().chars();
        let peek_char = chars.next();

        Self {
            source,
            chars,
            pos: 0,
            line: 0,
            column: 0,
            diagnostics,
            indent_stack: vec![0],
            at_line_start: true,
            nesting_depth: 0,
            pending_dedents: 0,
            indent_char: None,
            last_was_newline: false,
            peek_char,
        }
    }

    /// Returns the next token from the input stream.
    pub fn next_token(&mut self) -> Token {
        loop {
            // Emit pending DEDENT tokens first
            if self.pending_dedents > 0 {
                self.pending_dedents -= 1;
                let span = self.current_span();
                return Token::new(TokenKind::Dedent, span);
            }

            // Handle indentation at the start of a line
            if self.at_line_start && self.nesting_depth == 0 {
                let result = self.handle_indentation_impl();
                if let Some(token) = result {
                    return token;
                }
                // If None, continue to scan the next token
                continue;
            }

            // Skip whitespace (except newlines)
            self.skip_horizontal_whitespace();

            let start_pos = self.pos;

            // Check for EOF
            let ch = match self.peek() {
                Some(c) => c,
                None => {
                    // Emit final DEDENTs before EOF
                    if self.indent_stack.len() > 1 {
                        self.indent_stack.pop();
                        self.pending_dedents = self.indent_stack.len() - 1;
                        if self.pending_dedents > 0 {
                            self.pending_dedents -= 1;
                            return Token::new(TokenKind::Dedent, self.current_span());
                        }
                    }
                    return Token::new(TokenKind::Eof, self.current_span());
                }
            };

            // Handle different token types
            let kind = match ch {
                // Newline
                '\n' | '\r' => self.scan_newline(),

                // Comments and Hash
                '#' => {
                    if self.peek_ahead(1) == Some('#') {
                        self.scan_doc_comment();
                        continue;  // Skip comment and continue loop
                    } else if self.peek_ahead(1) == Some('[') {
                        // This is a hash symbol for attributes, not a comment
                        self.advance();
                        TokenKind::Hash
                    } else {
                        self.scan_line_comment();
                        continue;  // Skip comment and continue loop
                    }
                }
                '/' if self.peek_ahead(1) == Some('*') => {
                    self.scan_block_comment();
                    continue;  // Skip comment and continue loop
                }

                // String literals
                '"' => self.scan_string_literal(),
                '\'' => self.scan_char_literal(),
                'r' if self.peek_ahead(1) == Some('"') || self.peek_ahead(1) == Some('#') => {
                    self.scan_raw_string()
                }
                'f' if self.peek_ahead(1) == Some('"') => self.scan_format_string(),
                'b' if self.peek_ahead(1) == Some('"') => self.scan_byte_string(),

                // Numbers
                '0'..='9' => self.scan_number(),
                '.' if matches!(self.peek_ahead(1), Some('0'..='9')) => self.scan_number(),

                // Identifiers and keywords
                'a'..='z' | 'A'..='Z' | '_' => self.scan_identifier_or_keyword(),

                // Unicode identifiers
                c if is_xid_start(c) => self.scan_identifier_or_keyword(),

                // Operators and punctuation
                '+' => self.scan_plus(),
                '-' => self.scan_minus(),
                '*' => self.scan_star(),
                '/' => self.scan_slash(),
                '%' => self.scan_percent(),
                '=' => self.scan_equals(),
                '!' => self.scan_bang(),
                '<' => self.scan_less_than(),
                '>' => self.scan_greater_than(),
                '&' => self.scan_ampersand(),
                '|' => self.scan_pipe(),
                '^' => self.scan_caret(),
                '~' => {
                    self.advance();
                    TokenKind::Tilde
                }
                '?' => self.scan_question(),
                ':' => self.scan_colon(),
                '.' => self.scan_dot(),
                '@' => self.scan_at(),
                '$' => {
                    self.advance();
                    TokenKind::Dollar
                }

                // Delimiters
                '(' => {
                    self.advance();
                    self.nesting_depth += 1;
                    TokenKind::LParen
                }
                ')' => {
                    self.advance();
                    if self.nesting_depth > 0 {
                        self.nesting_depth -= 1;
                    }
                    TokenKind::RParen
                }
                '[' => {
                    self.advance();
                    self.nesting_depth += 1;
                    TokenKind::LBracket
                }
                ']' => {
                    self.advance();
                    if self.nesting_depth > 0 {
                        self.nesting_depth -= 1;
                    }
                    TokenKind::RBracket
                }
                '{' => {
                    self.advance();
                    self.nesting_depth += 1;
                    TokenKind::LBrace
                }
                '}' => {
                    self.advance();
                    if self.nesting_depth > 0 {
                        self.nesting_depth -= 1;
                    }
                    TokenKind::RBrace
                }
                ',' => {
                    self.advance();
                    TokenKind::Comma
                }
                ';' => {
                    self.advance();
                    TokenKind::Semicolon
                }

                // Unrecognized character
                _ => {
                    self.advance();
                    self.error(format!("unexpected character '{}'", ch), start_pos, self.pos);
                    continue;  // Skip and continue loop
                }
            };

            let span = Span::with_file(self.source.id(), start_pos.into(), self.pos.into());
            return Token::new(kind, span);
        }
    }

    /// Peeks at the current character without consuming it.
    fn peek(&self) -> Option<char> {
        self.peek_char
    }

    /// Peeks ahead n characters.
    fn peek_ahead(&self, n: usize) -> Option<char> {
        if n == 0 {
            return self.peek();
        }
        let mut chars = self.chars.clone();
        for _ in 0..n - 1 {
            chars.next();
        }
        chars.next()
    }

    /// Advances to the next character.
    fn advance(&mut self) -> Option<char> {
        let ch = self.peek_char?;

        // Update position tracking
        self.pos += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
            self.column = 0;
        } else {
            self.column += ch.len_utf8();
        }

        // Update peek
        self.peek_char = self.chars.next();

        Some(ch)
    }

    /// Returns the current position as a span.
    fn current_span(&self) -> Span {
        Span::with_file(self.source.id(), self.pos.into(), self.pos.into())
    }

    /// Skips horizontal whitespace (spaces and tabs, but not newlines).
    fn skip_horizontal_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == ' ' || ch == '\t' {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Handles indentation at the start of a line.
    /// Returns Some(token) if we should emit a token, None if we should continue scanning.
    fn handle_indentation_impl(&mut self) -> Option<Token> {
        self.at_line_start = false;

        let start_pos = self.pos;
        let mut indent_count = 0;

        // Count indentation
        while let Some(ch) = self.peek() {
            if ch == ' ' || ch == '\t' {
                // Check for mixed indentation
                if let Some(indent_ch) = self.indent_char {
                    if ch != indent_ch {
                        self.error(
                            format!("mixed indentation: cannot mix spaces and tabs"),
                            start_pos,
                            self.pos,
                        );
                    }
                } else {
                    self.indent_char = Some(ch);
                }
                indent_count += 1;
                self.advance();
            } else {
                break;
            }
        }

        // Check for blank line or comment-only line
        if let Some(ch) = self.peek() {
            if ch == '\n' || ch == '\r' {
                // Skip blank line and continue
                self.scan_newline();
                return None;
            } else if ch == '#' {
                // Check if it's a hash token (e.g., #[attribute]) or a comment (# or ##)
                let next_ch = self.peek_ahead(1);
                if next_ch == Some('[') {
                    // It's a hash token (#[...]), not a comment - continue normally
                    // Don't set at_line_start, let it be processed as a normal token
                } else {
                    // It's a comment-only line (either # or ##) - skip it
                    // Don't set at_line_start to avoid infinite loop
                    // The comment will be handled in the main loop
                    return None;
                }
            }
        } else {
            // EOF on empty line
            return None;
        }

        // Compare with current indentation level
        let current_level = *self.indent_stack.last().unwrap();

        if indent_count > current_level {
            // Indent
            self.indent_stack.push(indent_count);
            Some(Token::new(TokenKind::Indent, Span::with_file(self.source.id(), start_pos.into(), self.pos.into())))
        } else if indent_count < current_level {
            // Dedent
            let mut dedent_count = 0;
            while let Some(&level) = self.indent_stack.last() {
                if level <= indent_count {
                    break;
                }
                self.indent_stack.pop();
                dedent_count += 1;
            }

            // Check if indentation matches a previous level
            if let Some(&level) = self.indent_stack.last() {
                if level != indent_count {
                    self.error(
                        format!("indentation does not match any previous indentation level"),
                        start_pos,
                        self.pos,
                    );
                }
            }

            self.pending_dedents = dedent_count - 1;
            Some(Token::new(TokenKind::Dedent, Span::with_file(self.source.id(), start_pos.into(), self.pos.into())))
        } else {
            // Same indentation level - continue to next token
            None
        }
    }

    /// Scans a newline character.
    fn scan_newline(&mut self) -> TokenKind {
        let ch = self.advance().unwrap();

        // Handle CRLF
        if ch == '\r' && self.peek() == Some('\n') {
            self.advance();
        }

        self.at_line_start = true;
        self.last_was_newline = true;
        TokenKind::Newline
    }

    /// Scans a line comment (starting with #).
    fn scan_line_comment(&mut self) {
        self.advance(); // consume '#'

        // Consume until end of line
        while let Some(ch) = self.peek() {
            if ch == '\n' || ch == '\r' {
                break;
            }
            self.advance();
        }
    }

    /// Scans a documentation comment (starting with ##).
    fn scan_doc_comment(&mut self) {
        self.advance(); // consume first '#'
        self.advance(); // consume second '#'

        // Consume until end of line
        while let Some(ch) = self.peek() {
            if ch == '\n' || ch == '\r' {
                break;
            }
            self.advance();
        }
        // Doc comments are also skipped for now
        // TODO: store doc comments for later processing
    }

    /// Scans a block comment (/* ... */).
    fn scan_block_comment(&mut self) {
        let start_pos = self.pos;
        self.advance(); // consume '/'
        self.advance(); // consume '*'

        let mut depth = 1;

        while depth > 0 {
            match self.peek() {
                Some('*') if self.peek_ahead(1) == Some('/') => {
                    self.advance();
                    self.advance();
                    depth -= 1;
                }
                Some('/') if self.peek_ahead(1) == Some('*') => {
                    self.advance();
                    self.advance();
                    depth += 1;
                }
                Some(_) => {
                    self.advance();
                }
                None => {
                    self.error(
                        "unterminated block comment".to_string(),
                        start_pos,
                        self.pos,
                    );
                    break;
                }
            }
        }
    }

    /// Scans a string literal.
    fn scan_string_literal(&mut self) -> TokenKind {
        let start_pos = self.pos;
        self.advance(); // consume opening '"'

        // Check for heredoc (triple-quoted string)
        if self.peek() == Some('"') && self.peek_ahead(1) == Some('"') {
            self.advance(); // consume second '"'
            self.advance(); // consume third '"'
            return self.scan_heredoc(start_pos);
        }

        let mut value = String::new();

        loop {
            match self.peek() {
                Some('"') => {
                    self.advance();
                    break;
                }
                Some('\\') => {
                    self.advance();
                    if let Some(ch) = self.scan_escape_sequence() {
                        value.push(ch);
                    }
                }
                Some('\n') | Some('\r') | None => {
                    self.error("unterminated string literal".to_string(), start_pos, self.pos);
                    break;
                }
                Some(ch) => {
                    value.push(ch);
                    self.advance();
                }
            }
        }

        TokenKind::Literal(Literal::String(value))
    }

    /// Scans a heredoc string literal (""" ... """).
    fn scan_heredoc(&mut self, start_pos: usize) -> TokenKind {
        let mut value = String::new();

        loop {
            match self.peek() {
                Some('"') if self.peek_ahead(1) == Some('"') && self.peek_ahead(2) == Some('"') => {
                    self.advance();
                    self.advance();
                    self.advance();
                    break;
                }
                Some(ch) => {
                    value.push(ch);
                    self.advance();
                }
                None => {
                    self.error("unterminated heredoc string".to_string(), start_pos, self.pos);
                    break;
                }
            }
        }

        TokenKind::Literal(Literal::Heredoc(value))
    }

    /// Scans a raw string literal (r"..." or r#"..."#).
    fn scan_raw_string(&mut self) -> TokenKind {
        let start_pos = self.pos;
        self.advance(); // consume 'r'

        // Count hash symbols
        let mut hash_count = 0;
        while self.peek() == Some('#') {
            hash_count += 1;
            self.advance();
        }

        // Expect opening quote
        if self.peek() != Some('"') {
            self.error("expected '\"' after 'r' in raw string".to_string(), start_pos, self.pos);
            return self.next_token().kind;
        }
        self.advance();

        let mut value = String::new();

        loop {
            match self.peek() {
                Some('"') => {
                    self.advance();

                    // Check for matching hash symbols
                    let mut matched_hashes = 0;
                    while matched_hashes < hash_count && self.peek() == Some('#') {
                        matched_hashes += 1;
                        self.advance();
                    }

                    if matched_hashes == hash_count {
                        break;
                    } else {
                        // Not the end - add the quote and hashes to the value
                        value.push('"');
                        for _ in 0..matched_hashes {
                            value.push('#');
                        }
                    }
                }
                Some(ch) => {
                    value.push(ch);
                    self.advance();
                }
                None => {
                    self.error("unterminated raw string literal".to_string(), start_pos, self.pos);
                    break;
                }
            }
        }

        TokenKind::Literal(Literal::RawString(hash_count, value))
    }

    /// Scans a format string literal (f"...{expr}...").
    fn scan_format_string(&mut self) -> TokenKind {
        let start_pos = self.pos;
        self.advance(); // consume 'f'
        self.advance(); // consume '"'

        let mut value = String::new();

        loop {
            match self.peek() {
                Some('"') => {
                    self.advance();
                    break;
                }
                Some('\\') => {
                    self.advance();
                    if let Some(ch) = self.scan_escape_sequence() {
                        value.push(ch);
                    }
                }
                Some('{') => {
                    // Handle interpolation
                    value.push('{');
                    self.advance();

                    // For now, we just scan through the interpolation
                    // The parser will handle the actual expression parsing
                    let mut brace_depth = 1;
                    while brace_depth > 0 {
                        match self.peek() {
                            Some('{') => {
                                value.push('{');
                                self.advance();
                                brace_depth += 1;
                            }
                            Some('}') => {
                                value.push('}');
                                self.advance();
                                brace_depth -= 1;
                            }
                            Some(ch) => {
                                value.push(ch);
                                self.advance();
                            }
                            None => {
                                self.error("unterminated interpolation in f-string".to_string(), start_pos, self.pos);
                                break;
                            }
                        }
                    }
                }
                Some('\n') | Some('\r') | None => {
                    self.error("unterminated f-string literal".to_string(), start_pos, self.pos);
                    break;
                }
                Some(ch) => {
                    value.push(ch);
                    self.advance();
                }
            }
        }

        TokenKind::Literal(Literal::FormatString(value))
    }

    /// Scans a byte string literal (b"...").
    fn scan_byte_string(&mut self) -> TokenKind {
        let start_pos = self.pos;
        self.advance(); // consume 'b'
        self.advance(); // consume '"'

        let mut bytes = Vec::new();

        loop {
            match self.peek() {
                Some('"') => {
                    self.advance();
                    break;
                }
                Some('\\') => {
                    self.advance();
                    if let Some(ch) = self.scan_escape_sequence() {
                        if ch.is_ascii() {
                            bytes.push(ch as u8);
                        } else {
                            self.error("non-ASCII character in byte string".to_string(), start_pos, self.pos);
                        }
                    }
                }
                Some('\n') | Some('\r') | None => {
                    self.error("unterminated byte string literal".to_string(), start_pos, self.pos);
                    break;
                }
                Some(ch) if ch.is_ascii() => {
                    bytes.push(ch as u8);
                    self.advance();
                }
                Some(_) => {
                    self.error("non-ASCII character in byte string".to_string(), start_pos, self.pos);
                    self.advance();
                }
            }
        }

        TokenKind::Literal(Literal::ByteString(bytes))
    }

    /// Scans a character literal.
    fn scan_char_literal(&mut self) -> TokenKind {
        let start_pos = self.pos;
        self.advance(); // consume opening '\''

        let ch = match self.peek() {
            Some('\\') => {
                self.advance();
                self.scan_escape_sequence()
            }
            Some('\'') | Some('\n') | Some('\r') | None => {
                self.error("empty character literal".to_string(), start_pos, self.pos);
                None
            }
            Some(ch) => {
                self.advance();
                Some(ch)
            }
        };

        // Expect closing quote
        if self.peek() != Some('\'') {
            self.error("unterminated character literal".to_string(), start_pos, self.pos);
        } else {
            self.advance();
        }

        TokenKind::Literal(Literal::Char(ch.unwrap_or('\0')))
    }

    /// Scans an escape sequence and returns the resulting character.
    fn scan_escape_sequence(&mut self) -> Option<char> {
        match self.peek()? {
            'n' => {
                self.advance();
                Some('\n')
            }
            'r' => {
                self.advance();
                Some('\r')
            }
            't' => {
                self.advance();
                Some('\t')
            }
            '\\' => {
                self.advance();
                Some('\\')
            }
            '\'' => {
                self.advance();
                Some('\'')
            }
            '"' => {
                self.advance();
                Some('"')
            }
            '0' => {
                self.advance();
                Some('\0')
            }
            'x' => {
                self.advance();
                self.scan_hex_escape(2)
            }
            'u' => {
                self.advance();
                if self.peek() == Some('{') {
                    self.advance();
                    let ch = self.scan_unicode_escape();
                    if self.peek() == Some('}') {
                        self.advance();
                    } else {
                        self.error("expected '}' after unicode escape".to_string(), self.pos, self.pos);
                    }
                    ch
                } else {
                    self.error("expected '{' after '\\u'".to_string(), self.pos, self.pos);
                    None
                }
            }
            ch => {
                self.error(format!("unknown escape sequence '\\{}'", ch), self.pos, self.pos);
                self.advance();
                None
            }
        }
    }

    /// Scans a hex escape sequence (\xHH).
    fn scan_hex_escape(&mut self, count: usize) -> Option<char> {
        let mut value = 0u32;
        for _ in 0..count {
            match self.peek() {
                Some(ch) if ch.is_ascii_hexdigit() => {
                    value = value * 16 + ch.to_digit(16).unwrap();
                    self.advance();
                }
                _ => {
                    self.error("invalid hex escape sequence".to_string(), self.pos, self.pos);
                    return None;
                }
            }
        }
        char::from_u32(value)
    }

    /// Scans a unicode escape sequence (\u{HHHHHH}).
    fn scan_unicode_escape(&mut self) -> Option<char> {
        let mut value = 0u32;
        let mut count = 0;

        while let Some(ch) = self.peek() {
            if ch.is_ascii_hexdigit() {
                value = value * 16 + ch.to_digit(16).unwrap();
                count += 1;
                self.advance();
                if count > 6 {
                    self.error("unicode escape sequence too long".to_string(), self.pos, self.pos);
                    return None;
                }
            } else {
                break;
            }
        }

        if count == 0 {
            self.error("empty unicode escape sequence".to_string(), self.pos, self.pos);
            return None;
        }

        char::from_u32(value)
    }

    /// Scans a number literal (integer or float).
    fn scan_number(&mut self) -> TokenKind {
        let start_pos = self.pos;

        // Check for special prefixes
        if self.peek() == Some('0') {
            match self.peek_ahead(1) {
                Some('b') | Some('B') => return self.scan_binary_literal(),
                Some('o') | Some('O') => return self.scan_octal_literal(),
                Some('x') | Some('X') => return self.scan_hex_literal(),
                _ => {}
            }
        }

        // Decimal number (could be int or float)
        let mut value = String::new();
        let mut is_float = false;

        // Handle leading dot for floats like .5
        if self.peek() == Some('.') {
            is_float = true;
            value.push('.');
            self.advance();
        }

        // Scan digits
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                value.push(ch);
                self.advance();
            } else if ch == '_' {
                self.advance(); // skip separator
            } else {
                break;
            }
        }

        // Check for decimal point
        if !is_float && self.peek() == Some('.') && self.peek_ahead(1).map(|c| c.is_ascii_digit()).unwrap_or(false) {
            is_float = true;
            value.push('.');
            self.advance();

            // Scan fractional part
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() {
                    value.push(ch);
                    self.advance();
                } else if ch == '_' {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // Check for exponent
        if let Some('e') | Some('E') = self.peek() {
            is_float = true;
            value.push('e');
            self.advance();

            if let Some('+') | Some('-') = self.peek() {
                value.push(self.advance().unwrap());
            }

            let exp_start = value.len();
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() {
                    value.push(ch);
                    self.advance();
                } else if ch == '_' {
                    self.advance();
                } else {
                    break;
                }
            }

            if value.len() == exp_start {
                self.error("expected digits after exponent".to_string(), start_pos, self.pos);
            }
        }

        // Check for suffix
        if is_float {
            let suffix = self.scan_float_suffix();
            TokenKind::Literal(Literal::Float(value, suffix))
        } else {
            let suffix = self.scan_int_suffix();
            TokenKind::Literal(Literal::Integer(value, suffix))
        }
    }

    /// Scans a binary literal (0b...).
    fn scan_binary_literal(&mut self) -> TokenKind {
        self.advance(); // consume '0'
        self.advance(); // consume 'b' or 'B'

        let mut value = String::from("0b");

        while let Some(ch) = self.peek() {
            if ch == '0' || ch == '1' {
                value.push(ch);
                self.advance();
            } else if ch == '_' {
                self.advance();
            } else {
                break;
            }
        }

        let suffix = self.scan_int_suffix();
        TokenKind::Literal(Literal::Integer(value, suffix))
    }

    /// Scans an octal literal (0o...).
    fn scan_octal_literal(&mut self) -> TokenKind {
        self.advance(); // consume '0'
        self.advance(); // consume 'o' or 'O'

        let mut value = String::from("0o");

        while let Some(ch) = self.peek() {
            if ch >= '0' && ch <= '7' {
                value.push(ch);
                self.advance();
            } else if ch == '_' {
                self.advance();
            } else {
                break;
            }
        }

        let suffix = self.scan_int_suffix();
        TokenKind::Literal(Literal::Integer(value, suffix))
    }

    /// Scans a hexadecimal literal (0x...).
    fn scan_hex_literal(&mut self) -> TokenKind {
        self.advance(); // consume '0'
        self.advance(); // consume 'x' or 'X'

        let mut value = String::from("0x");

        while let Some(ch) = self.peek() {
            if ch.is_ascii_hexdigit() {
                value.push(ch);
                self.advance();
            } else if ch == '_' {
                self.advance();
            } else {
                break;
            }
        }

        let suffix = self.scan_int_suffix();
        TokenKind::Literal(Literal::Integer(value, suffix))
    }

    /// Scans an integer type suffix.
    fn scan_int_suffix(&mut self) -> Option<IntSuffix> {
        let start_pos = self.pos;

        // Check for i8, i16, i32, i64, i128, isize
        if self.peek() == Some('i') {
            self.advance();
            if self.peek() == Some('8') {
                self.advance();
                return Some(IntSuffix::I8);
            } else if self.peek() == Some('1') && self.peek_ahead(1) == Some('6') {
                self.advance();
                self.advance();
                return Some(IntSuffix::I16);
            } else if self.peek() == Some('3') && self.peek_ahead(1) == Some('2') {
                self.advance();
                self.advance();
                return Some(IntSuffix::I32);
            } else if self.peek() == Some('6') && self.peek_ahead(1) == Some('4') {
                self.advance();
                self.advance();
                return Some(IntSuffix::I64);
            } else if self.peek() == Some('1') && self.peek_ahead(1) == Some('2') && self.peek_ahead(2) == Some('8') {
                self.advance();
                self.advance();
                self.advance();
                return Some(IntSuffix::I128);
            } else if self.peek() == Some('s') && self.peek_ahead(1) == Some('i') && self.peek_ahead(2) == Some('z') && self.peek_ahead(3) == Some('e') {
                self.advance();
                self.advance();
                self.advance();
                self.advance();
                return Some(IntSuffix::ISize);
            } else {
                // Reset if invalid suffix
                while self.pos > start_pos {
                    self.pos -= 1;
                }
            }
        }

        // Check for u8, u16, u32, u64, u128, usize
        if self.peek() == Some('u') {
            self.advance();
            if self.peek() == Some('8') {
                self.advance();
                return Some(IntSuffix::U8);
            } else if self.peek() == Some('1') && self.peek_ahead(1) == Some('6') {
                self.advance();
                self.advance();
                return Some(IntSuffix::U16);
            } else if self.peek() == Some('3') && self.peek_ahead(1) == Some('2') {
                self.advance();
                self.advance();
                return Some(IntSuffix::U32);
            } else if self.peek() == Some('6') && self.peek_ahead(1) == Some('4') {
                self.advance();
                self.advance();
                return Some(IntSuffix::U64);
            } else if self.peek() == Some('1') && self.peek_ahead(1) == Some('2') && self.peek_ahead(2) == Some('8') {
                self.advance();
                self.advance();
                self.advance();
                return Some(IntSuffix::U128);
            } else if self.peek() == Some('s') && self.peek_ahead(1) == Some('i') && self.peek_ahead(2) == Some('z') && self.peek_ahead(3) == Some('e') {
                self.advance();
                self.advance();
                self.advance();
                self.advance();
                return Some(IntSuffix::USize);
            } else {
                // Reset if invalid suffix
                while self.pos > start_pos {
                    self.pos -= 1;
                }
            }
        }

        None
    }

    /// Scans a float type suffix.
    fn scan_float_suffix(&mut self) -> Option<FloatSuffix> {
        if self.peek() == Some('f') {
            self.advance();
            if self.peek() == Some('3') && self.peek_ahead(1) == Some('2') {
                self.advance();
                self.advance();
                return Some(FloatSuffix::F32);
            } else if self.peek() == Some('6') && self.peek_ahead(1) == Some('4') {
                self.advance();
                self.advance();
                return Some(FloatSuffix::F64);
            }
        }
        None
    }

    /// Scans an identifier or keyword.
    fn scan_identifier_or_keyword(&mut self) -> TokenKind {
        let mut name = String::new();

        // First character (already validated as XID_Start)
        name.push(self.advance().unwrap());

        // Subsequent characters
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' || is_xid_continue(ch) {
                name.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Check if it's a keyword
        match name.as_str() {
            "def" => TokenKind::Def,
            "let" => TokenKind::Let,
            "var" => TokenKind::Var,
            "const" => TokenKind::Const,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "trait" => TokenKind::Trait,
            "impl" => TokenKind::Impl,
            "type" => TokenKind::Type,
            "class" => TokenKind::Class,
            "interface" => TokenKind::Interface,
            "if" => TokenKind::If,
            "elif" => TokenKind::Elif,
            "else" => TokenKind::Else,
            "match" => TokenKind::Match,
            "case" => TokenKind::Case,
            "for" => TokenKind::For,
            "while" => TokenKind::While,
            "loop" => TokenKind::Loop,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "return" => TokenKind::Return,
            "yield" => TokenKind::Yield,
            "await" => TokenKind::Await,
            "int" => TokenKind::Int,
            "float" => TokenKind::Float,
            "bool" => TokenKind::Bool,
            "str" => TokenKind::Str,
            "char" => TokenKind::Char,
            "i8" => TokenKind::I8,
            "i16" => TokenKind::I16,
            "i32" => TokenKind::I32,
            "i64" => TokenKind::I64,
            "i128" => TokenKind::I128,
            "isize" => TokenKind::ISize,
            "u8" => TokenKind::U8,
            "u16" => TokenKind::U16,
            "u32" => TokenKind::U32,
            "u64" => TokenKind::U64,
            "u128" => TokenKind::U128,
            "usize" => TokenKind::USize,
            "f32" => TokenKind::F32,
            "f64" => TokenKind::F64,
            "import" => TokenKind::Import,
            "from" => TokenKind::From,
            "as" => TokenKind::As,
            "export" => TokenKind::Export,
            "pub" => TokenKind::Pub,
            "priv" => TokenKind::Priv,
            "protected" => TokenKind::Protected,
            "ref" => TokenKind::Ref,
            "mut" => TokenKind::Mut,
            "move" => TokenKind::Move,
            "copy" => TokenKind::Copy,
            "clone" => TokenKind::Clone,
            "box" => TokenKind::Box,
            "alloc" => TokenKind::Alloc,
            "defer" => TokenKind::Defer,
            "drop" => TokenKind::Drop,
            "static" => TokenKind::Static,
            "unsafe" => TokenKind::Unsafe,
            "async" => TokenKind::Async,
            "spawn" => TokenKind::Spawn,
            "send" => TokenKind::Send,
            "recv" => TokenKind::Recv,
            "select" => TokenKind::Select,
            "to" => TokenKind::To,
            "default" => TokenKind::Default,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "none" => TokenKind::None,
            "null" => TokenKind::Null,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            "in" => TokenKind::In,
            "is" => TokenKind::Is,
            "self" => TokenKind::SelfLower,
            "Self" => TokenKind::SelfUpper,
            "super" => TokenKind::Super,
            "where" => TokenKind::Where,
            "with" => TokenKind::With,
            "try" => TokenKind::Try,
            "catch" => TokenKind::Catch,
            "finally" => TokenKind::Finally,
            "raise" => TokenKind::Raise,
            "throw" => TokenKind::Throw,
            "assert" => TokenKind::Assert,
            "lambda" => TokenKind::Lambda,
            "comptime" => TokenKind::Comptime,
            "macro" => TokenKind::Macro,
            "extern" => TokenKind::Extern,
            _ => TokenKind::Ident(name),
        }
    }

    // Operator scanning methods

    fn scan_plus(&mut self) -> TokenKind {
        self.advance();
        if self.peek() == Some('=') {
            self.advance();
            TokenKind::PlusEq
        } else {
            TokenKind::Plus
        }
    }

    fn scan_minus(&mut self) -> TokenKind {
        self.advance();
        match self.peek() {
            Some('=') => {
                self.advance();
                TokenKind::MinusEq
            }
            Some('>') => {
                self.advance();
                TokenKind::Arrow
            }
            _ => TokenKind::Minus,
        }
    }

    fn scan_star(&mut self) -> TokenKind {
        self.advance();
        match self.peek() {
            Some('*') => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::StarStarEq
                } else {
                    TokenKind::StarStar
                }
            }
            Some('=') => {
                self.advance();
                TokenKind::StarEq
            }
            _ => TokenKind::Star,
        }
    }

    fn scan_slash(&mut self) -> TokenKind {
        self.advance();
        match self.peek() {
            Some('/') => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::SlashSlashEq
                } else {
                    TokenKind::SlashSlash
                }
            }
            Some('=') => {
                self.advance();
                TokenKind::SlashEq
            }
            _ => TokenKind::Slash,
        }
    }

    fn scan_percent(&mut self) -> TokenKind {
        self.advance();
        if self.peek() == Some('=') {
            self.advance();
            TokenKind::PercentEq
        } else {
            TokenKind::Percent
        }
    }

    fn scan_equals(&mut self) -> TokenKind {
        self.advance();
        match self.peek() {
            Some('=') => {
                self.advance();
                TokenKind::EqEq
            }
            Some('>') => {
                self.advance();
                TokenKind::FatArrow
            }
            _ => TokenKind::Eq,
        }
    }

    fn scan_bang(&mut self) -> TokenKind {
        self.advance();
        if self.peek() == Some('=') {
            self.advance();
            TokenKind::BangEq
        } else {
            TokenKind::Bang
        }
    }

    fn scan_less_than(&mut self) -> TokenKind {
        self.advance();
        match self.peek() {
            Some('=') => {
                self.advance();
                if self.peek() == Some('>') {
                    self.advance();
                    TokenKind::Spaceship
                } else {
                    TokenKind::LtEq
                }
            }
            Some('<') => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::LtLtEq
                } else {
                    TokenKind::LtLt
                }
            }
            Some('|') => {
                self.advance();
                TokenKind::LtPipe
            }
            _ => TokenKind::Lt,
        }
    }

    fn scan_greater_than(&mut self) -> TokenKind {
        self.advance();
        match self.peek() {
            Some('=') => {
                self.advance();
                TokenKind::GtEq
            }
            Some('>') => {
                self.advance();
                match self.peek() {
                    Some('>') => {
                        self.advance();
                        if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::GtGtGtEq
                        } else {
                            TokenKind::GtGtGt
                        }
                    }
                    Some('=') => {
                        self.advance();
                        TokenKind::GtGtEq
                    }
                    _ => TokenKind::GtGt,
                }
            }
            _ => TokenKind::Gt,
        }
    }

    fn scan_ampersand(&mut self) -> TokenKind {
        self.advance();
        match self.peek() {
            Some('&') => {
                self.advance();
                TokenKind::AndAnd
            }
            Some('=') => {
                self.advance();
                TokenKind::AmpersandEq
            }
            _ => TokenKind::Ampersand,
        }
    }

    fn scan_pipe(&mut self) -> TokenKind {
        self.advance();
        match self.peek() {
            Some('|') => {
                self.advance();
                TokenKind::OrOr
            }
            Some('=') => {
                self.advance();
                TokenKind::PipeEq
            }
            Some('>') => {
                self.advance();
                TokenKind::PipeGt
            }
            _ => TokenKind::Pipe,
        }
    }

    fn scan_caret(&mut self) -> TokenKind {
        self.advance();
        if self.peek() == Some('=') {
            self.advance();
            TokenKind::CaretEq
        } else {
            TokenKind::Caret
        }
    }

    fn scan_question(&mut self) -> TokenKind {
        self.advance();
        match self.peek() {
            Some('?') => {
                self.advance();
                TokenKind::QuestionQuestion
            }
            Some(':') => {
                self.advance();
                TokenKind::QuestionColon
            }
            _ => TokenKind::Question,
        }
    }

    fn scan_colon(&mut self) -> TokenKind {
        self.advance();
        match self.peek() {
            Some(':') => {
                self.advance();
                TokenKind::ColonColon
            }
            Some('=') => {
                self.advance();
                TokenKind::ColonEq
            }
            _ => TokenKind::Colon,
        }
    }

    fn scan_dot(&mut self) -> TokenKind {
        self.advance();
        match self.peek() {
            Some('.') => {
                self.advance();
                match self.peek() {
                    Some('.') => {
                        self.advance();
                        TokenKind::DotDotDot
                    }
                    Some('=') => {
                        self.advance();
                        TokenKind::DotDotEq
                    }
                    _ => TokenKind::DotDot,
                }
            }
            _ => TokenKind::Dot,
        }
    }

    fn scan_at(&mut self) -> TokenKind {
        self.advance();

        // Check for raw identifier (@keyword)
        if let Some(ch) = self.peek() {
            if ch.is_ascii_alphabetic() || ch == '_' || is_xid_start(ch) {
                // This is a raw identifier - scan the keyword
                let kind = self.scan_identifier_or_keyword();
                // Convert keyword to identifier
                if let TokenKind::Ident(_) = kind {
                    return kind;
                } else {
                    // It was a keyword, convert to identifier
                    // We need to extract the keyword name
                    return TokenKind::Ident(format!("{}", kind).trim_matches('\'').to_string());
                }
            }
        }

        TokenKind::At
    }

    /// Reports an error.
    fn error(&self, message: String, start: usize, end: usize) {
        let span = Span::with_file(self.source.id(), start.into(), end.into());
        let diagnostic = Diagnostic::error(message, self.source.id(), span);
        self.diagnostics.emit(diagnostic);
    }
}

/// Checks if a character is a valid XID_Start character.
fn is_xid_start(ch: char) -> bool {
    unicode_xid::UnicodeXID::is_xid_start(ch)
}

/// Checks if a character is a valid XID_Continue character.
fn is_xid_continue(ch: char) -> bool {
    unicode_xid::UnicodeXID::is_xid_continue(ch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use covibe_util::source::{FileId, SourceMap};
    use std::path::PathBuf;

    fn lex_source(content: &str) -> Vec<Token> {
        let file_id = FileId::from_raw(0);
        let path = PathBuf::from("test.covibe");
        let source = SourceFile::new(file_id, path, content.to_string());
        let source_map = SourceMap::new();
        let diagnostics = DiagnosticEngine::new(source_map);
        let mut lexer = Lexer::new(&source, &diagnostics);

        let mut tokens = Vec::new();
        loop {
            let token = lexer.next_token();
            if token.kind == TokenKind::Eof {
                tokens.push(token);
                break;
            }
            tokens.push(token);
        }
        tokens
    }

    #[test]
    fn test_keywords() {
        let tokens = lex_source("def let var const");
        assert_eq!(tokens.len(), 5); // 4 keywords + EOF
        assert!(matches!(tokens[0].kind, TokenKind::Def));
        assert!(matches!(tokens[1].kind, TokenKind::Let));
        assert!(matches!(tokens[2].kind, TokenKind::Var));
        assert!(matches!(tokens[3].kind, TokenKind::Const));
    }

    #[test]
    fn test_identifiers() {
        let tokens = lex_source("foo bar _private");
        assert_eq!(tokens.len(), 4); // 3 identifiers + EOF
        assert!(matches!(tokens[0].kind, TokenKind::Ident(ref s) if s == "foo"));
        assert!(matches!(tokens[1].kind, TokenKind::Ident(ref s) if s == "bar"));
        assert!(matches!(tokens[2].kind, TokenKind::Ident(ref s) if s == "_private"));
    }

    #[test]
    fn test_integer_literals() {
        let tokens = lex_source("42 0b1010 0o755 0xDEAD");
        assert_eq!(tokens.len(), 5);
        assert!(matches!(tokens[0].kind, TokenKind::Literal(Literal::Integer(ref s, None)) if s == "42"));
        assert!(matches!(tokens[1].kind, TokenKind::Literal(Literal::Integer(ref s, None)) if s == "0b1010"));
        assert!(matches!(tokens[2].kind, TokenKind::Literal(Literal::Integer(ref s, None)) if s == "0o755"));
        assert!(matches!(tokens[3].kind, TokenKind::Literal(Literal::Integer(ref s, None)) if s == "0xDEAD"));
    }

    #[test]
    fn test_float_literals() {
        let tokens = lex_source("3.14 2.5e10 .5");
        assert_eq!(tokens.len(), 4);
        assert!(matches!(tokens[0].kind, TokenKind::Literal(Literal::Float(ref s, None)) if s == "3.14"));
        assert!(matches!(tokens[1].kind, TokenKind::Literal(Literal::Float(ref s, None)) if s == "2.5e10"));
        assert!(matches!(tokens[2].kind, TokenKind::Literal(Literal::Float(ref s, None)) if s == ".5"));
    }

    #[test]
    fn test_string_literals() {
        let tokens = lex_source(r#""hello" r"raw\nstring" f"name is {x}""#);
        assert_eq!(tokens.len(), 4);
        assert!(matches!(tokens[0].kind, TokenKind::Literal(Literal::String(ref s)) if s == "hello"));
        assert!(matches!(tokens[1].kind, TokenKind::Literal(Literal::RawString(0, ref s)) if s == r"raw\nstring"));
        assert!(matches!(tokens[2].kind, TokenKind::Literal(Literal::FormatString(_))));
    }

    #[test]
    fn test_operators() {
        let tokens = lex_source("+ - * / == != < <= > >= && || !");
        assert!(matches!(tokens[0].kind, TokenKind::Plus));
        assert!(matches!(tokens[1].kind, TokenKind::Minus));
        assert!(matches!(tokens[2].kind, TokenKind::Star));
        assert!(matches!(tokens[3].kind, TokenKind::Slash));
        assert!(matches!(tokens[4].kind, TokenKind::EqEq));
        assert!(matches!(tokens[5].kind, TokenKind::BangEq));
        assert!(matches!(tokens[6].kind, TokenKind::Lt));
        assert!(matches!(tokens[7].kind, TokenKind::LtEq));
        assert!(matches!(tokens[8].kind, TokenKind::Gt));
        assert!(matches!(tokens[9].kind, TokenKind::GtEq));
        assert!(matches!(tokens[10].kind, TokenKind::AndAnd));
        assert!(matches!(tokens[11].kind, TokenKind::OrOr));
        assert!(matches!(tokens[12].kind, TokenKind::Bang));
    }

    #[test]
    fn test_indentation() {
        let source = "def foo():\n    return 42";
        let tokens = lex_source(source);

        // Find INDENT token
        let has_indent = tokens.iter().any(|t| matches!(t.kind, TokenKind::Indent));
        assert!(has_indent, "Should have INDENT token");
    }

    #[test]
    fn test_line_comment() {
        let tokens = lex_source("let x = 42 # this is a comment\nlet y = 10");
        // Comments are skipped, so we should only see tokens for the code
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Ident(ref s) if s == "x")));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Ident(ref s) if s == "y")));
    }

    #[test]
    fn test_block_comment() {
        let tokens = lex_source("let x = /* block comment */ 42");
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Ident(ref s) if s == "x")));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Literal(Literal::Integer(ref s, None)) if s == "42")));
    }

    #[test]
    fn test_nested_block_comment() {
        let tokens = lex_source("let x = /* outer /* inner */ outer */ 42");
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Ident(ref s) if s == "x")));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Literal(Literal::Integer(ref s, None)) if s == "42")));
    }

    #[test]
    fn test_doc_comment() {
        let tokens = lex_source("## This is a doc comment\nlet x = 42");
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Ident(ref s) if s == "x")));
    }

    #[test]
    fn test_all_arithmetic_operators() {
        let tokens = lex_source("+ - * / // % **");
        assert!(matches!(tokens[0].kind, TokenKind::Plus));
        assert!(matches!(tokens[1].kind, TokenKind::Minus));
        assert!(matches!(tokens[2].kind, TokenKind::Star));
        assert!(matches!(tokens[3].kind, TokenKind::Slash));
        assert!(matches!(tokens[4].kind, TokenKind::SlashSlash));
        assert!(matches!(tokens[5].kind, TokenKind::Percent));
        assert!(matches!(tokens[6].kind, TokenKind::StarStar));
    }

    #[test]
    fn test_all_comparison_operators() {
        let tokens = lex_source("== != < <= > >= <=>");
        assert!(matches!(tokens[0].kind, TokenKind::EqEq));
        assert!(matches!(tokens[1].kind, TokenKind::BangEq));
        assert!(matches!(tokens[2].kind, TokenKind::Lt));
        assert!(matches!(tokens[3].kind, TokenKind::LtEq));
        assert!(matches!(tokens[4].kind, TokenKind::Gt));
        assert!(matches!(tokens[5].kind, TokenKind::GtEq));
        assert!(matches!(tokens[6].kind, TokenKind::Spaceship));
    }

    #[test]
    fn test_all_bitwise_operators() {
        let tokens = lex_source("& | ^ ~ << >> >>>");
        assert!(matches!(tokens[0].kind, TokenKind::Ampersand));
        assert!(matches!(tokens[1].kind, TokenKind::Pipe));
        assert!(matches!(tokens[2].kind, TokenKind::Caret));
        assert!(matches!(tokens[3].kind, TokenKind::Tilde));
        assert!(matches!(tokens[4].kind, TokenKind::LtLt));
        assert!(matches!(tokens[5].kind, TokenKind::GtGt));
        assert!(matches!(tokens[6].kind, TokenKind::GtGtGt));
    }

    #[test]
    fn test_all_assignment_operators() {
        let tokens = lex_source("= += -= *= /= //= %= **= &= |= ^= <<= >>= >>>=");
        assert!(matches!(tokens[0].kind, TokenKind::Eq));
        assert!(matches!(tokens[1].kind, TokenKind::PlusEq));
        assert!(matches!(tokens[2].kind, TokenKind::MinusEq));
        assert!(matches!(tokens[3].kind, TokenKind::StarEq));
        assert!(matches!(tokens[4].kind, TokenKind::SlashEq));
        assert!(matches!(tokens[5].kind, TokenKind::SlashSlashEq));
        assert!(matches!(tokens[6].kind, TokenKind::PercentEq));
        assert!(matches!(tokens[7].kind, TokenKind::StarStarEq));
        assert!(matches!(tokens[8].kind, TokenKind::AmpersandEq));
        assert!(matches!(tokens[9].kind, TokenKind::PipeEq));
        assert!(matches!(tokens[10].kind, TokenKind::CaretEq));
        assert!(matches!(tokens[11].kind, TokenKind::LtLtEq));
        assert!(matches!(tokens[12].kind, TokenKind::GtGtEq));
        assert!(matches!(tokens[13].kind, TokenKind::GtGtGtEq));
    }

    #[test]
    fn test_special_operators() {
        let tokens = lex_source("-> => .. ..= ... ? ?? ?: :: @ $ |> <|");
        assert!(matches!(tokens[0].kind, TokenKind::Arrow));
        assert!(matches!(tokens[1].kind, TokenKind::FatArrow));
        assert!(matches!(tokens[2].kind, TokenKind::DotDot));
        assert!(matches!(tokens[3].kind, TokenKind::DotDotEq));
        assert!(matches!(tokens[4].kind, TokenKind::DotDotDot));
        assert!(matches!(tokens[5].kind, TokenKind::Question));
        assert!(matches!(tokens[6].kind, TokenKind::QuestionQuestion));
        assert!(matches!(tokens[7].kind, TokenKind::QuestionColon));
        assert!(matches!(tokens[8].kind, TokenKind::ColonColon));
        assert!(matches!(tokens[9].kind, TokenKind::At));
        assert!(matches!(tokens[10].kind, TokenKind::Dollar));
        assert!(matches!(tokens[11].kind, TokenKind::PipeGt));
        assert!(matches!(tokens[12].kind, TokenKind::LtPipe));
    }

    #[test]
    fn test_walrus_operator() {
        let tokens = lex_source("x := 42");
        assert!(matches!(tokens[0].kind, TokenKind::Ident(ref s) if s == "x"));
        assert!(matches!(tokens[1].kind, TokenKind::ColonEq));
    }

    #[test]
    fn test_all_declaration_keywords() {
        let tokens = lex_source("def let var const struct enum trait impl type class interface");
        assert!(matches!(tokens[0].kind, TokenKind::Def));
        assert!(matches!(tokens[1].kind, TokenKind::Let));
        assert!(matches!(tokens[2].kind, TokenKind::Var));
        assert!(matches!(tokens[3].kind, TokenKind::Const));
        assert!(matches!(tokens[4].kind, TokenKind::Struct));
        assert!(matches!(tokens[5].kind, TokenKind::Enum));
        assert!(matches!(tokens[6].kind, TokenKind::Trait));
        assert!(matches!(tokens[7].kind, TokenKind::Impl));
        assert!(matches!(tokens[8].kind, TokenKind::Type));
        assert!(matches!(tokens[9].kind, TokenKind::Class));
        assert!(matches!(tokens[10].kind, TokenKind::Interface));
    }

    #[test]
    fn test_all_control_flow_keywords() {
        let tokens = lex_source("if elif else match case for while loop break continue return yield await");
        assert!(matches!(tokens[0].kind, TokenKind::If));
        assert!(matches!(tokens[1].kind, TokenKind::Elif));
        assert!(matches!(tokens[2].kind, TokenKind::Else));
        assert!(matches!(tokens[3].kind, TokenKind::Match));
        assert!(matches!(tokens[4].kind, TokenKind::Case));
        assert!(matches!(tokens[5].kind, TokenKind::For));
        assert!(matches!(tokens[6].kind, TokenKind::While));
        assert!(matches!(tokens[7].kind, TokenKind::Loop));
        assert!(matches!(tokens[8].kind, TokenKind::Break));
        assert!(matches!(tokens[9].kind, TokenKind::Continue));
        assert!(matches!(tokens[10].kind, TokenKind::Return));
        assert!(matches!(tokens[11].kind, TokenKind::Yield));
        assert!(matches!(tokens[12].kind, TokenKind::Await));
    }

    #[test]
    fn test_all_type_keywords() {
        let tokens = lex_source("int float bool str char i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64");
        assert!(matches!(tokens[0].kind, TokenKind::Int));
        assert!(matches!(tokens[1].kind, TokenKind::Float));
        assert!(matches!(tokens[2].kind, TokenKind::Bool));
        assert!(matches!(tokens[3].kind, TokenKind::Str));
        assert!(matches!(tokens[4].kind, TokenKind::Char));
        assert!(matches!(tokens[5].kind, TokenKind::I8));
        assert!(matches!(tokens[6].kind, TokenKind::I16));
        assert!(matches!(tokens[7].kind, TokenKind::I32));
        assert!(matches!(tokens[8].kind, TokenKind::I64));
        assert!(matches!(tokens[9].kind, TokenKind::I128));
        assert!(matches!(tokens[10].kind, TokenKind::ISize));
        assert!(matches!(tokens[11].kind, TokenKind::U8));
        assert!(matches!(tokens[12].kind, TokenKind::U16));
        assert!(matches!(tokens[13].kind, TokenKind::U32));
        assert!(matches!(tokens[14].kind, TokenKind::U64));
        assert!(matches!(tokens[15].kind, TokenKind::U128));
        assert!(matches!(tokens[16].kind, TokenKind::USize));
        assert!(matches!(tokens[17].kind, TokenKind::F32));
        assert!(matches!(tokens[18].kind, TokenKind::F64));
    }

    #[test]
    fn test_module_and_visibility_keywords() {
        let tokens = lex_source("import from as export pub priv protected");
        assert!(matches!(tokens[0].kind, TokenKind::Import));
        assert!(matches!(tokens[1].kind, TokenKind::From));
        assert!(matches!(tokens[2].kind, TokenKind::As));
        assert!(matches!(tokens[3].kind, TokenKind::Export));
        assert!(matches!(tokens[4].kind, TokenKind::Pub));
        assert!(matches!(tokens[5].kind, TokenKind::Priv));
        assert!(matches!(tokens[6].kind, TokenKind::Protected));
    }

    #[test]
    fn test_memory_and_ownership_keywords() {
        let tokens = lex_source("ref mut move copy clone box alloc defer drop static unsafe");
        assert!(matches!(tokens[0].kind, TokenKind::Ref));
        assert!(matches!(tokens[1].kind, TokenKind::Mut));
        assert!(matches!(tokens[2].kind, TokenKind::Move));
        assert!(matches!(tokens[3].kind, TokenKind::Copy));
        assert!(matches!(tokens[4].kind, TokenKind::Clone));
        assert!(matches!(tokens[5].kind, TokenKind::Box));
        assert!(matches!(tokens[6].kind, TokenKind::Alloc));
        assert!(matches!(tokens[7].kind, TokenKind::Defer));
        assert!(matches!(tokens[8].kind, TokenKind::Drop));
        assert!(matches!(tokens[9].kind, TokenKind::Static));
        assert!(matches!(tokens[10].kind, TokenKind::Unsafe));
    }

    #[test]
    fn test_concurrency_keywords() {
        let tokens = lex_source("async spawn send recv select");
        assert!(matches!(tokens[0].kind, TokenKind::Async));
        assert!(matches!(tokens[1].kind, TokenKind::Spawn));
        assert!(matches!(tokens[2].kind, TokenKind::Send));
        assert!(matches!(tokens[3].kind, TokenKind::Recv));
        assert!(matches!(tokens[4].kind, TokenKind::Select));
    }

    #[test]
    fn test_boolean_and_special_literal_keywords() {
        let tokens = lex_source("true false none null");
        assert!(matches!(tokens[0].kind, TokenKind::True));
        assert!(matches!(tokens[1].kind, TokenKind::False));
        assert!(matches!(tokens[2].kind, TokenKind::None));
        assert!(matches!(tokens[3].kind, TokenKind::Null));
    }

    #[test]
    fn test_operator_keywords() {
        let tokens = lex_source("and or not in is");
        assert!(matches!(tokens[0].kind, TokenKind::And));
        assert!(matches!(tokens[1].kind, TokenKind::Or));
        assert!(matches!(tokens[2].kind, TokenKind::Not));
        assert!(matches!(tokens[3].kind, TokenKind::In));
        assert!(matches!(tokens[4].kind, TokenKind::Is));
    }

    #[test]
    fn test_other_keywords() {
        let tokens = lex_source("self Self super where with try catch finally raise assert lambda comptime macro extern");
        assert!(matches!(tokens[0].kind, TokenKind::SelfLower));
        assert!(matches!(tokens[1].kind, TokenKind::SelfUpper));
        assert!(matches!(tokens[2].kind, TokenKind::Super));
        assert!(matches!(tokens[3].kind, TokenKind::Where));
        assert!(matches!(tokens[4].kind, TokenKind::With));
        assert!(matches!(tokens[5].kind, TokenKind::Try));
        assert!(matches!(tokens[6].kind, TokenKind::Catch));
        assert!(matches!(tokens[7].kind, TokenKind::Finally));
        assert!(matches!(tokens[8].kind, TokenKind::Raise));
        assert!(matches!(tokens[9].kind, TokenKind::Assert));
        assert!(matches!(tokens[10].kind, TokenKind::Lambda));
        assert!(matches!(tokens[11].kind, TokenKind::Comptime));
        assert!(matches!(tokens[12].kind, TokenKind::Macro));
        assert!(matches!(tokens[13].kind, TokenKind::Extern));
    }

    #[test]
    fn test_integer_with_suffixes() {
        let tokens = lex_source("42i8 1000i16 50000i32 9999999i64 123456789i128 100isize");
        assert!(matches!(tokens[0].kind, TokenKind::Literal(Literal::Integer(ref s, Some(IntSuffix::I8))) if s == "42"));
        assert!(matches!(tokens[1].kind, TokenKind::Literal(Literal::Integer(ref s, Some(IntSuffix::I16))) if s == "1000"));
        assert!(matches!(tokens[2].kind, TokenKind::Literal(Literal::Integer(ref s, Some(IntSuffix::I32))) if s == "50000"));
        assert!(matches!(tokens[3].kind, TokenKind::Literal(Literal::Integer(ref s, Some(IntSuffix::I64))) if s == "9999999"));
        assert!(matches!(tokens[4].kind, TokenKind::Literal(Literal::Integer(ref s, Some(IntSuffix::I128))) if s == "123456789"));
        assert!(matches!(tokens[5].kind, TokenKind::Literal(Literal::Integer(ref s, Some(IntSuffix::ISize))) if s == "100"));
    }

    #[test]
    fn test_unsigned_integer_with_suffixes() {
        let tokens = lex_source("42u8 1000u16 50000u32 9999999u64 123456789u128 100usize");
        assert!(matches!(tokens[0].kind, TokenKind::Literal(Literal::Integer(ref s, Some(IntSuffix::U8))) if s == "42"));
        assert!(matches!(tokens[1].kind, TokenKind::Literal(Literal::Integer(ref s, Some(IntSuffix::U16))) if s == "1000"));
        assert!(matches!(tokens[2].kind, TokenKind::Literal(Literal::Integer(ref s, Some(IntSuffix::U32))) if s == "50000"));
        assert!(matches!(tokens[3].kind, TokenKind::Literal(Literal::Integer(ref s, Some(IntSuffix::U64))) if s == "9999999"));
        assert!(matches!(tokens[4].kind, TokenKind::Literal(Literal::Integer(ref s, Some(IntSuffix::U128))) if s == "123456789"));
        assert!(matches!(tokens[5].kind, TokenKind::Literal(Literal::Integer(ref s, Some(IntSuffix::USize))) if s == "100"));
    }

    #[test]
    fn test_float_with_suffixes() {
        let tokens = lex_source("3.14f32 2.71828f64");
        assert!(matches!(tokens[0].kind, TokenKind::Literal(Literal::Float(ref s, Some(FloatSuffix::F32))) if s == "3.14"));
        assert!(matches!(tokens[1].kind, TokenKind::Literal(Literal::Float(ref s, Some(FloatSuffix::F64))) if s == "2.71828"));
    }

    #[test]
    fn test_integer_with_underscores() {
        let tokens = lex_source("1_000_000 0b1010_1010 0o755_644 0xDEAD_BEEF");
        assert!(matches!(tokens[0].kind, TokenKind::Literal(Literal::Integer(ref s, None)) if s == "1000000"));
        assert!(matches!(tokens[1].kind, TokenKind::Literal(Literal::Integer(ref s, None)) if s == "0b10101010"));
        assert!(matches!(tokens[2].kind, TokenKind::Literal(Literal::Integer(ref s, None)) if s == "0o755644"));
        assert!(matches!(tokens[3].kind, TokenKind::Literal(Literal::Integer(ref s, None)) if s == "0xDEADBEEF"));
    }

    #[test]
    fn test_float_with_exponent() {
        let tokens = lex_source("1e10 2.5e-3 3.14E+2");
        assert!(matches!(tokens[0].kind, TokenKind::Literal(Literal::Float(ref s, None)) if s == "1e10"));
        assert!(matches!(tokens[1].kind, TokenKind::Literal(Literal::Float(ref s, None)) if s == "2.5e-3"));
        assert!(matches!(tokens[2].kind, TokenKind::Literal(Literal::Float(ref s, None)) if s == "3.14e+2"));
    }

    #[test]
    fn test_string_with_escapes() {
        let tokens = lex_source(r#""hello\nworld\t\"quoted\"""#);
        assert!(matches!(tokens[0].kind, TokenKind::Literal(Literal::String(ref s)) if s == "hello\nworld\t\"quoted\""));
    }

    #[test]
    fn test_string_with_unicode_escape() {
        let tokens = lex_source(r#""\u{1F600}""#);
        assert!(matches!(tokens[0].kind, TokenKind::Literal(Literal::String(ref s)) if s == "😀"));
    }

    #[test]
    fn test_string_with_hex_escape() {
        let tokens = lex_source(r#""\x41\x42\x43""#);
        assert!(matches!(tokens[0].kind, TokenKind::Literal(Literal::String(ref s)) if s == "ABC"));
    }

    #[test]
    fn test_heredoc_string() {
        let tokens = lex_source("\"\"\"This is\na multiline\nstring\"\"\"");
        assert!(matches!(tokens[0].kind, TokenKind::Literal(Literal::Heredoc(ref s)) if s.contains("multiline")));
    }

    #[test]
    fn test_raw_string_basic() {
        let tokens = lex_source(r#"r"C:\path\to\file""#);
        assert!(matches!(tokens[0].kind, TokenKind::Literal(Literal::RawString(0, ref s)) if s == r"C:\path\to\file"));
    }

    #[test]
    fn test_raw_string_with_hashes() {
        let tokens = lex_source(r###"r#"string with "quotes" inside"#"###);
        assert!(matches!(tokens[0].kind, TokenKind::Literal(Literal::RawString(1, ref s)) if s == r#"string with "quotes" inside"#));
    }

    #[test]
    fn test_format_string_basic() {
        let tokens = lex_source(r#"f"Hello {name}!""#);
        assert!(matches!(tokens[0].kind, TokenKind::Literal(Literal::FormatString(ref s)) if s.contains("{name}")));
    }

    #[test]
    fn test_format_string_with_expression() {
        let tokens = lex_source(r#"f"Result: {x + y}""#);
        assert!(matches!(tokens[0].kind, TokenKind::Literal(Literal::FormatString(ref s)) if s.contains("{x + y}")));
    }

    #[test]
    fn test_format_string_nested_braces() {
        let tokens = lex_source(r#"f"Data: {data.get("key")}""#);
        assert!(matches!(tokens[0].kind, TokenKind::Literal(Literal::FormatString(_))));
    }

    #[test]
    fn test_byte_string() {
        let tokens = lex_source(r#"b"hello""#);
        assert!(matches!(tokens[0].kind, TokenKind::Literal(Literal::ByteString(ref bytes)) if bytes == b"hello"));
    }

    #[test]
    fn test_char_literal() {
        let tokens = lex_source("'a' 'Z' '0'");
        assert!(matches!(tokens[0].kind, TokenKind::Literal(Literal::Char('a'))));
        assert!(matches!(tokens[1].kind, TokenKind::Literal(Literal::Char('Z'))));
        assert!(matches!(tokens[2].kind, TokenKind::Literal(Literal::Char('0'))));
    }

    #[test]
    fn test_char_literal_with_escape() {
        let tokens = lex_source("'\\n' '\\t' '\\'' '\\\"' '\\\\'");
        assert!(matches!(tokens[0].kind, TokenKind::Literal(Literal::Char('\n'))));
        assert!(matches!(tokens[1].kind, TokenKind::Literal(Literal::Char('\t'))));
        assert!(matches!(tokens[2].kind, TokenKind::Literal(Literal::Char('\''))));
        assert!(matches!(tokens[3].kind, TokenKind::Literal(Literal::Char('"'))));
        assert!(matches!(tokens[4].kind, TokenKind::Literal(Literal::Char('\\'))));
    }

    #[test]
    fn test_char_literal_unicode() {
        let tokens = lex_source(r"'\u{03B1}'");
        assert!(matches!(tokens[0].kind, TokenKind::Literal(Literal::Char('α'))));
    }

    #[test]
    fn test_delimiters() {
        let tokens = lex_source("( ) [ ] { } , ; :");
        assert!(matches!(tokens[0].kind, TokenKind::LParen));
        assert!(matches!(tokens[1].kind, TokenKind::RParen));
        assert!(matches!(tokens[2].kind, TokenKind::LBracket));
        assert!(matches!(tokens[3].kind, TokenKind::RBracket));
        assert!(matches!(tokens[4].kind, TokenKind::LBrace));
        assert!(matches!(tokens[5].kind, TokenKind::RBrace));
        assert!(matches!(tokens[6].kind, TokenKind::Comma));
        assert!(matches!(tokens[7].kind, TokenKind::Semicolon));
        assert!(matches!(tokens[8].kind, TokenKind::Colon));
    }

    #[test]
    fn test_nesting_depth_tracking() {
        let source = "def foo():\n    if (x > 0 and\n        y > 0):\n        print(x)";
        let tokens = lex_source(source);
        // Should have parentheses tokens
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::LParen)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::RParen)));
    }

    #[test]
    fn test_indentation_increase() {
        let source = "def foo():\n    return 42";
        let tokens = lex_source(source);
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Indent)));
    }

    #[test]
    fn test_indentation_decrease() {
        let source = "def foo():\n    x = 1\n    y = 2\nz = 3";
        let tokens = lex_source(source);
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Indent)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Dedent)));
    }

    #[test]
    fn test_multiple_dedents() {
        let source = "if x:\n    if y:\n        if z:\n            foo()\nbar()";
        let tokens = lex_source(source);
        let dedent_count = tokens.iter().filter(|t| matches!(t.kind, TokenKind::Dedent)).count();
        assert!(dedent_count >= 3);
    }

    #[test]
    fn test_blank_lines_ignored() {
        let source = "let x = 1\n\n\nlet y = 2";
        let tokens = lex_source(source);
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Ident(ref s) if s == "x")));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Ident(ref s) if s == "y")));
    }

    #[test]
    fn test_unicode_identifier() {
        let tokens = lex_source("let 変数 = 42");
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Ident(ref s) if s == "変数")));
    }

    #[test]
    fn test_unicode_identifier_greek() {
        let tokens = lex_source("let α = 3.14");
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Ident(ref s) if s == "α")));
    }

    #[test]
    fn test_unicode_identifier_emoji_not_allowed() {
        // Emojis are not XID_Start, so this should be an error
        let tokens = lex_source("let 😀 = 42");
        // The emoji should cause an error and be skipped
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Literal(Literal::Integer(ref s, None)) if s == "42")));
    }

    #[test]
    fn test_raw_identifier() {
        let tokens = lex_source("@let @class @def");
        // Raw identifiers turn keywords into identifiers
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Ident(ref s) if s == "let")));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Ident(ref s) if s == "class")));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Ident(ref s) if s == "def")));
    }

    #[test]
    fn test_hash_symbol() {
        let tokens = lex_source("#[derive(Debug)]");
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Hash)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::LBracket)));
    }

    #[test]
    fn test_dot_access() {
        let tokens = lex_source("obj.field.method()");
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Dot)));
    }

    #[test]
    fn test_range_operators() {
        let tokens = lex_source("0..10 0..=10");
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::DotDot)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::DotDotEq)));
    }

    #[test]
    fn test_variadic_operator() {
        let tokens = lex_source("def foo(args...):");
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::DotDotDot)));
    }

    #[test]
    fn test_pipe_operator() {
        let tokens = lex_source("value |> func |> other");
        assert!(tokens.iter().filter(|t| matches!(t.kind, TokenKind::PipeGt)).count() == 2);
    }

    #[test]
    fn test_reverse_pipe_operator() {
        let tokens = lex_source("func <| value");
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::LtPipe)));
    }

    #[test]
    fn test_optional_chaining() {
        let tokens = lex_source("obj?.field?.method()");
        assert!(tokens.iter().filter(|t| matches!(t.kind, TokenKind::Question)).count() == 2);
    }

    #[test]
    fn test_null_coalescing() {
        let tokens = lex_source("value ?? default");
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::QuestionQuestion)));
    }

    #[test]
    fn test_ternary_operator() {
        let tokens = lex_source("condition ?: fallback");
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::QuestionColon)));
    }

    #[test]
    fn test_path_separator() {
        let tokens = lex_source("std::io::File");
        assert!(tokens.iter().filter(|t| matches!(t.kind, TokenKind::ColonColon)).count() == 2);
    }

    #[test]
    fn test_newline_handling() {
        let tokens = lex_source("let x = 1\nlet y = 2");
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Newline)));
    }

    #[test]
    fn test_crlf_newline() {
        let tokens = lex_source("let x = 1\r\nlet y = 2");
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Newline)));
    }

    #[test]
    fn test_eof_token() {
        let tokens = lex_source("let x = 42");
        assert!(matches!(tokens.last().unwrap().kind, TokenKind::Eof));
    }

    #[test]
    fn test_eof_with_dedents() {
        let tokens = lex_source("def foo():\n    if x:\n        y = 1");
        // Should emit DEDENT tokens before EOF
        assert!(matches!(tokens.last().unwrap().kind, TokenKind::Eof));
        let last_few = &tokens[tokens.len().saturating_sub(5)..];
        assert!(last_few.iter().any(|t| matches!(t.kind, TokenKind::Dedent)));
    }

    #[test]
    fn test_complex_expression() {
        let source = "result = (a + b) * c / (d - e) ** 2";
        let tokens = lex_source(source);
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Ident(ref s) if s == "result")));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Eq)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Plus)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Star)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Slash)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Minus)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::StarStar)));
    }

    #[test]
    fn test_function_definition() {
        let source = "def add(a: int, b: int) -> int:\n    return a + b";
        let tokens = lex_source(source);
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Def)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Ident(ref s) if s == "add")));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Arrow)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Return)));
    }

    #[test]
    fn test_match_expression() {
        let source = "match value:\n    case 1 => foo()\n    case 2 => bar()";
        let tokens = lex_source(source);
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Match)));
        assert!(tokens.iter().filter(|t| matches!(t.kind, TokenKind::Case)).count() == 2);
        assert!(tokens.iter().filter(|t| matches!(t.kind, TokenKind::FatArrow)).count() == 2);
    }

    #[test]
    fn test_async_function() {
        let source = "async def fetch():\n    await request()";
        let tokens = lex_source(source);
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Async)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Await)));
    }

    #[test]
    fn test_spawn_expression() {
        let source = "task = spawn worker()";
        let tokens = lex_source(source);
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Spawn)));
    }

    #[test]
    fn test_channel_operations() {
        let source = "chan.send(value)\nchan.recv()";
        let tokens = lex_source(source);
        // send and recv are method names, not keywords in this context
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Ident(ref s) if s == "chan")));
        assert!(tokens.iter().filter(|t| matches!(t.kind, TokenKind::Dot)).count() >= 2);
    }

    #[test]
    fn test_error_recovery_invalid_char() {
        let tokens = lex_source("let x = 1 \u{0007} let y = 2");
        // Should recover and continue parsing
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Ident(ref s) if s == "x")));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Ident(ref s) if s == "y")));
    }

    #[test]
    fn test_comprehensive_program() {
        let source = r#"
import std::io

def main():
    let x: int = 42
    let y = x * 2

    if y > 50:
        print(f"y is {y}")
    else:
        print("y is small")

    for i in 0..10:
        if i % 2 == 0:
            continue
        print(i)
"#;
        let tokens = lex_source(source);

        // Verify we got all major token types
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Import)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Def)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Let)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::If)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Else)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::For)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::In)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Continue)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Literal(Literal::FormatString(_)))));
        assert!(matches!(tokens.last().unwrap().kind, TokenKind::Eof));
    }
}
