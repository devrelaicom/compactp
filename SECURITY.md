# Security Policy

## Supported Versions

The latest released version of `compactp` is supported for security fixes. If no release has been published yet, the `main` branch is the supported line.

## Reporting a Vulnerability

Please do not open public GitHub issues for security-sensitive reports.

Use one of these private channels instead:

- GitHub Security Advisories for this repository
- Email: aaronbassett@gmail.com

Include:

- the `compactp` version or commit SHA
- the exact `.compact` input or the smallest reproducer you can provide
- the command you ran
- the observed behavior and why you believe it is security-relevant

## What to Report

`compactp` processes untrusted input. Please report any issue that can be triggered by malformed or adversarial source, including:

- panics
- hangs or non-terminating parses
- excessive CPU usage
- excessive memory usage
- stack overflows
- crashes in CLI or library mode
- parser differentials that could cause unsafe downstream tooling behavior

See [Reporting a Vulnerability](#reporting-a-vulnerability) above for how to submit.

## What counts as a security bug

compactp is a parser frontend that consumes untrusted user input. The
following are treated as security bugs and get prioritized:

- **Parser panic on any input.** The parser must never `panic!` on
  arbitrary UTF-8 input. Untrusted input that triggers a panic is a
  security bug.
- **Out-of-memory / runaway parse.** Input that causes unbounded
  memory growth or non-terminating parse is a security bug.
- **Stack overflow.** Input that causes a stack overflow (despite the
  bounded-recursion guard documented below) is a security bug.

Logic bugs in the parser (e.g., accepting input the upstream compiler
rejects, or rejecting input the upstream compiler accepts) are normal
bugs, not security bugs.

## Fuzz methodology

compactp ships two cargo-fuzz harnesses targeting the lexer and the
parser. Both feed arbitrary UTF-8 bytes to the public entry points
and assert:

- The function returns without panicking.
- For the parser specifically: when the parse succeeds with no errors,
  the CST's text content round-trips back to the input byte-for-byte
  (the lossless contract).

The harnesses live under `fuzz/fuzz_targets/`. Seed corpora live under
`fuzz/corpus/<target>/` and are minimized via `cargo fuzz cmin`.

A nightly CI job (`.github/workflows/fuzz-nightly.yml`) runs each
target for 10 minutes at 04:00 UTC and opens a labeled GitHub issue
on any crash.

For longer local sessions: `scripts/fuzz.sh --target <T> --duration
<minutes>` (see CONTRIBUTING.md for details).

## Bounded-depth guarantee

`ParseOptions::max_depth` (default `256`) bounds the depth of the CST
the parser returns, not only the depth of the parser's own recursion.
The counter is charged on entry to each recursive grammar function
(`expr_bp`, `ty`, `stmt`, `block`, `pattern`, `module_def`,
`version_term`) *and* per left-spine extension
inside the expression Pratt loop: left-associative operator chains
(`x+x+...+x`) and postfix chains (`f()()...`, `x[0][0]...`,
`x.a.a...`, `x as T as T...`) deepen the tree inside a single stack
frame, so charging entry alone would not bound them. On overflow, the
parser emits a recovery diagnostic and produces an `ERROR` node; the
CST stays lossless.

The spine charge is *height*-aware: it counts the height of the subtree
being wrapped, not one step along the current path. A path-only charge
is not sufficient — the left-hand side is parsed before the loop holds
any charge and releases its own charges on the way out, so nesting and
chaining together (`((( x +x+x… ) +x+x… ) +x+x… )`) would each re-spend
the same budget and compound into a Θ(`max_depth`²) tree.

Node depth is a small constant *multiple* of `max_depth`, because a
charge can sit beneath up to three uncharged wrapper nodes — either
`PAREN_EXPR` → `EXPR_SEQ` → `ASSIGN_EXPR` (for input shaped like
`( … =y,z )`) or `VERSION_PAREN_EXPR` → `VERSION_OR_EXPR` →
`VERSION_AND_EXPR` (for `( … &&2||3&&4 )`). Measured over flat chains,
pure nesting, element-position wrappers, composed
nesting-plus-chaining, nested destructuring patterns, nested modules and
nested version terms — every level count up to `4 × max_depth`, at
`max_depth` 8, 32, 64, 128, 256 and 512 — the worst returned node depth
is `3 × max_depth + 3`, 771 at the default. Peak depth is *exactly*
affine in `max_depth` at every one of those settings, so the multiplier
is a genuine constant and not something the input can drive.

Bounding the *tree* — not just the parser — is what matters for
consumers: rowan drops a tree recursively, and most CST walks recurse
in the tree's depth, so an unbounded tree could abort a consumer's
process with `SIGABRT` when the tree was merely dropped. For the
pattern, module and version grammars the parser's *own* recursion is the
binding constraint rather than the drop: with the tree leaked instead of
dropped, the overflow threshold is unchanged.

`max_depth` is therefore a stack budget for both the parser and the
consumer. Measured against the worst-case accepted shapes above on
`aarch64-apple-darwin`: at the default, parsing and dropping one needs
under 1 MiB of stack in debug and under 256 KiB in release, inside a
2 MiB thread. Raising the limit scales both linearly: on a 2 MiB stack
the deepest setting the parser itself survives is around 640 in debug
and around 3,100 in release. Frame sizes vary by target and optimization
level, so treat those as order-of-magnitude figures; raise the limit only
alongside a thread stack sized to match.

Known gap: nested `contract` declarations are still not charged.
`declarations::contract` recurses through `declarations::declaration`
back into itself, so `contract A { contract B { … } }` is unbounded at
every `max_depth` — `max_depth = 1` parses 2001 levels with zero
diagnostics — and overflows the parser's own stack at around 37,500
levels in a release build on an 8 MiB main thread. This is the same
class of bug as the pattern, module and version gaps fixed above and
needs the same fix; it is not yet tracked by an issue. Everything above
holds for input that does not nest `contract` declarations.

## Response Expectations

- Initial triage: within 3 business days
- Status update after reproduction: within 7 business days
- Remediation timeline: depends on severity and release coordination needs

If the report turns out to be an upstream Compact language/compiler issue instead of a `compactp` parser issue, we may coordinate disclosure or redirect you to the upstream project.
