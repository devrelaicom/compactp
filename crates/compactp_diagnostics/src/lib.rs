//! Structured diagnostics produced by the compactp parser frontend.
//!
//! This crate defines [`Diagnostic`], [`Severity`], [`DiagnosticCode`], and
//! [`LabeledSpan`] — the types every parser-side error or warning flows
//! through, and that every downstream consumer (CLI, library users, IDEs)
//! reads.
//!
//! Two renderers are provided: [`render_human`] for terminal-style rustc-like
//! output and [`render_json`] for machine-readable JSON.

#![deny(missing_docs)]

pub mod json;
pub mod render;

pub use json::render_json;
pub use render::render_human;

use rowan::TextRange;
use serde::Serialize;

/// A diagnostic message produced during parsing.
///
/// Combines a severity, a structured numeric code, a primary span (where the
/// issue is in source), optional secondary labeled spans for context, and
/// freeform notes printed below the rendered snippet.
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    /// Severity classification: error, warning, or note.
    pub severity: Severity,
    /// Structured diagnostic code (e.g. `E0012`).
    pub code: DiagnosticCode,
    /// Primary human-readable message shown in the rendered diagnostic.
    pub message: String,
    /// Source span this diagnostic refers to.
    #[serde(serialize_with = "serialize_text_range")]
    pub primary_span: TextRange,
    /// Optional secondary labeled spans providing additional context.
    pub secondary_spans: Vec<LabeledSpan>,
    /// Additional contextual notes printed below the span snippet.
    pub notes: Vec<String>,
}

impl Diagnostic {
    /// Create an error-level diagnostic.
    pub fn error(code: DiagnosticCode, message: String, span: TextRange) -> Self {
        Self {
            severity: Severity::Error,
            code,
            message,
            primary_span: span,
            secondary_spans: vec![],
            notes: vec![],
        }
    }

    /// Create a warning-level diagnostic.
    pub fn warning(code: DiagnosticCode, message: String, span: TextRange) -> Self {
        Self {
            severity: Severity::Warning,
            code,
            message,
            primary_span: span,
            secondary_spans: vec![],
            notes: vec![],
        }
    }

    /// Create a note-level diagnostic.
    pub fn note(code: DiagnosticCode, message: String, span: TextRange) -> Self {
        Self {
            severity: Severity::Note,
            code,
            message,
            primary_span: span,
            secondary_spans: vec![],
            notes: vec![],
        }
    }

    /// Append a note to this diagnostic (builder-style).
    #[must_use]
    pub fn with_note(mut self, note: String) -> Self {
        self.notes.push(note);
        self
    }

    /// Append a secondary labeled span to this diagnostic (builder-style).
    #[must_use]
    pub fn with_secondary(mut self, span: TextRange, label: Option<String>) -> Self {
        self.secondary_spans.push(LabeledSpan { span, label });
        self
    }
}

/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// A fatal parse or analysis error.
    Error,
    /// A non-fatal warning.
    Warning,
    /// An informational note.
    Note,
}

/// A structured diagnostic code (e.g., `E0001`).
///
/// Renders via [`std::fmt::Display`] as the prefix followed by the number
/// zero-padded to four digits (e.g., `E0012`).
#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticCode {
    /// Letter prefix identifying the code family (e.g. `"E"` for errors,
    /// `"W"` for warnings).
    pub prefix: &'static str,
    /// Numeric portion of the code, rendered zero-padded to four digits.
    pub number: u16,
}

impl DiagnosticCode {
    /// Construct a new diagnostic code from a static prefix and number.
    pub fn new(prefix: &'static str, number: u16) -> Self {
        Self { prefix, number }
    }
}

impl std::fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{:04}", self.prefix, self.number)
    }
}

/// A labeled span for secondary diagnostic locations.
#[derive(Debug, Clone, Serialize)]
pub struct LabeledSpan {
    /// Source range this secondary span points to.
    #[serde(serialize_with = "serialize_text_range")]
    pub span: TextRange,
    /// Optional human-readable label describing the span's role.
    pub label: Option<String>,
}

fn serialize_text_range<S: serde::Serializer>(
    range: &TextRange,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeStruct;
    let mut s = serializer.serialize_struct("TextRange", 2)?;
    s.serialize_field("start", &u32::from(range.start()))?;
    s.serialize_field("end", &u32::from(range.end()))?;
    s.end()
}
