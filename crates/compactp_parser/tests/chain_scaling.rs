//! Building the CST must cost time linear in the size of the tree.
//!
//! Regression coverage for issue #22. A left-associative operator chain
//! (`x+x+…+x`) or a postfix chain (`1()()…`, `x.a.a…`) grows a
//! *left-deep spine*: every extension wraps the whole tree built so far,
//! so the spine's subtree sizes run `1, 2, 3, …, n`.
//!
//! `rowan::GreenNodeBuilder`, which this crate used until #22, interns
//! nodes in a hash table whose rehash callback recomputes a node's hash
//! by walking its entire subtree. Growing that table therefore cost the
//! sum of all cached subtree sizes — `O(n log n)` for a balanced tree,
//! but `Θ(n²)` for a spine. Measured on `main` at f5015d0, release,
//! `aarch64-apple-darwin`, `max_depth = 100_000`: 5,000 terms 105 ms,
//! 10,000 terms 405 ms, 20,000 terms 1,606 ms — quadrupling for each
//! doubling, with ~90% of samples inside `rowan`'s `node_hash`.
//!
//! # Why this reads a clock
//!
//! The cost was pure hashing: same tree, same node count, same event
//! stream, same allocations. There is no counter the parser could
//! expose that would have caught it, because the work happened inside a
//! dependency.
//!
//! What makes the timing here stable is that it is *differential*. Both
//! shapes below build a spine of the same length, do the same number of
//! `precede` calls and produce a tree of the same depth. They differ in
//! one respect: `x+x+…+x` gives every `BINARY_EXPR` exactly three
//! children, which is narrow enough to intern, while `x + x + … + x`
//! puts whitespace either side of the operator and so gives it five,
//! which is not. Only the first shape ever reached the quadratic path.
//!
//! So the ratio between them is the measurement, and it cancels out
//! machine speed, build profile and background load — the two parses run
//! back to back on the same machine. On `main` that ratio was ~493 in a
//! debug build and ~670 in release. It is now ~0.9: the interned shape
//! is *cheaper*, having fewer tokens. The threshold is set at 10, which
//! leaves an order of magnitude of headroom above the fixed behaviour
//! and a further factor of ~50 below the broken one.
//!
//! `max_depth` is raised throughout. At the default, the left-spine
//! depth charge truncates these chains after a few hundred extensions
//! and emits diagnostics, so every shape here looks fast and linear no
//! matter how tree construction behaves — which is precisely why the
//! quadratic survived the depth work in #21/#23/#28. The documentation
//! on `ParseOptions::max_depth` invites raising it, so this is a
//! supported configuration, not a contrived one.
//!
//! Note on test hygiene: at these settings the CST is tens of thousands
//! of levels deep and rowan drops a tree recursively, so dropping one
//! here would abort the test process instead of reporting a failure.
//! Every tree below is deliberately leaked, and every check runs before
//! the assertion that could panic.

use compactp_parser::{ParseOptions, ParseResult, parse_with};
use compactp_syntax::SyntaxNode;
use std::time::{Duration, Instant};

const PRE: &str =
    "pragma language_version >= 0.23;\nexport pure circuit f(x: Field): Field { return ";
const POST: &str = "; }\n";

/// Chain length. Long enough that a quadratic term dwarfs everything
/// else, short enough that the fixed parser handles it in tens of
/// milliseconds in a debug build.
const TERMS: usize = 20_000;

/// Ratio of interned-spine time to un-interned-spine time above which
/// the test fails. See the module docs for how it was chosen.
const MAX_RATIO: f64 = 10.0;

/// Room to raise `max_depth` well past any chain used here, so the
/// depth charge never truncates the spine.
const DEEP: u32 = 1_000_000;

fn opts() -> ParseOptions {
    ParseOptions {
        max_depth: DEEP,
        ..Default::default()
    }
}

/// `x`, extended `n - 1` times by `sep` then `tail`.
fn chained(n: usize, sep: &str, tail: &str) -> String {
    let mut body = String::from("x");
    for _ in 1..n {
        body.push_str(sep);
        body.push_str(tail);
    }
    format!("{PRE}{body}{POST}")
}

/// Facts about a parse, gathered before the tree is leaked.
struct Parsed {
    text: String,
    diagnostics: Vec<String>,
}

/// Parse `src`, extract everything the tests need, then leak the tree.
///
/// The tree is read through a [`SyntaxNode`], whose text walk is
/// cursor-based and iterative; `GreenNode`'s own `Display` recurses in
/// tree depth and would overflow the stack on these inputs.
fn parse_and_leak(src: &str, max_depth: u32) -> Parsed {
    let ParseResult { green, errors } = parse_with(
        src,
        ParseOptions {
            max_depth,
            ..Default::default()
        },
    );
    let diagnostics = errors.iter().map(|d| format!("{d:?}")).collect();
    let root = SyntaxNode::new_root(green);
    let text = root.text().to_string();
    std::mem::forget(root);

    Parsed { text, diagnostics }
}

/// Time one parse, leaking the tree without walking it.
fn time_parse(src: &str) -> Duration {
    let start = Instant::now();
    let result = parse_with(src, opts());
    let elapsed = start.elapsed();
    std::mem::forget(result);
    elapsed
}

/// Best of three, to damp a single scheduling hiccup on a loaded runner.
fn best_of_three(src: &str) -> Duration {
    (0..3).map(|_| time_parse(src)).min().unwrap_or_default()
}

/// Bytes per second, the comparable quantity when two shapes differ in
/// size.
fn rate(src: &str, elapsed: Duration) -> f64 {
    src.len() as f64 / elapsed.as_secs_f64()
}

/// The headline regression. An interned left-deep spine must not cost
/// dramatically more than an un-interned one of the same length.
#[test]
fn interning_a_left_deep_spine_is_not_quadratic() {
    let interned = chained(TERMS, "+", "x");
    let uninterned = chained(TERMS, " + ", "x");

    // Interleaved, so a slow patch of the runner hits both alike.
    let a = best_of_three(&interned);
    let b = best_of_three(&uninterned);
    let a = a.min(best_of_three(&interned));
    let b = b.min(best_of_three(&uninterned));

    let ratio = a.as_secs_f64() / b.as_secs_f64();
    assert!(
        ratio < MAX_RATIO,
        "interned spine of {TERMS} terms took {a:?} against {b:?} for the \
         un-interned spine (ratio {ratio:.1}, limit {MAX_RATIO}); tree \
         construction has gone superlinear in chain length again"
    );
}

/// Every spine shape must round-trip byte-for-byte and parse cleanly at
/// a raised `max_depth`. The interner decides sharing per node kind and
/// child count, and these are the shapes where those decisions differ,
/// so this pins that none of them loses or reorders a byte.
#[test]
fn long_chains_round_trip_losslessly() {
    for (what, sep, tail) in [
        ("infix chain, 3-child nodes", "+", "x"),
        ("infix chain, 5-child nodes", " + ", "x"),
        ("member chain, 3-child nodes", "", ".a"),
        ("index chain, 4-child nodes", "", "[0]"),
        ("call chain", "", "()"),
        ("cast chain", " as Field", ""),
        ("mixed postfix chain", "", ".a[0]()+x"),
    ] {
        let src = chained(TERMS / 4, sep, tail);
        let parsed = parse_and_leak(&src, DEEP);
        assert_eq!(
            parsed.text, src,
            "{what}: CST must round-trip byte-for-byte"
        );
        assert!(
            parsed.diagnostics.is_empty(),
            "{what}: valid input must parse cleanly, got {:?}",
            parsed.diagnostics
        );
    }
}

/// A chain long enough to exhaust the raised budget still round-trips.
///
/// This is the interned spine meeting the depth cap: the parser stops
/// extending, wraps the tail in `ERROR` nodes and hands back a tree
/// whose spine is partly interned and partly not. Losslessness must
/// survive that seam.
#[test]
fn a_chain_that_overruns_max_depth_is_still_lossless() {
    let src = chained(TERMS, "+", "x");
    let parsed = parse_and_leak(&src, 512);
    assert_eq!(
        parsed.text, src,
        "CST must round-trip even when the cap fires"
    );
    assert!(
        !parsed.diagnostics.is_empty(),
        "the cap must report that it fired"
    );
}

/// Same input, same tree, same diagnostics — every time.
///
/// The interner is keyed on identities handed out in parse order and
/// hashed with a fixed seed, so nothing about which subtrees get shared
/// may vary between runs (CONSTITUTION.md, principle III).
#[test]
fn chain_parsing_is_deterministic() {
    let src = chained(TERMS / 4, "+", "x");
    let first = parse_and_leak(&src, DEEP);
    let second = parse_and_leak(&src, DEEP);
    assert_eq!(first.text, second.text);
    assert_eq!(first.diagnostics, second.diagnostics);
}

/// Nested parens are the other shape that used to be paced by lookahead
/// rather than by size.
///
/// `looks_like_lambda` has to decide whether a `(` opens a lambda
/// parameter list, and reads up to 100 tokens ahead to find out. It read
/// them with `Parser::nth`, which restarts from the current position on
/// every call, so the window cost ~5,000 token steps per `(` — and more
/// with trivia in it, since `nth` re-skips that too. Nesting pays it per
/// level: `((((…x…))))` parsed at ~0.8 MiB/s against ~92 MiB/s for
/// `tests/corpus`.
///
/// Compared against the *same nesting shape built from brackets*.
/// `[[[…x…]]]` recurses through the grammar identically and builds a
/// comparable tree, but a `[` has no lambda form to rule out and so no
/// lookahead window. Measured at `f5015d0`, debug: the paren shape ran
/// 34.4x slower per byte than the bracket shape at depth 1,000 and
/// 32.9x at depth 2,000. It is now 2.7x at both.
///
/// Runs on its own thread because nested input recurses the parser in
/// step with the nesting — the stack cost documented on
/// `ParseOptions::max_depth`, not a property under test here.
#[test]
fn nested_parens_are_not_paced_by_lookahead() {
    const DEPTH: usize = 1_000;
    const STACK: usize = 32 * 1024 * 1024;

    let measure = || -> Result<(), String> {
        let parens = format!("{PRE}{}x{}{POST}", "( ".repeat(DEPTH), " )".repeat(DEPTH));
        let brackets = format!("{PRE}{}x{}{POST}", "[ ".repeat(DEPTH), " ]".repeat(DEPTH));

        let paren_rate = rate(&parens, best_of_three(&parens));
        let bracket_rate = rate(&brackets, best_of_three(&brackets));
        let ratio = bracket_rate / paren_rate;
        if ratio >= MAX_RATIO {
            return Err(format!(
                "nesting {DEPTH} parens ran at {paren_rate:.0} B/s against \
                 {bracket_rate:.0} B/s for the same nesting in brackets \
                 (ratio {ratio:.1}, limit {MAX_RATIO}); paren lookahead is \
                 re-walking the token stream"
            ));
        }

        let parsed = parse_and_leak(&parens, DEEP);
        if parsed.text != parens {
            return Err("nested parens must round-trip byte-for-byte".to_string());
        }
        if !parsed.diagnostics.is_empty() {
            return Err(format!(
                "nested parens must parse cleanly, got {:?}",
                parsed.diagnostics
            ));
        }
        Ok(())
    };

    let outcome = std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(measure)
        .expect("spawn measurement thread")
        .join();

    match outcome {
        Ok(Ok(())) => {}
        Ok(Err(message)) => panic!("{message}"),
        Err(_) => panic!("measurement thread panicked"),
    }
}
