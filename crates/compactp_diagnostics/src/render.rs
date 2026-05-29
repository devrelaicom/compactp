//! Human-readable diagnostic rendering in rustc style.
//!
//! Produces output like:
//! ```text
//! error[E0012]: expected `;`
//!  --> test.compact:1:20
//!   |
//! 1 | ledger count: Field
//!   |                    ^ expected `;`
//!   |
//! ```

use crate::{Diagnostic, Severity};

/// Render a diagnostic as a human-readable string with optional ANSI color.
///
/// # Arguments
///
/// * `diag` - The diagnostic to render.
/// * `source` - The full source text the diagnostic refers to.
/// * `filename` - The filename to display in the location line.
/// * `colored` - Whether to emit ANSI escape codes for color.
pub fn render_human(diag: &Diagnostic, source: &str, filename: &str, colored: bool) -> String {
    let mut out = String::new();

    let start: usize = diag.primary_span.start().into();
    let end: usize = diag.primary_span.end().into();

    let (line, col) = offset_to_line_col(source, start);
    let line_text = source_line(source, line);
    let line_number = line + 1; // 1-based display
    let col_display = col + 1; // 1-based display

    // Compute the width needed for the line number gutter.
    let gutter_width = line_number.to_string().len();

    // --- Header: severity[code]: message ---
    let severity_str = match diag.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Note => "note",
    };

    if colored {
        let color_code = severity_ansi_code(diag.severity);
        out.push_str(&format!(
            "\x1b[{color_code}m{severity_str}[{code}]\x1b[0m\x1b[1m: {message}\x1b[0m\n",
            code = diag.code,
            message = diag.message,
        ));
    } else {
        out.push_str(&format!(
            "{severity_str}[{code}]: {message}\n",
            code = diag.code,
            message = diag.message,
        ));
    }

    // --- Location: --> file:line:col ---
    out.push_str(&format!(
        "{pad} --> {filename}:{line_number}:{col_display}\n",
        pad = " ".repeat(gutter_width),
    ));

    // --- Blank gutter line ---
    out.push_str(&format!("{pad} |\n", pad = " ".repeat(gutter_width)));

    // --- Source line ---
    out.push_str(&format!("{line_number} | {line_text}\n",));

    // --- Caret underline ---
    // The underline length is the span length, clamped to at least 1 character
    // and at most the remaining characters on the line.
    let span_len = end.saturating_sub(start).max(1);
    let line_start_offset = line_start(source, line);
    let max_underline = line_text.len().saturating_sub(start - line_start_offset);
    let underline_len = span_len.min(max_underline).max(1);

    let leading_spaces = col;
    if colored {
        let color_code = severity_ansi_code(diag.severity);
        out.push_str(&format!(
            "{pad} | {spaces}\x1b[{color_code}m{carets}\x1b[0m {message}\n",
            pad = " ".repeat(gutter_width),
            spaces = " ".repeat(leading_spaces),
            carets = "^".repeat(underline_len),
            message = diag.message,
        ));
    } else {
        out.push_str(&format!(
            "{pad} | {spaces}{carets} {message}\n",
            pad = " ".repeat(gutter_width),
            spaces = " ".repeat(leading_spaces),
            carets = "^".repeat(underline_len),
            message = diag.message,
        ));
    }

    // --- Trailing gutter line ---
    out.push_str(&format!("{pad} |\n", pad = " ".repeat(gutter_width)));

    // --- Notes ---
    for note in &diag.notes {
        if colored {
            out.push_str(&format!(
                "{pad} = \x1b[1mnote\x1b[0m: {note}\n",
                pad = " ".repeat(gutter_width),
            ));
        } else {
            out.push_str(&format!(
                "{pad} = note: {note}\n",
                pad = " ".repeat(gutter_width),
            ));
        }
    }

    out
}

/// Convert a byte offset into a (0-based line, 0-based column) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(source.len());
    let mut line = 0;
    let mut col = 0;
    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// Get the byte offset of the start of the given 0-based line.
fn line_start(source: &str, target_line: usize) -> usize {
    if target_line == 0 {
        return 0;
    }
    let mut current_line = 0;
    for (i, ch) in source.char_indices() {
        if ch == '\n' {
            current_line += 1;
            if current_line == target_line {
                return i + 1;
            }
        }
    }
    source.len()
}

/// Extract the text content of a 0-based line (without trailing newline).
fn source_line(source: &str, line: usize) -> &str {
    let start = line_start(source, line);
    let rest = &source[start..];
    match rest.find('\n') {
        Some(end) => &rest[..end],
        None => rest,
    }
}

/// Return the ANSI color code for a given severity.
fn severity_ansi_code(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "1;31",   // bold red
        Severity::Warning => "1;33", // bold yellow
        Severity::Note => "1;36",    // bold cyan
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Diagnostic, DiagnosticCode, Severity};
    use rowan::TextRange;

    #[test]
    fn render_human_error_diagnostic() {
        let diag = Diagnostic {
            severity: Severity::Error,
            code: DiagnosticCode::new("E", 12),
            message: "expected `;`".into(),
            primary_span: TextRange::new(19.into(), 20.into()),
            secondary_spans: vec![],
            notes: vec![],
        };
        let source = "ledger count: Field\n";
        let rendered = render_human(&diag, source, "test.compact", false);
        insta::assert_snapshot!(rendered);
    }

    #[test]
    fn render_human_warning_diagnostic() {
        let diag = Diagnostic {
            severity: Severity::Warning,
            code: DiagnosticCode::new("W", 1),
            message: "unused field".into(),
            primary_span: TextRange::new(7.into(), 12.into()),
            secondary_spans: vec![],
            notes: vec!["consider removing this field".into()],
        };
        let source = "ledger count: Field;\n";
        let rendered = render_human(&diag, source, "example.compact", false);
        insta::assert_snapshot!(rendered);
    }

    #[test]
    fn render_human_with_notes() {
        let diag = Diagnostic {
            severity: Severity::Error,
            code: DiagnosticCode::new("E", 100),
            message: "unknown type `Strng`".into(),
            primary_span: TextRange::new(15.into(), 20.into()),
            secondary_spans: vec![],
            notes: vec![
                "did you mean `String`?".into(),
                "defined types: String, Int, Bool".into(),
            ],
        };
        let source = "ledger name : Strng;\n";
        let rendered = render_human(&diag, source, "types.compact", false);
        insta::assert_snapshot!(rendered);
    }

    #[test]
    fn render_human_multiline_number_gutter() {
        // Diagnostic on a line with a multi-digit line number.
        // Offset 66 points to 'f' (column 6) on line 11.
        let diag = Diagnostic {
            severity: Severity::Error,
            code: DiagnosticCode::new("E", 1),
            message: "unexpected token".into(),
            primary_span: TextRange::new(66.into(), 67.into()),
            secondary_spans: vec![],
            notes: vec![],
        };
        let source = "line1\nline2\nline3\nline4\nline5\n\
                       line6\nline7\nline8\nline9\nline10\n\
                       abcdefghijklmnopqrstuvwxyz\n";
        let rendered = render_human(&diag, source, "big.compact", false);
        insta::assert_snapshot!(rendered);
    }

    #[test]
    fn render_human_colored() {
        let diag = Diagnostic {
            severity: Severity::Error,
            code: DiagnosticCode::new("E", 12),
            message: "expected `;`".into(),
            primary_span: TextRange::new(19.into(), 20.into()),
            secondary_spans: vec![],
            notes: vec![],
        };
        let source = "ledger count: Field\n";
        let rendered = render_human(&diag, source, "test.compact", true);
        insta::assert_snapshot!(rendered);
    }
}
