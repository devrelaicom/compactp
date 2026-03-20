//! JSON diagnostic renderer.
//!
//! Produces a machine-readable JSON representation of a diagnostic, including
//! resolved line/column information computed from the source text.

use crate::{Diagnostic, Severity};

/// Render a diagnostic as a [`serde_json::Value`].
///
/// The returned JSON object has the following shape:
/// ```json
/// {
///   "severity": "error",
///   "code": "E0012",
///   "message": "expected `;`",
///   "primary_span": {
///     "start": { "offset": 19, "line": 1, "column": 20 },
///     "end": { "offset": 20, "line": 1, "column": 21 }
///   },
///   "secondary_spans": [],
///   "notes": []
/// }
/// ```
///
/// Line and column numbers are 1-based in the JSON output.
pub fn render_json(diag: &Diagnostic, source: &str) -> serde_json::Value {
    let start_offset: usize = diag.primary_span.start().into();
    let end_offset: usize = diag.primary_span.end().into();
    let (start_line, start_col) = offset_to_line_col(source, start_offset);
    let (end_line, end_col) = offset_to_line_col(source, end_offset);

    let severity_str = match diag.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Note => "note",
    };

    let secondary: Vec<serde_json::Value> = diag
        .secondary_spans
        .iter()
        .map(|ls| {
            let s_offset: usize = ls.span.start().into();
            let e_offset: usize = ls.span.end().into();
            let (sl, sc) = offset_to_line_col(source, s_offset);
            let (el, ec) = offset_to_line_col(source, e_offset);
            serde_json::json!({
                "start": { "offset": s_offset, "line": sl, "column": sc },
                "end": { "offset": e_offset, "line": el, "column": ec },
                "label": ls.label,
            })
        })
        .collect();

    serde_json::json!({
        "severity": severity_str,
        "code": diag.code.to_string(),
        "message": diag.message,
        "primary_span": {
            "start": { "offset": start_offset, "line": start_line, "column": start_col },
            "end": { "offset": end_offset, "line": end_line, "column": end_col },
        },
        "secondary_spans": secondary,
        "notes": diag.notes,
    })
}

/// Convert a byte offset into a (1-based line, 1-based column) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(source.len());
    let mut line = 1;
    let mut col = 1;
    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Diagnostic, DiagnosticCode, LabeledSpan, Severity};
    use rowan::TextRange;

    #[test]
    fn render_json_error_diagnostic() {
        let diag = Diagnostic {
            severity: Severity::Error,
            code: DiagnosticCode::new("E", 12),
            message: "expected `;`".into(),
            primary_span: TextRange::new(19.into(), 20.into()),
            secondary_spans: vec![],
            notes: vec![],
        };
        let source = "ledger count: Field\n";
        let json = render_json(&diag, source);
        let pretty = serde_json::to_string_pretty(&json).unwrap();
        insta::assert_snapshot!(pretty);
    }

    #[test]
    fn render_json_with_secondary_spans() {
        let diag = Diagnostic {
            severity: Severity::Warning,
            code: DiagnosticCode::new("W", 5),
            message: "duplicate field name".into(),
            primary_span: TextRange::new(30.into(), 35.into()),
            secondary_spans: vec![LabeledSpan {
                span: TextRange::new(7.into(), 12.into()),
                label: Some("first defined here".into()),
            }],
            notes: vec!["rename one of the fields".into()],
        };
        let source = "ledger count: Field;\nledger count: Field;\n";
        let json = render_json(&diag, source);
        let pretty = serde_json::to_string_pretty(&json).unwrap();
        insta::assert_snapshot!(pretty);
    }
}
