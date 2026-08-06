//! Every input, at every `max_depth`, must round-trip byte-for-byte and
//! parse the same way twice — and produce a tree that is *provably the
//! same tree* as the one a previous revision produced.
//!
//! The first two properties are asserted here. The third cannot be, in
//! one checkout: it needs a second revision to compare against. So this
//! file is two things.
//!
//! # The always-on invariants
//!
//! [`losslessness_and_determinism_hold_everywhere`] walks every input
//! this crate can reach — `tests/corpus`, both fuzz corpora, the CLI
//! fixtures — plus a generated set (see [`generated`]) at `max_depth` 8,
//! 32, 64, 128, 256 and 512, and asserts that the CST round-trips to the
//! input byte-for-byte and that a second parse of the same bytes gives
//! the same tree and the same diagnostics. Those are CONSTITUTION.md
//! principle III, in the two forms a parser can violate them.
//!
//! # The differential instrument
//!
//! [`print_fingerprints`] is `#[ignore]`d because it asserts nothing. It
//! prints one line per (input, `max_depth`) pair — a structural
//! fingerprint over node kinds, token kinds and token text in preorder,
//! plus node depth, diagnostic count and a diagnostic hash. Diff two
//! revisions to prove a change moved no tree:
//!
//! ```text
//! git worktree add ../base origin/main
//! (cd ../base && cargo test -p compactp_parser --test tree_equivalence \
//!      -- --ignored --nocapture print_fingerprints) > /tmp/base.txt
//! cargo test -p compactp_parser --test tree_equivalence \
//!      -- --ignored --nocapture print_fingerprints > /tmp/head.txt
//! diff /tmp/base.txt /tmp/head.txt
//! ```
//!
//! This is the evidence that carried issue #22: swapping tree
//! construction off `rowan::GreenNodeBuilder` is only safe if it builds
//! the identical tree, and no unit test can show that. It is committed
//! because the argument is not reproducible without it.
//!
//! # Why the generated inputs matter more than the corpus
//!
//! The committed corpus is the wrong instrument for this class on its
//! own: its median file is under a kilobyte and it contains almost no
//! operator chains. The shapes that discriminate here are the ones
//! [`generated`] builds — chains that straddle the interner's
//! three-child boundary, chains that overrun the depth cap, and lambda
//! and generic-type shapes that straddle `looks_like_lambda`'s
//! hundred-token cutoff. Add to that set when a change touches tree
//! construction or lookahead.

use compactp_parser::{ParseOptions, ParseResult, parse_with};
use compactp_syntax::SyntaxNode;
use rowan::{NodeOrToken, WalkEvent};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// The settings every input is checked at. Spans the documented
/// default (256) and both sides of it.
const DEPTHS: [u32; 6] = [8, 32, 64, 128, 256, 512];

const PRE: &str =
    "pragma language_version >= 0.23;\nexport pure circuit f(x: Field): Field { return ";
const POST: &str = "; }\n";

/// Repository root, from this crate's manifest directory.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/<crate>/ is two levels below the repo root")
        .to_path_buf()
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    // Sorted so the fingerprint listing is stable across filesystems.
    paths.sort();
    for path in paths {
        if path.is_dir() {
            collect(&path, out);
        } else if path.is_file() {
            out.push(path);
        }
    }
}

/// Every input file in the repository, as `(label, source)`.
///
/// Fuzz inputs are arbitrary bytes, so they are read lossily — the
/// parser's contract is over `&str`, and the fuzz target does the same.
fn repo_inputs() -> Vec<(String, String)> {
    let root = repo_root();
    let mut paths = Vec::new();
    for dir in ["tests/corpus", "fuzz/corpus", "crates/compactp/tests"] {
        collect(&root.join(dir), &mut paths);
    }
    paths
        .into_iter()
        .filter_map(|path| {
            let bytes = std::fs::read(&path).ok()?;
            let label = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .display()
                .to_string();
            Some((label, String::from_utf8_lossy(&bytes).into_owned()))
        })
        .collect()
}

/// Shapes chosen to sit on the boundaries this crate's tree
/// construction and lookahead actually turn on, which the committed
/// corpus does not exercise.
fn generated() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut push = |label: String, body: String| out.push((label, format!("{PRE}{body}{POST}")));

    // Chains either side of the interner's three-child threshold. The
    // tight forms give the spine node exactly three children and so are
    // interned; the spaced forms give it four and are not. Lengths
    // straddle the depth cap at every setting in DEPTHS.
    for links in [7usize, 31, 63, 127, 255, 511, 600] {
        for (what, sep, tail) in [
            ("infix-tight", "+", "x"),
            ("infix-spaced", " + ", "x"),
            ("member", "", ".a"),
            ("index", "", "[0]"),
            ("call", "", "()"),
            ("call-from-literal", "", "()"),
            ("cast", " as Field", ""),
            ("ternary", "", "?1:x"),
            ("mixed-postfix", "", ".a[0]()+x"),
        ] {
            let mut body = String::from(if what == "call-from-literal" {
                "1"
            } else {
                "x"
            });
            for _ in 0..links {
                body.push_str(sep);
                body.push_str(tail);
            }
            push(format!("gen:{what}:{links}"), body);
        }
    }

    // Nesting, which reaches the depth charge through a different route
    // than chaining, plus the composed form that separates a
    // height-aware charge from a path-only one.
    for depth in [7usize, 63, 255, 300] {
        push(
            format!("gen:paren-nest:{depth}"),
            format!("{}x{}", "(".repeat(depth), ")".repeat(depth)),
        );
        push(
            format!("gen:paren-nest-ws:{depth}"),
            format!("{}x{}", "( ".repeat(depth), " )".repeat(depth)),
        );
        push(
            format!("gen:bracket-nest:{depth}"),
            format!("{}x{}", "[".repeat(depth), "]".repeat(depth)),
        );
    }

    // `looks_like_lambda` reads a hundred-token window and gives up past
    // it, so its behaviour changes at that cutoff. Parameter counts and
    // generic-argument nesting depths straddle it from both directions.
    for params in [0usize, 1, 2, 48, 49, 50, 51, 52, 120] {
        let list = (0..params)
            .map(|i| format!("p{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        push(
            format!("gen:lambda-params:{params}"),
            format!("({list}) => 1"),
        );
        push(
            format!("gen:lambda-params-typed:{params}"),
            format!("({list}): Field => 1"),
        );
    }
    for nest in [1usize, 30, 31, 32, 33, 34, 60] {
        let ty = format!("{}Field{}", "A<".repeat(nest), ">".repeat(nest));
        push(
            format!("gen:lambda-generic:{nest}"),
            format!("(a: {ty}) => a"),
        );
        push(format!("gen:paren-generic:{nest}"), format!("(a as {ty})"));
    }

    // Malformed shapes, so the equivalence claim covers the recovery
    // paths and not only clean input.
    for (what, body) in [
        ("unclosed-paren", "(((x"),
        ("unclosed-bracket", "[[[x"),
        ("stray-close", "x)))"),
        ("dangling-op", "x+++"),
        ("empty-ternary", "x?:"),
        ("fat-arrow-only", "=> x"),
        ("comma-run", "x,,,,x"),
    ] {
        push(format!("gen:malformed:{what}"), body.to_string());
    }

    out
}

fn all_inputs() -> Vec<(String, String)> {
    let mut inputs = repo_inputs();
    inputs.extend(generated());
    inputs
}

/// What a parse produced, gathered without recursing in tree depth.
struct Parsed {
    fingerprint: u64,
    depth: usize,
    text: String,
    diagnostics: Vec<String>,
}

/// Structural fingerprint: node kinds, token kinds and token text in
/// preorder, with node open/close bracketing so shape is captured as
/// well as content.
///
/// Walked with rowan's cursor iterator rather than recursively: at
/// `max_depth` 512 an accepted tree can be two thousand levels deep, and
/// a recursive walk would abort the test process rather than report.
fn parse_at(src: &str, max_depth: u32) -> Parsed {
    let ParseResult { green, errors } = parse_with(
        src,
        ParseOptions {
            max_depth,
            ..Default::default()
        },
    );
    let diagnostics = errors.iter().map(|d| format!("{d:?}")).collect();
    let root = SyntaxNode::new_root(green);

    let mut hasher = DefaultHasher::new();
    let mut depth = 0usize;
    let mut max = 0usize;
    for event in root.preorder_with_tokens() {
        match event {
            WalkEvent::Enter(NodeOrToken::Node(node)) => {
                depth += 1;
                max = max.max(depth);
                0u8.hash(&mut hasher);
                format!("{:?}", node.kind()).hash(&mut hasher);
            }
            WalkEvent::Enter(NodeOrToken::Token(token)) => {
                1u8.hash(&mut hasher);
                format!("{:?}", token.kind()).hash(&mut hasher);
                token.text().hash(&mut hasher);
            }
            WalkEvent::Leave(NodeOrToken::Node(_)) => {
                depth = depth.saturating_sub(1);
                2u8.hash(&mut hasher);
            }
            WalkEvent::Leave(NodeOrToken::Token(_)) => {}
        }
    }
    let text = root.text().to_string();

    // Every setting here is bounded, so the tree is at most a couple of
    // thousand levels and drops safely. Guard it anyway: if a future
    // change breaks the depth bound, leak rather than abort, so the
    // failure is reportable.
    if max > depth_ceiling(max_depth) {
        std::mem::forget(root);
    }

    Parsed {
        fingerprint: hasher.finish(),
        depth: max,
        text,
        diagnostics,
    }
}

/// Depth past which a tree is unsafe to drop here. Deliberately looser
/// than `tests/depth_bound.rs`'s bound — this file does not police the
/// depth invariant, it only avoids aborting on a violation of it.
fn depth_ceiling(max_depth: u32) -> usize {
    8 * max_depth as usize + 64
}

/// The CST covers every byte of its input, and parsing is a function of
/// the input alone — at every `max_depth`, for every input the
/// repository can reach plus the generated boundary shapes.
#[test]
fn losslessness_and_determinism_hold_everywhere() {
    let inputs = all_inputs();
    assert!(
        inputs.len() > 500,
        "expected the repository's corpora to be present, found {} inputs",
        inputs.len()
    );

    for (label, src) in &inputs {
        for max_depth in DEPTHS {
            let first = parse_at(src, max_depth);
            assert_eq!(
                first.text.len(),
                src.len(),
                "{label} @ max_depth {max_depth}: CST text is {} bytes for a \
                 {} byte input",
                first.text.len(),
                src.len()
            );
            assert!(
                first.text == *src,
                "{label} @ max_depth {max_depth}: CST must round-trip \
                 byte-for-byte"
            );

            let second = parse_at(src, max_depth);
            assert_eq!(
                first.fingerprint, second.fingerprint,
                "{label} @ max_depth {max_depth}: two parses of the same bytes \
                 produced different trees"
            );
            assert_eq!(
                first.diagnostics, second.diagnostics,
                "{label} @ max_depth {max_depth}: two parses of the same bytes \
                 produced different diagnostics"
            );
        }
    }
}

/// Prints the fingerprint table for cross-revision diffing. Asserts
/// nothing; see the module docs for how to use it.
#[test]
#[ignore = "instrument, not an assertion: diff its output across two revisions"]
fn print_fingerprints() {
    for (label, src) in all_inputs() {
        for max_depth in DEPTHS {
            let parsed = parse_at(&src, max_depth);
            let mut diag_hash = DefaultHasher::new();
            parsed.diagnostics.hash(&mut diag_hash);
            println!(
                "{label}\tmd={max_depth}\tfp={:016x}\tdepth={}\terrs={}\tdiag={:016x}\tlossless={}",
                parsed.fingerprint,
                parsed.depth,
                parsed.diagnostics.len(),
                diag_hash.finish(),
                parsed.text == src,
            );
        }
    }
}
