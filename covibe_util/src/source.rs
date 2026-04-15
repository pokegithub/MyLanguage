//! Source file representation and management.
//!
//! This module provides types for representing source files and managing
//! collections of source files. It tracks file contents, line breaks, and
//! provides efficient querying of source locations.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use rustc_hash::FxHashMap;

use crate::span::{BytePos, LineCol, Span};

/// Unique identifier for a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(u32);

impl FileId {
    /// Creates a new FileId from a raw u32.
    pub const fn from_raw(raw: u32) -> Self {
        FileId(raw)
    }

    /// Returns the raw u32 value.
    pub const fn as_raw(self) -> u32 {
        self.0
    }

    /// The sentinel value for an invalid or unknown file.
    pub const INVALID: FileId = FileId(u32::MAX);
}

/// Represents a single source file with its contents and metadata.
#[derive(Debug, Clone)]
pub struct SourceFile {
    /// Unique identifier for this file.
    id: FileId,

    /// Path to this file (may be synthetic for REPL input, etc.)
    path: PathBuf,

    /// The complete source text of this file.
    source: Arc<str>,

    /// Byte positions of line breaks in the source.
    /// line_starts[i] is the byte position of the start of line i.
    /// The first line always starts at position 0.
    line_starts: Vec<BytePos>,
}

impl SourceFile {
    /// Creates a new SourceFile from a path and source text.
    pub fn new(id: FileId, path: PathBuf, source: String) -> Self {
        let source = Arc::from(source);
        let line_starts = Self::compute_line_starts(&source);

        SourceFile {
            id,
            path,
            source,
            line_starts,
        }
    }

    /// Returns the file ID.
    pub fn id(&self) -> FileId {
        self.id
    }

    /// Returns the file path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the complete source text.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Returns the number of lines in this file.
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Returns the byte position of the start of the given line (0-indexed).
    pub fn line_start(&self, line: usize) -> Option<BytePos> {
        self.line_starts.get(line).copied()
    }

    /// Converts a byte position to a line and column (both 0-indexed).
    pub fn lookup_line_col(&self, pos: BytePos) -> LineCol {
        // Binary search to find the line containing this position
        match self.line_starts.binary_search(&pos) {
            Ok(line) => {
                // Position is exactly at the start of a line
                LineCol { line, column: 0 }
            }
            Err(next_line) => {
                // Position is somewhere within a line
                if next_line == 0 {
                    // Before the first line (shouldn't happen with valid positions)
                    LineCol { line: 0, column: 0 }
                } else {
                    let line = next_line - 1;
                    let line_start = self.line_starts[line];
                    let column = (pos.0 - line_start.0) as usize;
                    LineCol { line, column }
                }
            }
        }
    }

    /// Returns the source text for a given span within this file.
    pub fn source_text(&self, span: Span) -> &str {
        let start = span.start().0 as usize;
        let end = span.end().0 as usize;
        &self.source[start.min(self.source.len())..end.min(self.source.len())]
    }

    /// Returns the source text for a given line (0-indexed).
    pub fn line_text(&self, line: usize) -> Option<&str> {
        let start = self.line_starts.get(line)?.0 as usize;
        let end = if line + 1 < self.line_starts.len() {
            self.line_starts[line + 1].0 as usize
        } else {
            self.source.len()
        };

        // Remove trailing newline if present
        let text = &self.source[start..end];
        Some(text.strip_suffix('\n')
            .or_else(|| text.strip_suffix("\r\n"))
            .unwrap_or(text))
    }

    /// Computes the byte positions where each line starts.
    fn compute_line_starts(source: &str) -> Vec<BytePos> {
        let mut line_starts = vec![BytePos(0)];

        let bytes = source.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'\n' => {
                    // LF
                    line_starts.push(BytePos((i + 1) as u32));
                }
                b'\r' if i + 1 < bytes.len() && bytes[i + 1] == b'\n' => {
                    // CRLF - skip the CR and count the LF
                    i += 1;
                    line_starts.push(BytePos((i + 1) as u32));
                }
                b'\r' => {
                    // CR alone (old Mac style)
                    line_starts.push(BytePos((i + 1) as u32));
                }
                _ => {}
            }
            i += 1;
        }

        line_starts
    }
}

/// Maps FileIds to SourceFiles and manages source file lifecycle.
#[derive(Debug, Clone, Default)]
pub struct SourceMap {
    inner: Arc<RwLock<SourceMapInner>>,
}

#[derive(Debug)]
struct SourceMapInner {
    files: FxHashMap<FileId, Arc<SourceFile>>,
    next_id: u32,
}

impl Default for SourceMapInner {
    fn default() -> Self {
        SourceMapInner {
            files: FxHashMap::default(),
            next_id: 0,
        }
    }
}

impl SourceMap {
    /// Creates a new empty SourceMap.
    pub fn new() -> Self {
        SourceMap {
            inner: Arc::new(RwLock::new(SourceMapInner::default())),
        }
    }

    /// Adds a source file and returns its FileId.
    pub fn add_file(&self, path: PathBuf, source: String) -> FileId {
        let mut inner = self.inner.write();
        let id = FileId(inner.next_id);
        inner.next_id += 1;

        let file = Arc::new(SourceFile::new(id, path, source));
        inner.files.insert(id, file);

        id
    }

    /// Retrieves a source file by its ID.
    pub fn get_file(&self, id: FileId) -> Option<Arc<SourceFile>> {
        let inner = self.inner.read();
        inner.files.get(&id).cloned()
    }

    /// Returns the number of files in this SourceMap.
    pub fn file_count(&self) -> usize {
        let inner = self.inner.read();
        inner.files.len()
    }

    /// Iterates over all file IDs.
    pub fn file_ids(&self) -> Vec<FileId> {
        let inner = self.inner.read();
        inner.files.keys().copied().collect()
    }

    /// Looks up a source file by path.
    pub fn get_file_by_path(&self, path: &Path) -> Option<Arc<SourceFile>> {
        let inner = self.inner.read();
        inner.files.values()
            .find(|f| f.path() == path)
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_file_line_starts() {
        let source = "line 1\nline 2\nline 3";
        let file = SourceFile::new(FileId(0), PathBuf::from("test.cvb"), source.to_string());

        assert_eq!(file.line_count(), 3);
        assert_eq!(file.line_start(0), Some(BytePos(0)));
        assert_eq!(file.line_start(1), Some(BytePos(7)));
        assert_eq!(file.line_start(2), Some(BytePos(14)));
    }

    #[test]
    fn test_source_file_crlf() {
        let source = "line 1\r\nline 2\r\nline 3";
        let file = SourceFile::new(FileId(0), PathBuf::from("test.cvb"), source.to_string());

        assert_eq!(file.line_count(), 3);
        assert_eq!(file.line_start(0), Some(BytePos(0)));
        assert_eq!(file.line_start(1), Some(BytePos(8)));
        assert_eq!(file.line_start(2), Some(BytePos(16)));
    }

    #[test]
    fn test_lookup_line_col() {
        let source = "hello\nworld\nfoo";
        let file = SourceFile::new(FileId(0), PathBuf::from("test.cvb"), source.to_string());

        // "hello\n" = positions 0-5, newline at 5
        // "world\n" = positions 6-11, newline at 11
        // "foo" = positions 12-14

        assert_eq!(file.lookup_line_col(BytePos(0)), LineCol { line: 0, column: 0 });
        assert_eq!(file.lookup_line_col(BytePos(3)), LineCol { line: 0, column: 3 });
        assert_eq!(file.lookup_line_col(BytePos(6)), LineCol { line: 1, column: 0 });
        assert_eq!(file.lookup_line_col(BytePos(9)), LineCol { line: 1, column: 3 });
        assert_eq!(file.lookup_line_col(BytePos(12)), LineCol { line: 2, column: 0 });
    }

    #[test]
    fn test_line_text() {
        let source = "first\nsecond\nthird";
        let file = SourceFile::new(FileId(0), PathBuf::from("test.cvb"), source.to_string());

        assert_eq!(file.line_text(0), Some("first"));
        assert_eq!(file.line_text(1), Some("second"));
        assert_eq!(file.line_text(2), Some("third"));
        assert_eq!(file.line_text(3), None);
    }

    #[test]
    fn test_source_map() {
        let map = SourceMap::new();

        let id1 = map.add_file(PathBuf::from("file1.cvb"), "content 1".to_string());
        let id2 = map.add_file(PathBuf::from("file2.cvb"), "content 2".to_string());

        assert_eq!(map.file_count(), 2);

        let file1 = map.get_file(id1).unwrap();
        assert_eq!(file1.source(), "content 1");
        assert_eq!(file1.path(), Path::new("file1.cvb"));

        let file2 = map.get_file(id2).unwrap();
        assert_eq!(file2.source(), "content 2");

        let found = map.get_file_by_path(Path::new("file1.cvb")).unwrap();
        assert_eq!(found.id(), id1);
    }

    #[test]
    fn test_source_text() {
        let source = "hello world";
        let file = SourceFile::new(FileId(0), PathBuf::from("test.cvb"), source.to_string());

        let span = Span::new(BytePos(0), BytePos(5));
        assert_eq!(file.source_text(span), "hello");

        let span = Span::new(BytePos(6), BytePos(11));
        assert_eq!(file.source_text(span), "world");
    }

    #[test]
    fn test_empty_file() {
        let file = SourceFile::new(FileId(0), PathBuf::from("empty.cvb"), String::new());
        assert_eq!(file.line_count(), 1);
        assert_eq!(file.line_text(0), Some(""));
    }

    #[test]
    fn test_file_ending_with_newline() {
        let source = "line 1\nline 2\n";
        let file = SourceFile::new(FileId(0), PathBuf::from("test.cvb"), source.to_string());

        assert_eq!(file.line_count(), 3); // Empty line after final newline
        assert_eq!(file.line_text(0), Some("line 1"));
        assert_eq!(file.line_text(1), Some("line 2"));
        assert_eq!(file.line_text(2), Some(""));
    }
}
