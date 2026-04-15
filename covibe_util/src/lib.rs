//! CoVibe Utility Library
//!
//! Core utilities shared across all compiler crates:
//! - Source file representation and management
//! - Span types for tracking source locations
//! - String interning for efficient symbol storage
//! - Diagnostic engine for beautiful error reporting

pub mod source;
pub mod span;
pub mod interner;
pub mod diagnostic;

pub use source::{SourceFile, SourceMap, FileId};
pub use span::{Span, BytePos, LineCol, HasSpan, Spanned};
pub use interner::{Symbol, Interner, KnownSymbols};
pub use diagnostic::{Diagnostic, DiagnosticEngine, Severity, Label};
