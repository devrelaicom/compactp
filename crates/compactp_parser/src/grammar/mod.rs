mod declarations;
mod expressions;
mod imports;
mod patterns;
mod statements;
mod types;
mod version;

use crate::parser::Parser;
use compactp_syntax::SyntaxKind;
use compactp_syntax::SyntaxKind::*;

/// Run one iteration of a recovery loop, guaranteeing forward progress.
///
/// The loops that repeatedly parse list elements — declarations at file
/// scope, statements in a block, members of a `module` or `contract` —
/// are bounded only by `errors_exhausted()`. That bounds the *diagnostic
/// budget*, not the loop: a production that reports an error and
/// consumes nothing leaves the loop to run again on the same token, so
/// the loop emits diagnostics until it reaches `max_errors`. The count
/// then tracks `ParseOptions::max_errors` rather than the size of the
/// input, turning a public tuning knob into a memory and CPU amplifier —
/// `export`, six bytes, reaches `max_errors` diagnostics on its own.
///
/// Requiring a token per iteration removes the amplifier at its source.
/// A stalled production has already reported the problem, so the skipped
/// token is wrapped in `ERROR` without a second diagnostic; a stall that
/// somehow reported *nothing* gets one, so a token is never discarded
/// silently. Either way the token stays in the tree and the CST remains
/// lossless.
///
/// Loops whose body ends in an unconditional `break` on a malformed
/// element (`struct_def`, `record_type`) terminate structurally and do
/// not need this.
fn step_ensuring_progress<F>(p: &mut Parser, parse_fn: F)
where
    F: FnOnce(&mut Parser),
{
    let tok_before = p.tok_pos();
    let errors_before = p.error_count;
    parse_fn(p);
    if p.tok_pos() != tok_before || p.at_end() {
        return;
    }
    if p.error_count == errors_before {
        p.error("unexpected token");
    }
    let m = p.start();
    p.bump_any();
    m.complete(p, ERROR);
}

/// Parse a complete source file: `source_file → pelt* EOF`
pub(crate) fn source_file(p: &mut Parser) {
    let m = p.start();
    while !p.at_end() {
        if p.errors_exhausted() {
            // Wrap all remaining tokens in ERROR
            let err = p.start();
            while !p.at_end() {
                p.bump_any();
            }
            err.complete(p, ERROR);
            break;
        }
        step_ensuring_progress(p, declarations::declaration);
    }
    // Eat trailing trivia so it's inside the SOURCE_FILE node
    p.eat_trivia();
    m.complete(p, SOURCE_FILE);
}

/// Top-level keywords used for error recovery synchronization.
const TOP_LEVEL_KEYWORDS: &[SyntaxKind] = &[
    PRAGMA_KW,
    INCLUDE_KW,
    IMPORT_KW,
    EXPORT_KW,
    MODULE_KW,
    LEDGER_KW,
    CONSTRUCTOR_KW,
    CIRCUIT_KW,
    WITNESS_KW,
    CONTRACT_KW,
    STRUCT_KW,
    ENUM_KW,
    PURE_KW,
    SEALED_KW,
    EOF,
];

/// Skip tokens until we find one in the recovery set, wrapping skipped tokens in ERROR.
fn error_recover_to_declaration(p: &mut Parser) {
    let m = p.start();
    p.error("unexpected token at top level");
    while !p.at_end() && !TOP_LEVEL_KEYWORDS.contains(&p.current()) {
        p.bump_any();
    }
    m.complete(p, ERROR);
}

/// Parse a comma-separated list of items using `parse_fn`, ending when `close` is seen.
/// Does NOT consume the closing delimiter.
fn comma_sep<F>(p: &mut Parser, close: SyntaxKind, mut parse_fn: F)
where
    F: FnMut(&mut Parser),
{
    if !p.at(close) {
        parse_fn(p);
        while p.eat(COMMA) {
            // Allow trailing comma
            if p.at(close) {
                break;
            }
            parse_fn(p);
        }
    }
}

/// Parse a comma-separated list requiring at least one element.
fn comma_sep1<F>(p: &mut Parser, close: SyntaxKind, mut parse_fn: F)
where
    F: FnMut(&mut Parser),
{
    parse_fn(p);
    while p.eat(COMMA) {
        if p.at(close) {
            break;
        }
        parse_fn(p);
    }
}

#[cfg(test)]
mod tests {
    use crate::parse;
    use compactp_syntax::SyntaxNode;
    use expect_test::{Expect, expect};

    /// Parse source and format the CST as an indented tree for snapshot comparison.
    pub(crate) fn check(input: &str, expected: Expect) {
        let result = parse(input);
        let root = SyntaxNode::new_root(result.green);
        let mut buf = String::new();
        format_tree(&root, 0, &mut buf);
        if !result.errors.is_empty() {
            buf.push_str("errors:\n");
            for e in &result.errors {
                buf.push_str(&format!("  {}\n", e.message));
            }
        }
        expected.assert_eq(&buf);
    }

    fn format_tree(node: &SyntaxNode, indent: usize, buf: &mut String) {
        let pad = "  ".repeat(indent);
        buf.push_str(&format!("{pad}{:?}@{:?}\n", node.kind(), node.text_range()));
        for child in node.children_with_tokens() {
            match child {
                rowan::NodeOrToken::Node(n) => {
                    format_tree(&n, indent + 1, buf);
                }
                rowan::NodeOrToken::Token(t) => {
                    buf.push_str(&format!(
                        "{pad}  {:?}@{:?} {:?}\n",
                        t.kind(),
                        t.text_range(),
                        t.text()
                    ));
                }
            }
        }
    }

    #[test]
    fn parse_empty() {
        check(
            "",
            expect![[r#"
                SOURCE_FILE@0..0
            "#]],
        );
    }

    #[test]
    fn parse_error_recovery_at_top_level() {
        check(
            "@@@ circuit foo() : Field { }",
            expect![[r#"
                SOURCE_FILE@0..29
                  ERROR@0..3
                    ERROR@0..1 "@"
                    ERROR@1..2 "@"
                    ERROR@2..3 "@"
                  CIRCUIT_DEF@3..29
                    WHITESPACE@3..4 " "
                    CIRCUIT_KW@4..11 "circuit"
                    WHITESPACE@11..12 " "
                    IDENT@12..15 "foo"
                    L_PAREN@15..16 "("
                    R_PAREN@16..17 ")"
                    WHITESPACE@17..18 " "
                    COLON@18..19 ":"
                    FIELD_TYPE@19..25
                      WHITESPACE@19..20 " "
                      FIELD_KW@20..25 "Field"
                    BLOCK@25..29
                      WHITESPACE@25..26 " "
                      L_BRACE@26..27 "{"
                      WHITESPACE@27..28 " "
                      R_BRACE@28..29 "}"
                errors:
                  unexpected token at top level
            "#]],
        );
    }
}
