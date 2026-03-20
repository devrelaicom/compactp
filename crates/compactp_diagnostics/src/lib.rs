pub mod json;
pub mod render;

use rowan::TextRange;
use serde::Serialize;

/// A diagnostic message produced during parsing.
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: DiagnosticCode,
    pub message: String,
    #[serde(serialize_with = "serialize_text_range")]
    pub primary_span: TextRange,
    pub secondary_spans: Vec<LabeledSpan>,
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
    Error,
    Warning,
    Note,
}

/// A structured diagnostic code (e.g., E0001).
#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticCode {
    pub prefix: &'static str,
    pub number: u16,
}

impl DiagnosticCode {
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
    #[serde(serialize_with = "serialize_text_range")]
    pub span: TextRange,
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
