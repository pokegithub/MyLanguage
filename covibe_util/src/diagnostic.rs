//! Diagnostic engine for beautiful error reporting.
//!
//! This module provides a diagnostic system for reporting errors, warnings,
//! and other messages to the user. It uses the ariadne crate to produce
//! high-quality, colored error messages with source context.

use std::fmt;
use std::io::{self, Write};
use std::sync::Arc;
use parking_lot::Mutex;

use ariadne::{Color, ColorGenerator, Label as AriadneLabel, Report, ReportKind, Source};

use crate::source::{FileId, SourceMap};
use crate::span::Span;

/// Severity level of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// An error that prevents compilation from succeeding.
    Error,
    /// A warning about potentially problematic code.
    Warning,
    /// An informational note or hint.
    Note,
    /// A help message suggesting how to fix an issue.
    Help,
}

impl Severity {
    /// Returns the ariadne ReportKind for this severity.
    fn to_report_kind(&self) -> ReportKind<'static> {
        match self {
            Severity::Error => ReportKind::Error,
            Severity::Warning => ReportKind::Warning,
            Severity::Note => ReportKind::Advice,
            Severity::Help => ReportKind::Advice,
        }
    }

    /// Returns the display name for this severity.
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
            Severity::Help => "help",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A label attached to a specific source location in a diagnostic.
#[derive(Debug, Clone)]
pub struct Label {
    /// The span this label points to.
    pub span: Span,
    /// The message for this label.
    pub message: Option<String>,
    /// The color to use for this label (optional).
    pub color: Option<Color>,
}

impl Label {
    /// Creates a new label at a given span.
    pub fn new(span: Span) -> Self {
        Label {
            span,
            message: None,
            color: None,
        }
    }

    /// Creates a new label with a message.
    pub fn with_message<S: Into<String>>(span: Span, message: S) -> Self {
        Label {
            span,
            message: Some(message.into()),
            color: None,
        }
    }

    /// Sets the message for this label.
    pub fn message<S: Into<String>>(mut self, message: S) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Sets the color for this label.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

}

/// A diagnostic message (error, warning, note, etc.).
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Severity level of this diagnostic.
    pub severity: Severity,
    /// The primary message.
    pub message: String,
    /// The file where the primary error occurred.
    pub file: FileId,
    /// The primary span where the error occurred.
    pub primary_span: Span,
    /// Additional labels pointing to related locations.
    pub labels: Vec<Label>,
    /// Optional help message.
    pub help: Option<String>,
    /// Optional note message.
    pub note: Option<String>,
}

impl Diagnostic {
    /// Creates a new error diagnostic.
    pub fn error<S: Into<String>>(message: S, file: FileId, span: Span) -> Self {
        Diagnostic {
            severity: Severity::Error,
            message: message.into(),
            file,
            primary_span: span,
            labels: vec![],
            help: None,
            note: None,
        }
    }

    /// Creates a new warning diagnostic.
    pub fn warning<S: Into<String>>(message: S, file: FileId, span: Span) -> Self {
        Diagnostic {
            severity: Severity::Warning,
            message: message.into(),
            file,
            primary_span: span,
            labels: vec![],
            help: None,
            note: None,
        }
    }

    /// Creates a new note diagnostic.
    pub fn note<S: Into<String>>(message: S, file: FileId, span: Span) -> Self {
        Diagnostic {
            severity: Severity::Note,
            message: message.into(),
            file,
            primary_span: span,
            labels: vec![],
            help: None,
            note: None,
        }
    }

    /// Adds a label to this diagnostic.
    pub fn with_label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    /// Adds multiple labels to this diagnostic.
    pub fn with_labels(mut self, labels: Vec<Label>) -> Self {
        self.labels.extend(labels);
        self
    }

    /// Adds a help message.
    pub fn with_help<S: Into<String>>(mut self, help: S) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Adds a note message.
    pub fn with_note<S: Into<String>>(mut self, note: S) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Emits this diagnostic using ariadne to a writer.
    pub fn emit<W: Write>(&self, source_map: &SourceMap, writer: &mut W) -> io::Result<()> {
        let file = source_map.get_file(self.file);

        if file.is_none() {
            // If we can't get the source file, print a simple text message
            writeln!(
                writer,
                "{}: {} at {:?}",
                self.severity, self.message, self.primary_span
            )?;
            return Ok(());
        }

        let file = file.unwrap();
        let file_path = file.path().display().to_string();

        // Build the ariadne report with the file path as ID
        let start_offset = self.primary_span.start().to_usize();
        let end_offset = self.primary_span.end().to_usize();

        let mut report = Report::build(
            self.severity.to_report_kind(),
            file_path.clone(),
            start_offset,
        )
        .with_message(&self.message);

        // Add the primary label
        let mut color_gen = ColorGenerator::new();
        let primary_color = color_gen.next();

        let primary_range = start_offset..end_offset;
        let primary_label = AriadneLabel::new((file_path.clone(), primary_range))
            .with_message(&self.message)
            .with_color(primary_color);

        report = report.with_label(primary_label);

        // Add additional labels
        for label in &self.labels {
            let color = label.color.unwrap_or_else(|| color_gen.next());
            let range = label.span.start().to_usize()..label.span.end().to_usize();

            let mut ariadne_label = AriadneLabel::new((file_path.clone(), range))
                .with_color(color);

            if let Some(ref msg) = label.message {
                ariadne_label = ariadne_label.with_message(msg);
            }

            report = report.with_label(ariadne_label);
        }

        // Add help message if present
        if let Some(ref help) = self.help {
            report = report.with_help(help);
        }

        // Add note message if present
        if let Some(ref note) = self.note {
            report = report.with_note(note);
        }

        // Finish and write the report
        let cache = SingleFileCache {
            path: file_path,
            source: Source::from(file.source().to_string()),
        };

        report.finish().write(cache, writer)?;

        Ok(())
    }
}

/// Simple cache for a single source file, used with ariadne.
struct SingleFileCache {
    path: String,
    source: Source,
}

impl ariadne::Cache<String> for SingleFileCache {
    type Storage = String;

    fn fetch(&mut self, id: &String) -> Result<&Source, Box<dyn fmt::Debug + '_>> {
        if *id == self.path {
            Ok(&self.source)
        } else {
            Err(Box::new(format!("Unknown file: {}", id)))
        }
    }

    fn display<'a>(&self, id: &'a String) -> Option<Box<dyn fmt::Display + 'a>> {
        Some(Box::new(id.clone()))
    }
}

/// The diagnostic engine manages and reports diagnostics.
#[derive(Debug, Clone)]
pub struct DiagnosticEngine {
    source_map: SourceMap,
    diagnostics: Arc<Mutex<Vec<Diagnostic>>>,
    error_count: Arc<Mutex<usize>>,
    warning_count: Arc<Mutex<usize>>,
}

impl DiagnosticEngine {
    /// Creates a new DiagnosticEngine with a source map.
    pub fn new(source_map: SourceMap) -> Self {
        DiagnosticEngine {
            source_map,
            diagnostics: Arc::new(Mutex::new(Vec::new())),
            error_count: Arc::new(Mutex::new(0)),
            warning_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Emits a diagnostic.
    pub fn emit(&self, diagnostic: Diagnostic) {
        let mut diagnostics = self.diagnostics.lock();

        match diagnostic.severity {
            Severity::Error => {
                let mut error_count = self.error_count.lock();
                *error_count += 1;
            }
            Severity::Warning => {
                let mut warning_count = self.warning_count.lock();
                *warning_count += 1;
            }
            _ => {}
        }

        diagnostics.push(diagnostic);
    }

    /// Emits an error diagnostic.
    pub fn error<S: Into<String>>(&self, message: S, file: FileId, span: Span) {
        self.emit(Diagnostic::error(message, file, span));
    }

    /// Emits a warning diagnostic.
    pub fn warning<S: Into<String>>(&self, message: S, file: FileId, span: Span) {
        self.emit(Diagnostic::warning(message, file, span));
    }

    /// Emits a note diagnostic.
    pub fn note<S: Into<String>>(&self, message: S, file: FileId, span: Span) {
        self.emit(Diagnostic::note(message, file, span));
    }

    /// Returns the number of errors emitted.
    pub fn error_count(&self) -> usize {
        *self.error_count.lock()
    }

    /// Returns the number of warnings emitted.
    pub fn warning_count(&self) -> usize {
        *self.warning_count.lock()
    }

    /// Returns true if any errors have been emitted.
    pub fn has_errors(&self) -> bool {
        self.error_count() > 0
    }

    /// Prints all diagnostics to stderr.
    pub fn print_all(&self) {
        let diagnostics = self.diagnostics.lock();
        let mut stderr = io::stderr();

        for diagnostic in diagnostics.iter() {
            let _ = diagnostic.emit(&self.source_map, &mut stderr);
        }
    }

    /// Prints all diagnostics to a writer.
    pub fn print_all_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let diagnostics = self.diagnostics.lock();

        for diagnostic in diagnostics.iter() {
            diagnostic.emit(&self.source_map, writer)?;
        }

        Ok(())
    }

    /// Clears all diagnostics.
    pub fn clear(&self) {
        let mut diagnostics = self.diagnostics.lock();
        diagnostics.clear();

        let mut error_count = self.error_count.lock();
        *error_count = 0;

        let mut warning_count = self.warning_count.lock();
        *warning_count = 0;
    }

    /// Returns a reference to the source map.
    pub fn source_map(&self) -> &SourceMap {
        &self.source_map
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_severity() {
        assert_eq!(Severity::Error.as_str(), "error");
        assert_eq!(Severity::Warning.as_str(), "warning");
        assert!(Severity::Error < Severity::Warning);
        assert!(Severity::Warning < Severity::Note);
    }

    #[test]
    fn test_label_creation() {
        let span = Span::from_offsets(0, 5);
        let label = Label::new(span);

        assert_eq!(label.span, span);
        assert!(label.message.is_none());

        let label_with_msg = Label::with_message(span, "test message");
        assert_eq!(label_with_msg.message.as_ref().unwrap(), "test message");
    }

    #[test]
    fn test_diagnostic_creation() {
        let file = FileId::from_raw(0);
        let span = Span::from_offsets(10, 20);

        let diag = Diagnostic::error("test error", file, span);

        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.message, "test error");
        assert_eq!(diag.file, file);
        assert_eq!(diag.primary_span, span);
    }

    #[test]
    fn test_diagnostic_with_labels() {
        let file = FileId::from_raw(0);
        let span1 = Span::from_offsets(10, 20);
        let span2 = Span::from_offsets(30, 40);

        let label = Label::with_message(span2, "related location");
        let diag = Diagnostic::error("main error", file, span1)
            .with_label(label)
            .with_help("try fixing this")
            .with_note("additional context");

        assert_eq!(diag.labels.len(), 1);
        assert_eq!(diag.help.as_ref().unwrap(), "try fixing this");
        assert_eq!(diag.note.as_ref().unwrap(), "additional context");
    }

    #[test]
    fn test_diagnostic_engine() {
        let source_map = SourceMap::new();
        let file_id = source_map.add_file(
            PathBuf::from("test.cvb"),
            "let x = 42;\nlet y = x + 1;".to_string(),
        );

        let engine = DiagnosticEngine::new(source_map);

        assert_eq!(engine.error_count(), 0);
        assert_eq!(engine.warning_count(), 0);
        assert!(!engine.has_errors());

        let span = Span::from_offsets(4, 5).with_file_id(file_id);
        engine.error("undefined variable", file_id, span);

        assert_eq!(engine.error_count(), 1);
        assert!(engine.has_errors());

        engine.warning("unused variable", file_id, span);
        assert_eq!(engine.warning_count(), 1);
    }

    #[test]
    fn test_diagnostic_engine_clear() {
        let source_map = SourceMap::new();
        let file_id = source_map.add_file(PathBuf::from("test.cvb"), "code".to_string());
        let engine = DiagnosticEngine::new(source_map);

        let span = Span::from_offsets(0, 4).with_file_id(file_id);
        engine.error("error 1", file_id, span);
        engine.warning("warning 1", file_id, span);

        assert_eq!(engine.error_count(), 1);
        assert_eq!(engine.warning_count(), 1);

        engine.clear();

        assert_eq!(engine.error_count(), 0);
        assert_eq!(engine.warning_count(), 0);
    }

    #[test]
    fn test_emit_diagnostic() {
        let source_map = SourceMap::new();
        let file_id = source_map.add_file(
            PathBuf::from("example.cvb"),
            "let x = 42;\nlet y = x + 1;".to_string(),
        );

        let span = Span::from_offsets(4, 5).with_file_id(file_id);
        let diagnostic = Diagnostic::error("variable 'x' is already defined", file_id, span)
            .with_help("use a different name or remove the duplicate definition");

        let mut output = Vec::new();
        let result = diagnostic.emit(&source_map, &mut output);

        assert!(result.is_ok());
        assert!(!output.is_empty());
    }
}
