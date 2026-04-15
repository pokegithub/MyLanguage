//! Source code span tracking.
//!
//! This module provides types for representing positions and ranges within
//! source files. Spans are used throughout the compiler to track the origin
//! of every AST node, type, and diagnostic.

use std::fmt;

use crate::source::FileId;

/// A byte position within a source file.
///
/// Byte positions are 0-indexed and measure the offset in bytes from the
/// start of the file. They are NOT character indices due to UTF-8 encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BytePos(pub u32);

impl BytePos {
    /// The position at the start of a file.
    pub const ZERO: BytePos = BytePos(0);

    /// Creates a new BytePos.
    pub const fn new(pos: u32) -> Self {
        BytePos(pos)
    }

    /// Returns the position as a usize for indexing.
    pub fn to_usize(self) -> usize {
        self.0 as usize
    }

    /// Advances this position by a given offset.
    pub fn advance(self, offset: u32) -> BytePos {
        BytePos(self.0 + offset)
    }

    /// Advances this position by the byte length of a string.
    pub fn advance_by_str(self, s: &str) -> BytePos {
        BytePos(self.0 + s.len() as u32)
    }
}

impl fmt::Display for BytePos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<usize> for BytePos {
    fn from(pos: usize) -> Self {
        BytePos(pos as u32)
    }
}

impl From<u32> for BytePos {
    fn from(pos: u32) -> Self {
        BytePos(pos)
    }
}

/// A line and column position within a source file.
///
/// Both line and column are 0-indexed internally, but displayed as 1-indexed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LineCol {
    pub line: usize,
    pub column: usize,
}

impl LineCol {
    /// Creates a new LineCol (0-indexed).
    pub const fn new(line: usize, column: usize) -> Self {
        LineCol { line, column }
    }

    /// Returns the 1-indexed line number for display.
    pub fn display_line(&self) -> usize {
        self.line + 1
    }

    /// Returns the 1-indexed column number for display.
    pub fn display_column(&self) -> usize {
        self.column + 1
    }
}

impl fmt::Display for LineCol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.display_line(), self.display_column())
    }
}

/// A contiguous range within a single source file.
///
/// Spans are half-open intervals [start, end), meaning the start position
/// is included but the end position is excluded.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    file: FileId,
    start: BytePos,
    end: BytePos,
}

impl Span {
    /// Creates a new span.
    pub const fn new(start: BytePos, end: BytePos) -> Self {
        Span {
            file: FileId::INVALID,
            start,
            end,
        }
    }

    /// Creates a new span with a specific file ID.
    pub const fn with_file(file: FileId, start: BytePos, end: BytePos) -> Self {
        Span { file, start, end }
    }

    /// Creates a span from byte offsets.
    pub fn from_offsets(start: u32, end: u32) -> Self {
        Span::new(BytePos(start), BytePos(end))
    }

    /// Creates a zero-length span at a specific position.
    pub const fn at(pos: BytePos) -> Self {
        Span::new(pos, pos)
    }

    /// Returns the file ID this span belongs to.
    pub const fn file(&self) -> FileId {
        self.file
    }

    /// Returns the start position of this span.
    pub const fn start(&self) -> BytePos {
        self.start
    }

    /// Returns the end position of this span.
    pub const fn end(&self) -> BytePos {
        self.end
    }

    /// Returns the length of this span in bytes.
    pub fn len(&self) -> u32 {
        self.end.0 - self.start.0
    }

    /// Returns true if this span has zero length.
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Checks if this span contains a given position.
    pub fn contains(&self, pos: BytePos) -> bool {
        self.start <= pos && pos < self.end
    }

    /// Checks if this span overlaps with another span.
    pub fn overlaps(&self, other: Span) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// Returns the smallest span that contains both this span and another.
    pub fn merge(&self, other: Span) -> Span {
        assert_eq!(self.file, other.file, "Cannot merge spans from different files");
        Span {
            file: self.file,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Extends this span to include another span.
    pub fn extend(&mut self, other: Span) {
        *self = self.merge(other);
    }

    /// Returns a new span that extends to a given position.
    pub fn to(&self, end: BytePos) -> Span {
        Span {
            file: self.file,
            start: self.start,
            end,
        }
    }

    /// Returns a new span that starts from a given position.
    pub fn from(&self, start: BytePos) -> Span {
        Span {
            file: self.file,
            start,
            end: self.end,
        }
    }

    /// Shrinks the span from the start by a given amount.
    pub fn shrink_start(&self, amount: u32) -> Span {
        Span {
            file: self.file,
            start: BytePos(self.start.0 + amount),
            end: self.end,
        }
    }

    /// Shrinks the span from the end by a given amount.
    pub fn shrink_end(&self, amount: u32) -> Span {
        Span {
            file: self.file,
            start: self.start,
            end: BytePos(self.end.0.saturating_sub(amount)),
        }
    }

    /// Sets the file ID for this span.
    pub fn with_file_id(mut self, file: FileId) -> Self {
        self.file = file;
        self
    }

    /// A sentinel value for an invalid or unknown span.
    pub const INVALID: Span = Span {
        file: FileId::INVALID,
        start: BytePos(0),
        end: BytePos(0),
    };
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Span(file={:?}, {}..{})", self.file, self.start.0, self.end.0)
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

impl Default for Span {
    fn default() -> Self {
        Span::INVALID
    }
}

/// Trait for types that have an associated source span.
pub trait HasSpan {
    /// Returns the span of this value.
    fn span(&self) -> Span;
}

impl HasSpan for Span {
    fn span(&self) -> Span {
        *self
    }
}

/// Combines a value with its source span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    /// Creates a new spanned value.
    pub const fn new(node: T, span: Span) -> Self {
        Spanned { node, span }
    }

    /// Maps the inner value while preserving the span.
    pub fn map<U, F>(self, f: F) -> Spanned<U>
    where
        F: FnOnce(T) -> U,
    {
        Spanned {
            node: f(self.node),
            span: self.span,
        }
    }

    /// Returns a reference to the inner value.
    pub fn as_ref(&self) -> &T {
        &self.node
    }

    /// Returns a mutable reference to the inner value.
    pub fn as_mut(&mut self) -> &mut T {
        &mut self.node
    }
}

impl<T> HasSpan for Spanned<T> {
    fn span(&self) -> Span {
        self.span
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_pos() {
        let pos = BytePos(10);
        assert_eq!(pos.advance(5), BytePos(15));
        assert_eq!(pos.advance_by_str("hello"), BytePos(15));
        assert_eq!(pos.to_usize(), 10);
    }

    #[test]
    fn test_line_col_display() {
        let lc = LineCol::new(0, 0);
        assert_eq!(lc.display_line(), 1);
        assert_eq!(lc.display_column(), 1);
        assert_eq!(format!("{}", lc), "1:1");

        let lc = LineCol::new(5, 10);
        assert_eq!(format!("{}", lc), "6:11");
    }

    #[test]
    fn test_span_creation() {
        let span = Span::new(BytePos(10), BytePos(20));
        assert_eq!(span.start(), BytePos(10));
        assert_eq!(span.end(), BytePos(20));
        assert_eq!(span.len(), 10);
        assert!(!span.is_empty());

        let empty = Span::at(BytePos(15));
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn test_span_contains() {
        let span = Span::new(BytePos(10), BytePos(20));
        assert!(span.contains(BytePos(10)));
        assert!(span.contains(BytePos(15)));
        assert!(!span.contains(BytePos(20))); // Half-open interval
        assert!(!span.contains(BytePos(5)));
        assert!(!span.contains(BytePos(25)));
    }

    #[test]
    fn test_span_overlaps() {
        let span1 = Span::new(BytePos(10), BytePos(20));
        let span2 = Span::new(BytePos(15), BytePos(25));
        let span3 = Span::new(BytePos(20), BytePos(30));
        let span4 = Span::new(BytePos(0), BytePos(5));

        assert!(span1.overlaps(span2));
        assert!(span2.overlaps(span1));
        assert!(!span1.overlaps(span3)); // Adjacent but not overlapping
        assert!(!span1.overlaps(span4));
    }

    #[test]
    fn test_span_merge() {
        let span1 = Span::new(BytePos(10), BytePos(20));
        let span2 = Span::new(BytePos(15), BytePos(30));
        let merged = span1.merge(span2);

        assert_eq!(merged.start(), BytePos(10));
        assert_eq!(merged.end(), BytePos(30));
    }

    #[test]
    fn test_span_shrink() {
        let span = Span::new(BytePos(10), BytePos(20));

        let shrunk_start = span.shrink_start(2);
        assert_eq!(shrunk_start, Span::new(BytePos(12), BytePos(20)));

        let shrunk_end = span.shrink_end(3);
        assert_eq!(shrunk_end, Span::new(BytePos(10), BytePos(17)));
    }

    #[test]
    fn test_spanned_value() {
        let span = Span::new(BytePos(0), BytePos(5));
        let spanned = Spanned::new(42, span);

        assert_eq!(spanned.node, 42);
        assert_eq!(spanned.span, span);

        let mapped = spanned.map(|x| x * 2);
        assert_eq!(mapped.node, 84);
        assert_eq!(mapped.span, span);
    }

    #[test]
    fn test_span_to_from() {
        let span = Span::new(BytePos(10), BytePos(20));

        let extended = span.to(BytePos(30));
        assert_eq!(extended, Span::new(BytePos(10), BytePos(30)));

        let moved = span.from(BytePos(5));
        assert_eq!(moved, Span::new(BytePos(5), BytePos(20)));
    }
}
