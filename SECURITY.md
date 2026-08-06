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
`version_term`, `contract`) *and* per left-spine extension
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

Those eight charges cover the whole grammar, and the argument for that
has two legs, because there are two ways to deepen a tree.

The first is call recursion, and every recursion cycle in the parser
passes through at least one charge. Established statically, by
enumerating the strongly connected components of the grammar's call
graph with the charged functions deleted — the remainder is acyclic —
and empirically, by driving 34 nesting shapes, at least one per
recursive production plus each of the four routes into `contract`,
50,000 levels deep at the default options, each returning a bounded,
lossless tree rather than aborting.

The second is `CompletedMarker::precede`, which inserts a parent above
an already-finished subtree from inside a single stack frame. That
deepens the tree with no call cycle at all, so an SCC argument is
structurally blind to it — and it is exactly what left
[#21](https://github.com/devrelaicom/compactp/issues/21) unbounded.
Its six call sites are all inside the Pratt loop, which is height-charged
per extension. Nothing else deepens a tree: `marker.rs` and `sink.rs`
are iterative, so `grammar/*.rs` is the whole recursion surface.

Together the two legs leave no way to nest without spending the counter,
and so no production that is unbounded at every setting of `max_depth`.

Node depth is a small constant *multiple* of `max_depth`, because a
charge can sit beneath up to three uncharged wrapper nodes. Three
grammars reach that maximum:

- types: `TYPE_REF` → `GENERIC_ARG_LIST` → `GENERIC_ARG` (`A<A<…>>`)
- expressions: `PAREN_EXPR` → `EXPR_SEQ` → `ASSIGN_EXPR` (`( … =y,z )`)
- version terms: `VERSION_PAREN_EXPR` → `VERSION_OR_EXPR` →
  `VERSION_AND_EXPR` (`( … &&2||3&&4 )`)

Measured over flat chains, pure nesting, element-position wrappers,
composed nesting-plus-chaining, nested destructuring patterns, nested
modules, nested contracts through each of the four routes that reach
them, nested version terms, and nested generic types in every position
that accepts one — every level count up to `4 × max_depth`, at
`max_depth` 8, 32, 64, 128, 256 and 512 — the worst returned node depth
is `3 × max_depth + 4`, 772 at the default. The worst shape is a generic
type in parameter or struct-field position (`circuit f(a: A<A<…>>)`),
which pays the three type wrappers plus one uncharged node above the
first charge. Peak depth is *exactly* affine in `max_depth` at every one
of those settings, so the multiplier is a genuine constant and not
something the input can drive. That constant is a measurement over the
families above, not a proof over every possible input; the enumeration
has been corrected twice already, so treat it as the best current figure
rather than a ceiling the grammar can never move.

Bounding the *tree* — not just the parser — is what matters for
consumers: rowan drops a tree recursively, and most CST walks recurse
in the tree's depth, so an unbounded tree could abort a consumer's
process with `SIGABRT` when the tree was merely dropped. For the
pattern, module, version and contract grammars the parser's *own*
recursion is the binding constraint rather than the drop: with the tree
leaked instead of dropped, the overflow threshold is unchanged.

`max_depth` is therefore a stack budget for both the parser and the
consumer. Measured against the worst-case accepted shapes above on
`aarch64-apple-darwin`: at the default, parsing and dropping one needs
under 1 MiB of stack in debug and under 256 KiB in release, inside a
2 MiB thread. Raising the limit scales both linearly: on a 2 MiB stack
the deepest setting the parser itself survives is around 640 in debug
and around 3,100 in release. Frame sizes vary by target and optimization
level, so treat those as order-of-magnitude figures; raise the limit only
alongside a thread stack sized to match.

What this guarantee covers is depth, and the stack that depth costs. It
is not a statement about parse time or peak memory, which are bounded by
other properties of the parser and are not claimed here.

## Bounded error budget

`ParseOptions::max_errors` (default `256`) is a ceiling on reported
diagnostics, not a target. Every list-parsing recovery loop consumes at
least one token per iteration, so the number of diagnostics a given
input can produce is a function of that input and is reached well before
the budget for anything but pathological source.

That property is enforced rather than incidental. The loops are bounded
by the error budget, which does not by itself bound the *loop*: a
production that reported an error and consumed nothing would leave the
loop to run again on the same token and emit diagnostics until it hit
`max_errors`. The count would then track the option rather than the
input — `export`, six bytes, reached `max_errors` diagnostics on its own,
and at `max_errors = 5_000_000` a 115-byte input produced 5,000,002
diagnostics and over 600 MiB of message strings. Raising `max_errors` is
exactly what a language server or batch linter does, so this made a
tuning knob into a memory and CPU amplifier. Loops now skip a token
inside an `ERROR` node when a production stalls; `tests/error_budget.rs`
asserts that the diagnostic count does not change when `max_errors` is
raised from 100,000 to 1,000,000.

## Response Expectations

- Initial triage: within 3 business days
- Status update after reproduction: within 7 business days
- Remediation timeline: depends on severity and release coordination needs

If the report turns out to be an upstream Compact language/compiler issue instead of a `compactp` parser issue, we may coordinate disclosure or redirect you to the upstream project.
