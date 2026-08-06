//! `ParseOptions::max_depth` bounds the depth of the *returned tree*.
//!
//! Regression coverage for issue #21. A long left-associative operator
//! chain (`x+x+...+x`) or postfix chain (`f()()...`, `x[0][0]...`,
//! `x.a.a...`, `x as T as T...`) is valid Compact, and the Pratt loop
//! grows it by wrapping the current left-hand side in a new parent on
//! every iteration — all inside a single stack frame. Charging the depth
//! counter only on *entry* to `expr_bp` therefore bounded the parser's
//! recursion but not the tree it handed back: ~10 KB of valid source
//! produced a 5000-deep CST with zero diagnostics, and rowan's recursive
//! `Drop` then aborted the process (SIGABRT, uncatchable) on any thread
//! with a 2 MiB stack.
//!
//! Note on test hygiene: every assertion below is written so that a
//! regression fails the test rather than aborting the test process. A
//! tree deep enough to trip these bounds is also deep enough to overflow
//! the stack when dropped, so the depth check runs first and the tree is
//! deliberately leaked before panicking.

use compactp_parser::{ParseOptions, ParseResult, parse, parse_with};
use compactp_syntax::SyntaxNode;

const PRE: &str =
    "pragma language_version >= 0.23;\nexport pure circuit f(x: Field): Field { return ";
const POST: &str = "; }\n";

/// Number of chain links used by the tests. Matches the issue's reproducer.
const LINKS: usize = 5000;

/// Build a valid circuit whose body is `x`, extended `n - 1` times by
/// `sep` + `tail` (e.g. `sep = "+"`, `tail = "x"` gives `x+x+...+x`).
fn chained(n: usize, sep: &str, tail: &str) -> String {
    let mut body = String::from("x");
    for _ in 1..n {
        body.push_str(sep);
        body.push_str(tail);
    }
    format!("{PRE}{body}{POST}")
}

/// Maximum node depth of `root`, computed with an explicit work-stack.
///
/// Deliberately iterative: a recursive walk would overflow the stack on
/// exactly the input these tests exercise, which would turn a reportable
/// assertion failure into an uncatchable process abort.
fn max_node_depth(root: &SyntaxNode) -> usize {
    let mut stack = vec![(root.clone(), 1usize)];
    let mut max = 0usize;
    while let Some((node, depth)) = stack.pop() {
        if depth > max {
            max = depth;
        }
        for child in node.children() {
            stack.push((child, depth + 1));
        }
    }
    max
}

/// Depth a tree parsed with `max_depth` is allowed to reach.
///
/// A single depth charge can sit beneath a few uncharged wrapper nodes
/// (`PAREN_EXPR`, `EXPR_SEQ`, …), so the bound is a constant multiple
/// rather than equality. Measured worst case across the shapes below is
/// ~1.25x; 4x leaves room for grammar changes while still being three
/// orders of magnitude below the unbounded depth (`LINKS`) this test
/// exists to catch.
fn depth_limit(max_depth: u32) -> usize {
    4 * max_depth as usize + 16
}

/// Take the CST out of `result`, first proving it is shallow enough that
/// dropping it is safe.
///
/// On a depth violation the tree is leaked rather than dropped: a
/// regression makes it thousands of levels deep, and rowan drops
/// recursively, so dropping here would abort the whole test process
/// before the failure could be reported. Every test in this file routes
/// tree ownership through this function for that reason.
fn bounded_root(result: ParseResult, max_depth: u32, what: &str) -> SyntaxNode {
    let root = SyntaxNode::new_root(result.green);
    let depth = max_node_depth(&root);
    let limit = depth_limit(max_depth);
    if depth > limit {
        std::mem::forget(root);
        panic!("{what}: CST depth {depth} exceeds bound {limit} for max_depth {max_depth}");
    }
    root
}

/// Assert the parse is depth-bounded and byte-exactly lossless.
fn assert_bounded_and_lossless(result: ParseResult, src: &str, max_depth: u32, what: &str) {
    let root = bounded_root(result, max_depth, what);
    assert_eq!(
        root.text().to_string(),
        src,
        "{what}: CST must round-trip to the input byte-for-byte"
    );
}

/// The headline case from issue #21: `return x+x+...+x;` with 5000 terms.
#[test]
fn left_associative_infix_chain_is_depth_bounded() {
    let src = chained(LINKS, "+", "x");
    assert_bounded_and_lossless(parse(&src), &src, 256, "infix chain");
}

/// Postfix chains extend the same left spine and were unbounded for the
/// same reason.
#[test]
fn postfix_chains_are_depth_bounded() {
    for (what, sep, tail) in [
        ("call chain", "", "()"),
        ("index chain", "", "[0]"),
        ("member chain", "", ".a"),
        ("cast chain", " as Field", ""),
        ("mixed postfix chain", "", ".a[0]()+x"),
    ] {
        let src = chained(LINKS, sep, tail);
        assert_bounded_and_lossless(parse(&src), &src, 256, what);
    }
}

/// A non-default `max_depth` must bound the returned tree too — both
/// downward (stricter) and upward (the documented escape hatch).
#[test]
fn max_depth_option_bounds_the_returned_tree() {
    for max_depth in [8u32, 32, 64, 256, 1024] {
        let src = chained(LINKS, "+", "x");
        let result = parse_with(
            &src,
            ParseOptions {
                recover: true,
                max_errors: 256,
                max_depth,
            },
        );
        assert_bounded_and_lossless(result, &src, max_depth, &format!("max_depth {max_depth}"));
    }
}

/// The bound is announced, not silent. Truncating the tree without a
/// diagnostic would make valid-looking input parse "cleanly" into a tree
/// that no longer reflects the source.
#[test]
fn depth_bounded_chain_reports_a_diagnostic() {
    let src = chained(LINKS, "+", "x");
    let result = parse(&src);
    let messages: Vec<String> = result.errors.iter().map(|e| e.message.clone()).collect();
    drop(bounded_root(result, 256, "diagnostic check"));

    assert!(
        messages.iter().any(|m| m.contains("nesting depth limit")),
        "expected a nesting-depth diagnostic, got: {:?}",
        &messages[..messages.len().min(5)]
    );
}

/// Ordinary Compact must be nowhere near the limit — the bound may not
/// turn realistic source into `ERROR` nodes.
#[test]
fn realistic_nesting_is_unaffected() {
    let src = concat!(
        "pragma language_version >= 0.23;\n",
        "export pure circuit f(x: Field, y: Field): Field {\n",
        "  const a = x + y * (x - y) + x.b.c[0] as Field;\n",
        "  return a > 0 ? a + x + y + x * y : (x + y) * (x - y);\n",
        "}\n"
    );
    let result = parse(src);
    assert!(
        result.errors.is_empty(),
        "realistic source must parse cleanly: {:?}",
        result.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
    let root = SyntaxNode::new_root(result.green);
    assert!(
        max_node_depth(&root) < 32,
        "realistic source must stay far below the default limit"
    );
}

/// Issue #21's exact failure mode: dropping the tree on a 2 MiB stack —
/// the size a Tokio worker thread uses by default — aborted the process.
///
/// The drop is guarded by an iterative depth check so a regression
/// returns an error to be reported rather than aborting the harness.
#[test]
fn deep_chain_drops_cleanly_on_a_2mib_stack_thread() {
    let worker = std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let src = chained(LINKS, "+", "x");
            let root = SyntaxNode::new_root(parse(&src).green);
            let depth = max_node_depth(&root);
            if depth > depth_limit(256) {
                // Never drop a tree this deep: rowan's recursive `Drop`
                // would abort the process instead of failing the test.
                std::mem::forget(root);
                return Err(depth);
            }
            drop(root); // issue #21: this aborted the process
            Ok(depth)
        })
        .expect("spawn 2 MiB worker thread");

    match worker
        .join()
        .expect("worker thread must not abort or panic")
    {
        Ok(_) => {}
        Err(depth) => panic!("CST depth {depth} exceeds bound {}", depth_limit(256)),
    }
}
