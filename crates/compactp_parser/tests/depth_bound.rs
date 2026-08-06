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
//! Flat chains alone do **not** cover this. `lhs` is parsed before the
//! loop holds any charge and releases its own charges on the way out, so
//! a per-extension charge of one still lets the spine re-spend the budget
//! `lhs` already used. Nesting and chaining then compound into a
//! Θ(`max_depth`²) tree — at the default that is a 91x overshoot, enough
//! to abort on a 2 MiB stack again. The `composed_*` tests below are the
//! ones that distinguish a real bound from a path-only one; keep them.
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

/// Build a valid circuit that nests a chain inside a chain, `depth` times:
/// `E(0) = x`, `E(m) = open E(m-1) (sep tail)*links close`.
///
/// With `open`/`close` = `(`/`)` and `sep`/`tail` = `+`/`x` this is
/// `((( x +x+x… ) +x+x… ) +x+x… )` — the shape that separates a
/// height-aware depth charge from a path-only one. Each nesting level
/// hands the Pratt loop an already-built subtree to wrap, so a charge
/// that only counts path steps lets every level re-spend the full budget.
fn composed(depth: usize, links: usize, open: &str, close: &str, sep: &str, tail: &str) -> String {
    let mut body = String::from("x");
    for _ in 0..depth {
        let mut next = String::with_capacity(body.len() + links * (sep.len() + tail.len()) + 2);
        next.push_str(open);
        next.push_str(&body);
        for _ in 0..links {
            next.push_str(sep);
            next.push_str(tail);
        }
        next.push_str(close);
        body = next;
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
/// The invariant the parser establishes is on *charged* depth: the
/// counter tracks the deepest point of the subtree under construction
/// and never exceeds `max_depth`. Node depth is a constant multiple of
/// that, not equal to it, because a charge can sit beneath uncharged
/// wrapper nodes — so only a constant multiple is meaningful to assert.
///
/// The multiplier is set by the deepest wrapper stack the grammar can
/// place between two charges, which is three:
/// `PAREN_EXPR → EXPR_SEQ → ASSIGN_EXPR` for `( … =y,z )`. Element
/// positions contribute two (`ARRAY_EXPR → SPREAD_EXPR`, `CALL_EXPR →
/// NAMED_ARG`, `STRUCT_EXPR → STRUCT_FIELD_INIT`) at a flat 2.00x
/// regardless of nesting count — those are the first thing that would
/// shift if an element wrapper were added.
/// `uncharged_wrappers_bound_the_node_depth_ratio` pins all of them.
///
/// Measured worst over those families at `max_depth` 32/64/256 is
/// `3 x max_depth - 1` (95, 191, 767) — ratio 3.00, and *identical*
/// across `max_depth`, which is what makes it a constant rather than an
/// input-controlled multiplier. `4x` plus a constant leaves headroom for
/// a further wrapper while still failing loudly on the bug this file
/// exists to catch: the path-only charge produced ratio 91 at the
/// default (depth 23,297 against a 1,040 bound).
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

/// Nesting composed with chaining — the shape a flat-chain test cannot
/// catch.
///
/// Each nesting level hands the Pratt loop an already-built subtree to
/// wrap. A charge that counts only path steps lets every level re-spend
/// the budget the inner subtree already used, compounding to a
/// Θ(`max_depth`²) tree: at `max_depth` 256 this input reached depth
/// 23,297 (ratio 91) and aborted the process when dropped.
#[test]
fn composed_nesting_and_chaining_is_depth_bounded() {
    for (what, open, close, sep, tail) in [
        ("paren + infix", "(", ")", "+", "x"),
        ("paren + call", "(", ")", "", "()"),
        ("paren + member", "(", ")", "", ".a"),
        ("paren + index/cast", "(", ")", "", "[0] as Field"),
        ("array + infix", "[", "]", "+", "x"),
        ("unary + infix", "!(", ")", "+", "x"),
    ] {
        for max_depth in [16u32, 64, 256] {
            let n = max_depth as usize;
            let src = composed(n, n, open, close, sep, tail);
            let result = parse_with(
                &src,
                ParseOptions {
                    recover: true,
                    max_errors: 256,
                    max_depth,
                },
            );
            assert_bounded_and_lossless(
                result,
                &src,
                max_depth,
                &format!("composed {what} @ max_depth {max_depth}"),
            );
        }
    }
}

/// Uncharged wrapper nodes are what separate node depth from charged
/// depth, and they set the real constant in [`depth_limit`].
///
/// Between one depth charge and the next, an `lhs` arm opens a bounded
/// number of markers: one for most forms, two in element positions
/// (`ARRAY_EXPR → SPREAD_EXPR`, `CALL_EXPR → NAMED_ARG`,
/// `STRUCT_EXPR → STRUCT_FIELD_INIT`), and three at most —
/// `PAREN_EXPR → EXPR_SEQ → ASSIGN_EXPR` for `( … =y,z )`. Hence the
/// `3x` in [`depth_limit`]. Re-measure these if the grammar gains a
/// wrapper.
///
/// Two traps this test is shaped to avoid:
///
/// - Depth **peaks at `max_depth - 2` levels and collapses to 1x at
///   `max_depth` levels**: past the peak the cap fires mid-parse,
///   recovery desynchronises the trailing `=y,z`, and the extra markers
///   are abandoned. Pinning one level count would silently measure the
///   collapsed side, so scan a window.
/// - Passing the upper bound proves nothing on its own if the shape
///   stopped exercising its wrapper stack, so assert the peak actually
///   *reaches* the ratio too.
#[test]
fn uncharged_wrappers_bound_the_node_depth_ratio() {
    for (what, open, close, markers) in [
        ("paren", "(", ")", 1usize),
        ("array + spread", "[...", "]", 2),
        ("call + named arg", "g(a=", ")", 2),
        ("struct + field", "S{a:", "}", 2),
        ("paren + seq + assign", "(", "=y,z)", 3),
    ] {
        for max_depth in [32u32, 256] {
            let mut peak = 0usize;
            for levels in (max_depth as usize).saturating_sub(6)..=(max_depth as usize) {
                let src = composed(levels, 0, open, close, "", "");
                let result = parse_with(
                    &src,
                    ParseOptions {
                        recover: true,
                        max_errors: 256,
                        max_depth,
                    },
                );
                let root = bounded_root(
                    result,
                    max_depth,
                    &format!("{what} @ max_depth {max_depth}, {levels} levels"),
                );
                assert_eq!(
                    root.text().to_string(),
                    src,
                    "{what}: CST must round-trip to the input byte-for-byte"
                );
                peak = peak.max(max_node_depth(&root));
            }

            let expected = markers * max_depth as usize;
            let floor = expected * 9 / 10;
            assert!(
                peak >= floor,
                "{what} @ max_depth {max_depth}: peak depth {peak} never approached the \
                 {markers}x wrapper ratio (expected at least {floor}); this shape no longer \
                 exercises its wrapper stack, so it is not constraining depth_limit"
            );
        }
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
    let messages: Vec<String> = result.errors.iter().map(|e| e.message.clone()).collect();
    let root = bounded_root(result, 256, "realistic source");
    assert!(
        messages.is_empty(),
        "realistic source must parse cleanly: {messages:?}"
    );
    assert!(
        max_node_depth(&root) < 32,
        "realistic source must stay far below the default limit"
    );
}

/// Parse and drop `src` on a 2 MiB stack — the size a Tokio worker
/// thread uses by default, and the scenario issue #21 reported.
///
/// The drop is guarded by an iterative depth check so a regression
/// reports a depth rather than aborting the harness. At the default
/// `max_depth` the whole pipeline needs about 1 MiB in a debug build,
/// so 2 MiB exercises the real scenario without being flaky.
fn parse_and_drop_on_small_stack(src: String, what: &'static str) {
    let worker = std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
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
        Err(depth) => panic!(
            "{what}: CST depth {depth} exceeds bound {}",
            depth_limit(256)
        ),
    }
}

/// Issue #21's exact failure mode, flat-chain form.
#[test]
fn deep_chain_drops_cleanly_on_a_2mib_stack_thread() {
    parse_and_drop_on_small_stack(chained(LINKS, "+", "x"), "infix chain");
}

/// Issue #21's failure mode via the composed shape, which survived the
/// first (path-only) fix and aborted a 2 MiB thread again at default
/// options — 131 KB of valid Compact, CST depth 23,297.
#[test]
fn composed_input_drops_cleanly_on_a_2mib_stack_thread() {
    parse_and_drop_on_small_stack(composed(256, 256, "(", ")", "+", "x"), "composed shape");
}
