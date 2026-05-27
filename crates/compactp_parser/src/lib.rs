mod event;
pub(crate) mod grammar;
mod marker;
mod parser;
mod sink;

use rowan::GreenNode;

/// The result of parsing Compact source code.
pub struct ParseResult {
    pub green: GreenNode,
    pub errors: Vec<compactp_diagnostics::Diagnostic>,
}

/// Options controlling parser behavior.
pub struct ParseOptions {
    /// Whether to attempt error recovery (default: true).
    pub recover: bool,
    /// Maximum number of errors before the parser stops recovery (default: 256).
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

/// Parse Compact source code with default options.
pub fn parse(source: &str) -> ParseResult {
    parse_with(source, ParseOptions::default())
}

/// Parse Compact source code with custom options.
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
