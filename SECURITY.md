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
(`expr_bp`, `ty`, `stmt`, `block`) *and* per left-spine extension
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
the same budget and compound into a Θ(`max_depth`²) tree. Measured over
flat chains, pure nesting, and composed shapes, the worst returned node
depth is `max_depth + 4`.

Bounding the *tree* — not just the parser — is what matters for
consumers: rowan drops a tree recursively, and most CST walks recurse
in the tree's depth, so an unbounded tree could abort a consumer's
process with `SIGABRT` when the tree was merely dropped.

`max_depth` is therefore a stack budget for both the parser and the
consumer. At the default, parsing and dropping the deepest accepted
input needs roughly 1 MiB of stack in debug and 200 KiB in release,
inside a 2 MiB thread. Raising it scales both linearly: on a 2 MiB
stack the parser itself overflows at `max_depth` around 1024 in debug
and 4096 in release. Raise it only alongside a thread stack sized to
match.

Known gap: the pattern, module, and version grammars do not yet charge
the counter, so deeply nested destructuring patterns
(`const [[[...x...]]] = y;`) remain unbounded. Tracked as
[#23](https://github.com/devrelaicom/compactp/issues/23).

## Response Expectations

- Initial triage: within 3 business days
- Status update after reproduction: within 7 business days
- Remediation timeline: depends on severity and release coordination needs

If the report turns out to be an upstream Compact language/compiler issue instead of a `compactp` parser issue, we may coordinate disclosure or redirect you to the upstream project.
