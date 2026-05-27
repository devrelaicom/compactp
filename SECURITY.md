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

## Bounded-recursion guarantee

The parser's recursive grammar functions (`expr_bp`, `ty`, `stmt`,
`block`) check `ParseOptions::max_depth` (default `256`) at each
entry. On overflow, the parser emits a recovery diagnostic and
produces an `ERROR` node rather than overflowing the stack. Adjust
`max_depth` via `ParseOptions` for inputs intentionally deeper than
the default.

## Response Expectations

- Initial triage: within 3 business days
- Status update after reproduction: within 7 business days
- Remediation timeline: depends on severity and release coordination needs

If the report turns out to be an upstream Compact language/compiler issue instead of a `compactp` parser issue, we may coordinate disclosure or redirect you to the upstream project.
