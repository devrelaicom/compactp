//! `ParseOptions::max_depth` bounds the depth of the *returned tree*.
//!
//! Regression coverage for issues #21 and #23. The two are different
//! failure modes of the same invariant, and both are covered here.
//!
//! **#23** — the simpler one. Three recursive productions never charged
//! the counter at all: `pattern` (`pattern → tuple_pattern →
//! tuple_pat_elt → pattern`), `module_def` (`module_def → declaration →
//! module_def`) and `version_term` (`version_term → version_or_expr →
//! version_and_expr → version_term`). No value of `max_depth`
//! constrained them: `max_depth = 1` still parsed 2001-deep module
//! nesting with zero diagnostics, and each shape recursed the *parser*
//! deep enough to overflow the stack and abort the process. Unlike #21
//! this was a parse-time overflow, not a drop-time one — the
//! `mem::forget` control aborts at the same input size as the drop path
//! — so a plain path charge on entry is the fix, not a height-aware one.
//! These grammars build every node above the path currently on the
//! stack (nothing outside `expressions.rs` calls
//! `CompletedMarker::precede`), so node depth stays a fixed multiple of
//! the charge.
//!
//! **#21** — the subtle one. A long left-associative operator
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
/// The multiplier is set by the deepest wrapper stack any grammar can
/// place between two charges, which is three. Three shapes reach it:
///
/// - types: `TYPE_REF → GENERIC_ARG_LIST → GENERIC_ARG` for `A<A<…>>`
/// - expressions: `PAREN_EXPR → EXPR_SEQ → ASSIGN_EXPR` for `( … =y,z )`
/// - version terms: `VERSION_PAREN_EXPR → VERSION_OR_EXPR →
///   VERSION_AND_EXPR` for `( … &&2||3&&4 )`
///
/// Element positions contribute two (`ARRAY_EXPR → SPREAD_EXPR`,
/// `CALL_EXPR → NAMED_ARG`, `STRUCT_EXPR → STRUCT_FIELD_INIT`), as do
/// patterns (`TUPLE_PAT → TUPLE_PAT_ELT`); modules contribute one
/// (`MODULE_DEF`). `uncharged_wrappers_bound_the_node_depth_ratio` and
/// `node_depth_is_affine_in_max_depth` pin all of them.
///
/// Measured worst over those families, scanning every level count up to
/// `4 x max_depth` at `max_depth` 8/32/64/128/256/512, is
/// `3 x max_depth + 4` — 772 at the default. The worst shape is a
/// generic type in parameter, struct-field or contract-member position
/// (`circuit f(a: A<A<…>>)`), which pays the three type wrappers plus
/// one uncharged node above the first charge. Peak depth is *exactly*
/// affine in `max_depth` at every one of those six settings, which is
/// what makes the multiplier a constant rather than something the input
/// can drive. `4x` plus a constant leaves headroom for a further wrapper
/// while still failing loudly on the bugs this file exists to catch:
/// #21's path-only charge produced ratio 91 at the default (depth 23,297
/// against a 1,040 bound), and #23's uncharged grammars produced depth
/// 4,005 from 4 KB of input.
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

// ---------------------------------------------------------------------
// Issue #23: pattern, module and version nesting
// ---------------------------------------------------------------------

/// `open` repeated `levels` times, then `seed`, then `close` repeated
/// `levels` times — a purely nested shape with no chaining.
fn nested(levels: usize, open: &str, close: &str, seed: &str) -> String {
    let mut body = String::from(seed);
    for _ in 0..levels {
        let mut next = String::with_capacity(body.len() + open.len() + close.len());
        next.push_str(open);
        next.push_str(&body);
        next.push_str(close);
        body = next;
    }
    body
}

/// `circuit f() : Field { const <nested pattern> = x; }` — reaches
/// `pattern` through `const_binding`.
fn const_pattern(levels: usize, open: &str, close: &str) -> String {
    format!(
        "circuit f() : Field {{ const {} = x; }}",
        nested(levels, open, close, "a")
    )
}

/// `circuit f(<nested pattern>: Field) : Field { return x; }` — reaches
/// `pattern` through `param`, one wrapper deeper than `const_pattern`.
fn param_pattern(levels: usize) -> String {
    format!(
        "circuit f({}: Field) : Field {{ return x; }}",
        nested(levels, "[", "]", "a")
    )
}

/// `module M{ module M{ … } }`, optionally `export`ed at every level.
fn modules(levels: usize, exported: bool, innermost: &str) -> String {
    let head = if exported {
        "export module M{"
    } else {
        "module M{"
    };
    let mut s = String::with_capacity(levels * (head.len() + 1) + innermost.len());
    for _ in 0..levels {
        s.push_str(head);
    }
    s.push_str(innermost);
    for _ in 0..levels {
        s.push('}');
    }
    s
}

/// `pragma language_version <nested version term>;`
fn pragma_version(levels: usize, open: &str, close: &str, seed: &str) -> String {
    format!(
        "pragma language_version {};",
        nested(levels, open, close, seed)
    )
}

/// `circuit f(): A<A<…<B>…>> { return x; }` — nests the type grammar
/// through `TYPE_REF → GENERIC_ARG_LIST → GENERIC_ARG`, three uncharged
/// wrapper nodes per `ty` charge.
fn generic_return_type(levels: usize) -> String {
    format!(
        "circuit f(): {} {{ return x; }}",
        nested(levels, "A<", ">", "B")
    )
}

/// The same nesting one node further down, in parameter position. This
/// is the deepest tree the parser will build at a given `max_depth`.
fn generic_param_type(levels: usize) -> String {
    format!(
        "circuit f(a: {}): Field {{ return x; }}",
        nested(levels, "A<", ">", "B")
    )
}

/// `max_depth` values the #23 families are exercised at.
///
/// Spread wide enough that a bound which is secretly a function of the
/// input would not hold at all four, and capped at the default because
/// nothing above it is a supported configuration without a matching
/// thread stack — `max_depth_option_bounds_the_returned_tree` already
/// covers the raised-limit case for the expression grammar.
const MDS: [u32; 4] = [8, 32, 64, 256];

/// Run `build` at several `max_depth` values with a level count far past
/// the limit, asserting the returned tree is bounded and lossless.
fn assert_family_bounded(what: &str, build: impl Fn(usize) -> String) {
    for max_depth in MDS {
        for levels in [max_depth as usize, 4 * max_depth as usize] {
            let src = build(levels);
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
                &format!("{what} @ max_depth {max_depth}, {levels} levels"),
            );
        }
    }
}

/// Issue #23: `pattern → tuple_pattern → tuple_pat_elt → pattern` (and
/// the `struct_pat_field` equivalent) never charged the counter, so
/// `const [[[…a…]]] = x;` built a tree as deep as the brackets were
/// nested — 4,005 levels from 4 KB of input at the default `max_depth`,
/// with zero diagnostics — and overflowed the parser's own stack at
/// around 37,500.
#[test]
fn pattern_nesting_is_depth_bounded() {
    assert_family_bounded("tuple pattern", |n| const_pattern(n, "[", "]"));
    assert_family_bounded("struct pattern", |n| const_pattern(n, "{a:", "}"));
    assert_family_bounded("mixed pattern", |n| const_pattern(n, "[{a:", "}]"));
    assert_family_bounded("pattern with siblings", |n| const_pattern(n, "[b,", ",c]"));
    assert_family_bounded("param pattern", param_pattern);
    // Reached through `pattern_or_parg`, which enters `tuple_pattern`
    // directly rather than through the charged `pattern` entry point.
    assert_family_bounded("lambda param pattern", |n| {
        format!(
            "circuit f() : Field {{ const g = ({}) => x; }}",
            nested(n, "[", "]", "a")
        )
    });
}

/// Issue #23: `module_def → declarations::declaration → module_def`
/// never charged the counter, so `module M{ module M{ … } }` was
/// unbounded at *every* `max_depth` — the issue's headline evidence was
/// 2001-deep nesting parsed cleanly at `max_depth = 1`.
#[test]
fn module_nesting_is_depth_bounded() {
    assert_family_bounded("module nesting", |n| modules(n, false, ""));
    assert_family_bounded("exported module nesting", |n| modules(n, true, ""));
    assert_family_bounded("module nesting with a body", |n| {
        modules(n, false, "circuit f() : Field { return a+b*(c-d); }")
    });
    // Two charged grammars sharing one budget: modules on the outside,
    // a deep pattern underneath. The total must still be bounded.
    assert_family_bounded("module nesting over a deep pattern", |n| {
        modules(n, false, &const_pattern(n, "[", "]"))
    });
}

/// Issue #23: `version_term → version_or_expr → version_and_expr →
/// version_term` never charged the counter. This is the cheapest of the
/// three to drive — it overflowed the parser's stack at around 21,750
/// parentheses — and the only one reachable through a single pragma,
/// i.e. through the CLI on a one-line file.
#[test]
fn version_nesting_is_depth_bounded() {
    assert_family_bounded("version parens", |n| pragma_version(n, "(", ")", ">= 0.23"));
    assert_family_bounded("version negation", |n| {
        format!("pragma language_version {}0.23;", "!".repeat(n))
    });
    // The worst wrapper stack in the whole grammar: three uncharged
    // nodes per charge (`VERSION_PAREN_EXPR → VERSION_OR_EXPR →
    // VERSION_AND_EXPR`).
    assert_family_bounded("version or/and", |n| {
        pragma_version(n, "(", "&&2||3&&4)", "0.1")
    });
    assert_family_bounded("version negation inside or/and", |n| {
        pragma_version(n, "(!", "&&2||3&&4)", "0.1")
    });
}

/// Peak node depth is *exactly affine* in `max_depth` — `slope × md +
/// intercept`, with both terms identical at every `max_depth`.
///
/// This is the property that separates a genuine constant from an
/// input-controlled multiplier, and it is stronger than checking a
/// ratio: a Θ(`max_depth`²) bug like #21's first fix has a slope that
/// grows with `max_depth`, so the intercepts would not agree.
///
/// `slope` is the number of nodes the grammar stacks between one charge
/// and the next; `intercept` is the uncharged prologue above the first
/// charge plus the leaf below the last. Re-measure both if the grammar
/// gains a wrapper — a failure here is a request to re-derive
/// [`depth_limit`], not to widen it blindly.
///
/// Depth peaks a level or two either side of `max_depth` and collapses
/// past it (once the cap fires mid-parse, recovery desynchronises the
/// trailing tokens and the extra markers are abandoned), so a window is
/// scanned rather than a single level count.
#[test]
fn node_depth_is_affine_in_max_depth() {
    type Shape = (&'static str, fn(usize) -> String, isize, isize);
    let shapes: [Shape; 10] = [
        ("tuple pattern", |n| const_pattern(n, "[", "]"), 2, 1),
        ("param pattern", param_pattern, 2, 4),
        ("module nesting", |n| modules(n, false, ""), 1, 2),
        (
            "module with a body",
            |n| modules(n, false, "circuit f() : Field { return a+b*(c-d); }"),
            1,
            3,
        ),
        (
            "version parens",
            |n| pragma_version(n, "(", ")", ">= 0.23"),
            1,
            3,
        ),
        (
            "version or/and",
            |n| pragma_version(n, "(", "&&2||3&&4)", "0.1"),
            3,
            3,
        ),
        // Issue #21's worst expression shape, re-pinned here so a change
        // to the shared counter shows up against both grammars.
        (
            "paren + seq + assign",
            |n| composed(n, 0, "(", "=y,z)", "", ""),
            3,
            -1,
        ),
        // The type grammar's own three-wrapper stack, `TYPE_REF →
        // GENERIC_ARG_LIST → GENERIC_ARG`. Nothing in this PR changes
        // `types.rs`, but this is the family that sets the *global*
        // worst case, so it belongs in the instrument that guards it.
        // Return position pays three wrappers per charge; parameter,
        // struct-field and contract-member positions add one more
        // uncharged node above the first charge and are the worst
        // shapes the parser accepts at any setting.
        ("generic type, return position", generic_return_type, 3, 3),
        ("generic type, param position", generic_param_type, 3, 4),
        (
            "generic type, struct field",
            |n| format!("struct S {{ a: {}; }}", nested(n, "A<", ">", "B")),
            3,
            4,
        ),
    ];

    for (what, build, slope, intercept) in shapes {
        for max_depth in [32u32, 64, 128, 256] {
            let mut peak = 0usize;
            for levels in (max_depth as usize).saturating_sub(6)..=(max_depth as usize + 3) {
                let src = build(levels);
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

            let expected = slope * max_depth as isize + intercept;
            assert_eq!(
                peak as isize, expected,
                "{what} @ max_depth {max_depth}: peak node depth {peak} is not \
                 {slope} x {max_depth} + {intercept} = {expected}. The per-charge \
                 wrapper stack changed; re-measure it and re-derive depth_limit \
                 rather than widening the bound."
            );
        }
    }
}

/// Each of the three grammars announces its cap rather than silently
/// truncating, and the CST still round-trips byte-for-byte on a run that
/// produces hundreds of diagnostics.
#[test]
fn depth_bounded_pattern_module_version_report_diagnostics() {
    for (what, src, needle) in [
        (
            "pattern",
            const_pattern(1024, "[", "]"),
            "pattern nesting depth limit",
        ),
        (
            "module",
            modules(1024, false, ""),
            "module nesting depth limit",
        ),
        (
            "version",
            pragma_version(1024, "(", ")", ">= 0.23"),
            "version expression nesting depth limit",
        ),
    ] {
        let result = parse(&src);
        let messages: Vec<String> = result.errors.iter().map(|e| e.message.clone()).collect();
        let root = bounded_root(result, 256, what);
        assert_eq!(
            root.text().to_string(),
            src,
            "{what}: CST must round-trip to the input byte-for-byte"
        );
        assert!(
            messages.iter().any(|m| m.contains(needle)),
            "{what}: expected a diagnostic containing {needle:?}, got: {:?}",
            &messages[..messages.len().min(5)]
        );
    }
}

/// Ordinary Compact using all three grammars must parse cleanly and stay
/// far below the limit — the caps may not turn realistic source into
/// `ERROR` nodes.
#[test]
fn realistic_pattern_module_version_nesting_is_unaffected() {
    let src = concat!(
        "pragma language_version (>= 0.23) && (< 1.0);\n",
        "module Utils {\n",
        "  export module Inner {\n",
        "    export circuit g(x: Field): Field { return x; }\n",
        "  }\n",
        "}\n",
        "circuit f([a, [b, c]]: [Field, [Field, Field]]): Field {\n",
        "  const {p, q: [r, s]} = t;\n",
        "  return a + b + c + p + r + s;\n",
        "}\n"
    );
    let result = parse(src);
    let messages: Vec<String> = result.errors.iter().map(|e| e.message.clone()).collect();
    let root = bounded_root(result, 256, "realistic pattern/module/version source");
    assert!(
        messages.is_empty(),
        "realistic source must parse cleanly: {messages:?}"
    );
    assert!(
        max_node_depth(&root) < 32,
        "realistic source must stay far below the default limit"
    );
}

/// Number of nesting levels used by the #23 small-stack tests.
///
/// Chosen so an *unfixed* parser both (a) returns a tree deeper than
/// [`depth_limit`] — 2,405 / 1,201 / 1,204 nodes for the three shapes
/// against a 1,040 bound — and (b) still completes the parse inside the
/// 2 MiB thread, needing 1,118 / 575 / 895 KiB in a debug build. Without
/// (b) a regression would overflow during the parse and abort the test
/// process instead of reporting a depth, which is the failure mode this
/// whole file is written to avoid.
const NEST_LEVELS: usize = 1200;

/// Issue #23, pattern shape: 2.4 KB of input, CST depth 2,405 unfixed.
#[test]
fn deep_pattern_drops_cleanly_on_a_2mib_stack_thread() {
    parse_and_drop_on_small_stack(const_pattern(NEST_LEVELS, "[", "]"), "nested pattern");
}

/// Issue #23, module shape: CST depth 1,201 unfixed, and unbounded at
/// every `max_depth`.
#[test]
fn deep_module_nesting_drops_cleanly_on_a_2mib_stack_thread() {
    parse_and_drop_on_small_stack(modules(NEST_LEVELS, false, ""), "nested modules");
}

/// Issue #23, version shape: the one that reaches through the CLI on a
/// single `pragma` line. CST depth 1,204 unfixed.
#[test]
fn deep_version_nesting_drops_cleanly_on_a_2mib_stack_thread() {
    parse_and_drop_on_small_stack(
        pragma_version(NEST_LEVELS, "(", ")", ">= 0.23"),
        "nested version term",
    );
}
