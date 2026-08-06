//! `ParseOptions::max_errors` is a *budget*, not a target.
//!
//! The parser's list-parsing loops — declarations at file scope,
//! statements in a block, members of a `module` or `contract` — recover
//! by calling a production and looping. They were bounded only by
//! `errors_exhausted()`, which bounds the diagnostic budget but not the
//! loop: a production that reported an error and consumed nothing left
//! the loop to run again on the same token, emitting diagnostics until
//! it reached `max_errors`.
//!
//! That makes the diagnostic count a function of the *option* rather
//! than of the input, which turns a documented tuning knob into a memory
//! and CPU amplifier. Raising `max_errors` is exactly what a language
//! server or a batch linter does. Measured before the fix: `export` —
//! six bytes — produced `max_errors` diagnostics; a 49-byte circuit with
//! a deeply nested destructuring pattern produced 20,002 at
//! `max_errors = 20_000`, and 5,000,002 (>600 MiB of message strings) at
//! five million.
//!
//! The invariant asserted here is deliberately simple and total: **for a
//! fixed input, raising `max_errors` past the point where the parser
//! stops reporting must not change the diagnostic count.** Any loop that
//! can emit without consuming violates it, whichever grammar the input
//! happens to exercise.
//!
//! Five of these inputs already amplified on `main` before issue #23 was
//! addressed; the pattern ones were reachable only once `pattern()`
//! started enforcing `max_depth`. Both kinds are kept here — the fix is
//! one progress guard shared by every affected loop, so the test covers
//! the guard rather than the inputs that found it.

use compactp_parser::{ParseOptions, parse_with};
use compactp_syntax::SyntaxNode;

/// Budgets far above what any input here legitimately needs. If the
/// count differs between them, diagnostics are being produced by the
/// budget rather than by the source.
const BUDGETS: [usize; 2] = [100_000, 1_000_000];

fn parse_at(src: &str, max_errors: usize, max_depth: u32) -> (usize, String) {
    let result = parse_with(
        src,
        ParseOptions {
            recover: true,
            max_errors,
            max_depth,
        },
    );
    let count = result.errors.len();
    let root = SyntaxNode::new_root(result.green);
    let text = root.text().to_string();
    (count, text)
}

/// Assert the diagnostic count is independent of `max_errors`, and that
/// the CST still round-trips byte-for-byte.
fn assert_budget_independent(what: &str, src: &str, max_depth: u32) {
    let mut baseline: Option<usize> = None;
    for max_errors in BUDGETS {
        let (count, text) = parse_at(src, max_errors, max_depth);
        assert_eq!(
            text, src,
            "{what}: CST must round-trip byte-for-byte (max_errors {max_errors})"
        );
        match baseline {
            None => baseline = Some(count),
            Some(first) => assert_eq!(
                count,
                first,
                "{what}: diagnostic count tracks max_errors ({first} at {}, {count} at \
                 {max_errors}) instead of the input. A recovery loop is emitting without \
                 consuming a token, so {} bytes of source can be amplified to max_errors \
                 diagnostics.",
                BUDGETS[0],
                src.len()
            ),
        }
    }
}

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

/// Inputs that stall a production without consuming a token. Each one
/// amplified to `max_errors` diagnostics before the progress guard.
#[test]
fn diagnostic_count_does_not_track_max_errors() {
    // Reachable on `main` before issue #23: a token no statement or
    // declaration production consumes, in a list position.
    assert_budget_independent(
        "stray `]` in statement position",
        "circuit f(): Field { ] }",
        256,
    );
    assert_budget_independent(
        "stray `)` in statement position",
        "circuit f(): Field { ) }",
        256,
    );
    assert_budget_independent("bare `export` at file scope", "export", 256);
    assert_budget_independent("`export ;` at file scope", "export ;", 256);
    assert_budget_independent("`export @` at file scope", "export @", 256);
    assert_budget_independent(
        "`export ;` in a contract body",
        "contract C { export ; }",
        256,
    );
    assert_budget_independent("`export ;` in a module body", "module M { export ; }", 256);

    // Reachable only once `pattern()` enforces `max_depth` (issue #23):
    // the cap fires mid-pattern and strands the unmatched delimiter.
    for max_depth in [8u32, 256] {
        let levels = max_depth as usize + 4;
        assert_budget_independent(
            "over-deep tuple pattern",
            &format!(
                "circuit f(): Field {{ const {} = x; }}",
                nested(levels, "[", "]", "a")
            ),
            max_depth,
        );
        assert_budget_independent(
            "over-deep struct pattern",
            &format!(
                "circuit f(): Field {{ const {} = x; }}",
                nested(levels, "{a:", "}", "a")
            ),
            max_depth,
        );
        assert_budget_independent(
            "over-deep module nesting",
            &format!("{}{}", "module M{".repeat(levels), "}".repeat(levels)),
            max_depth,
        );
        assert_budget_independent(
            "over-deep version term",
            &format!(
                "pragma language_version {}>= 0.23{};",
                "(".repeat(levels),
                ")".repeat(levels)
            ),
            max_depth,
        );
    }
}

/// The count must grow with the source, not stay pinned to the budget.
///
/// Budget independence alone would also be satisfied by a parser that
/// reported a fixed number of diagnostics regardless of input. This
/// pins the other direction: doubling the malformed input roughly
/// doubles the diagnostics, and a tenfold increase in input never
/// produces fewer.
#[test]
fn diagnostic_count_scales_with_input_size() {
    let counts: Vec<(usize, usize)> = [1usize, 10, 100]
        .iter()
        .map(|&n| {
            let src = "circuit f(): Field { ] }".repeat(n);
            let (count, text) = parse_at(&src, 1_000_000, 256);
            assert_eq!(text, src, "CST must round-trip byte-for-byte");
            (src.len(), count)
        })
        .collect();

    for pair in counts.windows(2) {
        let (small_bytes, small) = pair[0];
        let (big_bytes, big) = pair[1];
        assert!(
            big > small,
            "diagnostics did not grow with input: {small} for {small_bytes} bytes, \
             {big} for {big_bytes} bytes"
        );
        // Growth must stay proportional — a superlinear blow-up is the
        // amplification bug wearing a different hat.
        let ratio = big as f64 / small as f64;
        let size_ratio = big_bytes as f64 / small_bytes as f64;
        assert!(
            ratio <= size_ratio * 2.0,
            "diagnostics grew {ratio:.1}x for a {size_ratio:.1}x larger input \
             ({small} -> {big}); recovery is emitting superlinearly"
        );
    }
}

/// Realistic source must be unaffected: the progress guard may not
/// insert `ERROR` nodes into input that parses cleanly.
#[test]
fn valid_source_gains_no_diagnostics() {
    let src = concat!(
        "pragma language_version >= 0.23;\n",
        "module Utils { export circuit g(x: Field): Field { return x; } }\n",
        "struct S { a: Field; b: Field; }\n",
        "contract C { circuit m(): Field; }\n",
        "export pure circuit f(a: Field, [b, c]: [Field, Field]): Field {\n",
        "  const {p, q} = t;\n",
        "  return a + b + c + p + q;\n",
        "}\n"
    );
    for max_errors in [0usize, 1, 256, 1_000_000] {
        let (count, text) = parse_at(src, max_errors, 256);
        assert_eq!(text, src, "CST must round-trip byte-for-byte");
        assert_eq!(
            count, 0,
            "valid source must parse cleanly at max_errors {max_errors}"
        );
    }
}
