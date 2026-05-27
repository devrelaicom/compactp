//! Compact parser — produces a lossless CST and a list of diagnostics.
//!
//! The parser is recursive-descent with marker-based AST construction
//! (rowan), Pratt-style expression precedence handling, and explicit
//! error recovery. The public surface is intentionally minimal:
//! [`parse`], [`parse_with`], [`ParseOptions`], and [`ParseResult`].
//! Walk the CST via the `compactp_syntax` types and the typed wrappers
//! in `compactp_ast`.

#![deny(missing_docs)]

mod event;
pub(crate) mod grammar;
mod marker;
mod parser;
mod sink;

use rowan::GreenNode;

/// The result of parsing Compact source code.
///
/// Contains a lossless [`GreenNode`] (the rowan CST root) and any
/// diagnostics emitted during parsing. Even when `errors` is non-empty
/// the CST is still well-formed and covers the full input text — error
/// recovery wraps unrecognized regions in `ERROR` nodes rather than
/// dropping them.
pub struct ParseResult {
    /// Lossless rowan CST root covering the full input text.
    pub green: GreenNode,
    /// Diagnostics emitted during parsing.
    ///
    /// May be non-empty even when parsing recovered successfully.
    pub errors: Vec<compactp_diagnostics::Diagnostic>,
}

/// Options controlling parser behavior.
pub struct ParseOptions {
    /// Whether to attempt error recovery (default: `true`).
    ///
    /// When `false`, the parser stops at the first error instead of
    /// inserting `ERROR` nodes and resynchronizing.
    pub recover: bool,
    /// Maximum number of errors before the parser stops recovery
    /// (default: `256`).
    pub max_errors: usize,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            recover: true,
            max_errors: 256,
        }
    }
}

/// Parses Compact source code into a lossless concrete syntax tree
/// plus a list of structured diagnostics.
///
/// Uses default parser options. For control over recovery and error
/// limits, use [`parse_with`].
///
/// # Examples
///
/// ```
/// use compactp_parser::parse;
/// use compactp_syntax::{SyntaxKind, SyntaxNode};
///
/// let result = parse("");
/// let root = SyntaxNode::new_root(result.green);
/// assert_eq!(root.kind(), SyntaxKind::SOURCE_FILE);
/// assert!(result.errors.is_empty());
/// ```
pub fn parse(source: &str) -> ParseResult {
    parse_with(source, ParseOptions::default())
}

/// Parses Compact source code into a lossless CST using the supplied
/// [`ParseOptions`].
///
/// Equivalent to [`parse`] when called with [`ParseOptions::default`].
pub fn parse_with(source: &str, opts: ParseOptions) -> ParseResult {
    let tokens = compactp_lexer::lex(source);
    let mut p = parser::Parser::new(tokens.clone());
    p.set_options(&opts);

    grammar::source_file(&mut p);

    let events = p.events;
    let (green, errors) = sink::Sink::new(events, tokens).finish();

    ParseResult { green, errors }
}

#[cfg(test)]
mod tests {
    use super::*;
    use compactp_syntax::{SyntaxKind::*, SyntaxNode};

    #[test]
    fn parse_empty_input() {
        let result = parse("");
        let root = SyntaxNode::new_root(result.green);
        assert_eq!(root.kind(), SOURCE_FILE);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn parse_single_ident() {
        let result = parse("x");
        let root = SyntaxNode::new_root(result.green);
        assert_eq!(root.kind(), SOURCE_FILE);
        // A bare identifier at top level triggers error recovery; it should be wrapped
        // in an ERROR node but still present in the tree.
        assert_eq!(root.text().to_string(), "x");
        assert!(
            !result.errors.is_empty(),
            "expected a parse error for bare identifier"
        );
        // Verify errors are Diagnostic values with message field
        assert!(!result.errors[0].message.is_empty());
    }

    #[test]
    fn parse_whitespace_preserved() {
        let result = parse("  \n  ");
        let root = SyntaxNode::new_root(result.green);
        assert_eq!(root.kind(), SOURCE_FILE);
        // Whitespace should be in the tree (lossless)
        let text: String = root.text().to_string();
        assert_eq!(text, "  \n  ");
    }

    #[test]
    fn parse_lossless_roundtrip() {
        let source = "circuit foo() { return 42; }";
        let result = parse(source);
        let root = SyntaxNode::new_root(result.green);
        assert_eq!(root.text().to_string(), source);
    }
}
