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
    ///
    /// A ceiling, not a target. Every list-parsing recovery loop
    /// consumes at least one token per iteration, so the number of
    /// diagnostics an input can produce is a function of that input and
    /// is reached well before the budget for anything but pathological
    /// source. Raising this therefore surfaces more of a broken file's
    /// real problems rather than manufacturing diagnostics: the count is
    /// stable once the budget exceeds what the input warrants.
    pub max_errors: usize,
    /// Maximum nesting depth the parser will build (default: `256`).
    ///
    /// The counter is charged on entry to each recursive grammar
    /// function — expression, type, statement, block, pattern, module,
    /// version term and contract — *and* per left-spine extension inside
    /// the expression Pratt loop. That second charge is what bounds the
    /// depth of the tree the parser *returns*, rather than only the
    /// depth of its own stack: left-associative operator chains
    /// (`x+x+...+x`) and postfix chains (`f()()...`, `x[0][0]...`,
    /// `x.a.a...`, `x as T as T...`) deepen the tree from inside a
    /// single stack frame, so charging entry alone would leave them
    /// unbounded. The spine charge is height-aware — it counts the
    /// height of the subtree being wrapped, not one step along the
    /// current path — so nesting and chaining cannot compound.
    ///
    /// On overflow the parser emits a recovery diagnostic and produces
    /// an `ERROR` node instead of nesting further. The CST stays
    /// lossless: every byte of the input remains recoverable.
    ///
    /// Those eight charges cover the whole grammar. Every recursion
    /// cycle in the parser passes through at least one of them — but
    /// call recursion is not the only way to deepen a tree.
    /// `CompletedMarker::precede` inserts a parent above an
    /// already-finished subtree from inside one stack frame, with no
    /// call cycle at all, which is what left
    /// [issue #21](https://github.com/devrelaicom/compactp/issues/21)
    /// unbounded; its six call sites are all inside the
    /// height-charged Pratt loop.
    /// Together those two facts leave no way to nest without spending
    /// the counter, so there is no longer a production that is
    /// unbounded at every setting of this option.
    ///
    /// Node depth is a small constant *multiple* of this value rather
    /// than equal to it, because a charge can sit beneath up to three
    /// uncharged wrapper nodes. Three grammars reach that maximum —
    /// `TYPE_REF` → `GENERIC_ARG_LIST` → `GENERIC_ARG` for `A<A<…>>`,
    /// `PAREN_EXPR` → `EXPR_SEQ` → `ASSIGN_EXPR` for `( … =y,z )`, and
    /// `VERSION_PAREN_EXPR` → `VERSION_OR_EXPR` → `VERSION_AND_EXPR` for
    /// `( … &&2||3&&4 )`. Measured over flat chains, pure nesting,
    /// element-position wrappers, composed nesting-plus-chaining, nested
    /// destructuring patterns, nested modules, nested contracts through
    /// each of the four routes that reach them, nested version terms and
    /// nested generic types in every position that accepts one, the
    /// worst node depth is `3 × max_depth + 4` (772 at the default),
    /// from a generic type in parameter or struct-field position such as
    /// `circuit f(a: A<A<…>>)`. Peak depth is *exactly* affine in
    /// `max_depth` at 8, 32, 64, 128, 256 and 512, so it is a genuine
    /// constant rather than something the input can drive. That figure
    /// is a measurement over those families, not a proof over every
    /// possible input.
    ///
    /// # Stack cost
    ///
    /// This value is effectively a stack budget, for two parties:
    ///
    /// - *Consumers* of the tree — rowan drops a tree recursively, and
    ///   most CST walks recurse in the tree's depth.
    /// - The *parser itself* — nested input recurses through the grammar
    ///   in step with the counter.
    ///
    /// Measured against the worst-case accepted shapes above, on
    /// `aarch64-apple-darwin`: at the default, parsing and dropping one
    /// needs under 1 MiB of stack in a debug build and under 256 KiB in
    /// release — comfortably inside a 2 MiB thread (the common
    /// async-runtime worker default). Raising the limit scales both
    /// costs linearly: on a 2 MiB stack the deepest setting the parser
    /// itself survives is around 640 in debug and around 3,100 in
    /// release. Frame sizes vary by target and optimization level, so
    /// treat those as order-of-magnitude figures and raise this only
    /// alongside a thread stack sized to match. For scale, the deepest
    /// file in the upstream Compact corpus has a CST depth of 20.
    pub max_depth: u32,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            recover: true,
            max_errors: 256,
            max_depth: 256,
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
