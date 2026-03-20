// Infrastructure modules contain APIs used by grammar rules added in subsequent tasks.
#[allow(dead_code)]
mod event;
pub mod grammar;
#[allow(dead_code)]
mod marker;
#[allow(dead_code)]
mod parser;
mod sink;

use rowan::GreenNode;

/// The result of parsing Compact source code.
pub struct ParseResult {
    pub green: GreenNode,
    pub errors: Vec<String>,
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

/// Parse a file from disk.
pub fn parse_file(path: &std::path::Path) -> Result<ParseResult, std::io::Error> {
    let source = std::fs::read_to_string(path)?;
    Ok(parse(&source))
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
        // The token should be inside the SOURCE_FILE
        let has_ident = root
            .children_with_tokens()
            .filter_map(|c| c.into_token())
            .any(|t| t.kind() == IDENT);
        assert!(has_ident);
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
