# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
While in `0.x`, breaking changes may land in any minor release.

## [Unreleased]

### Fixed

- `ParseOptions::max_depth` now bounds the depth of the expression tree the
  parser returns, not only the depth of recursive-descent entry.
  Left-associative operator chains (`x+x+...+x`) and postfix chains
  (`f()()...`, `x[0][0]...`, `x.a.a...`, `x as T as T...`) deepen the tree from
  inside a single stack frame and were never charged against the limit, so
  roughly 10 KB of *valid* Compact produced a 5000-deep tree with zero
  diagnostics. Dropping that tree overflowed the stack and aborted the process
  with `SIGABRT` — uncatchable, `catch_unwind` does not help — on any thread
  with a 2 MiB stack, and the `compactp cst` and `compactp stats` commands
  aborted on their own main thread.
  ([#21](https://github.com/devrelaicom/compactp/issues/21))
- The spine charge is height-aware: it counts the height of the subtree being
  wrapped rather than one step along the current path. A path-only charge is
  insufficient, because the left-hand side is parsed before the loop holds any
  charge and releases its own charges on the way out — so nesting and chaining
  together (`((( x +x+x… ) +x+x… ) +x+x… )`) each re-spend the same budget and
  compound into a Θ(`max_depth`²) tree. At the default that reached CST depth
  23,297 from 131 KB of valid Compact, enough to abort a 2 MiB thread again.
  Worst measured node depth is now `3 × max_depth` (767 at the default), the
  multiplier being the deepest stack of uncharged wrapper nodes the grammar can
  place between two charges (`PAREN_EXPR` → `EXPR_SEQ` → `ASSIGN_EXPR`). That
  ratio is identical at `max_depth` 32, 64 and 256 — a genuine constant, not an
  input-controlled multiplier.
- `ParseOptions::max_depth` now also bounds the pattern, module and version
  grammars. Three recursive productions never charged the counter at all —
  `pattern → tuple_pattern → tuple_pat_elt → pattern`,
  `module_def → declaration → module_def` and
  `version_term → version_or_expr → version_and_expr → version_term` — so no
  setting of `max_depth` constrained them: `max_depth = 1` still parsed
  2001-deep module nesting with zero diagnostics. Each could be driven to a
  stack overflow that aborts the process uncatchably — around 37,500 nested
  brackets, 57,000 nested modules or 21,500 nested version parentheses on an
  8 MiB stack — and the version case reaches through the CLI on a single line:
  `pragma language_version ((((>= 0.23))));`. Unlike
  [#21](https://github.com/devrelaicom/compactp/issues/21), the binding
  constraint here is the parser's own recursion rather than the tree's `Drop`:
  leaking the tree instead of dropping it leaves the threshold unchanged, so a
  plain depth charge on entry is the fix. Worst measured node depth across all
  grammars is `3 × max_depth + 4` (772 at the default), from a generic type in
  parameter, struct-field or contract-member position, and is exactly affine in
  `max_depth` at 8, 32, 64, 128, 256 and 512. Charging `module_def` also
  improves that worst case from `main`'s 773
  (`module M{ struct S { a: A<A<…>>; } }`), whose previously documented figure
  of 767 was understated because the wrapper-stack enumeration omitted
  `TYPE_REF` → `GENERIC_ARG_LIST` → `GENERIC_ARG`.
  ([#23](https://github.com/devrelaicom/compactp/issues/23))
- `ParseOptions::max_errors` no longer amplifies. Every list-parsing recovery
  loop was bounded by the error budget but not by token progress, so a
  production that reported an error and consumed nothing left the loop to
  re-run on the same token until it reached `max_errors`. The diagnostic count
  tracked the option rather than the input: `export` — six bytes — reached
  `max_errors` diagnostics on its own, and at `max_errors = 5_000_000` a
  115-byte input produced 5,000,002 diagnostics and over 600 MiB of message
  strings. Raising `max_errors` is exactly what a language server or a batch
  linter does, so this made a tuning knob into a memory and CPU amplifier.
  Recovery loops now skip a token inside an `ERROR` node when a production
  stalls. Five of the reachable inputs predate this release (`export`, a stray
  `]` in statement position, `export ;` inside a `contract` or `module` body);
  two more became reachable when `pattern()` started enforcing `max_depth`.

### Known issues

- Nested `contract` declarations are still not depth-charged.
  `declarations::contract` recurses through `declarations::declaration` back
  into itself, so nested `contract` declarations are unbounded at every
  `max_depth` and overflow the parser's stack at around 37,500 levels.
  Reachable from a top-level `contract A { contract B { … } }`, from
  `export contract`, from a circuit or constructor body, and from a `module`
  body — the enclosing `block`/`stmt`/`module_def` charge is spent once on
  entry and the uncharged cycle then runs free, so rejecting only the
  top-level form does not mitigate it. Same class as
  [#23](https://github.com/devrelaicom/compactp/issues/23); tracked as
  [#28](https://github.com/devrelaicom/compactp/issues/28).

### Changed

- Deeply nested destructuring patterns, module declarations and pragma version
  terms now emit a `pattern nesting depth limit exceeded`,
  `module nesting depth limit exceeded` or
  `version expression nesting depth limit exceeded` diagnostic and an `ERROR`
  node once they pass `max_depth`, where they previously nested without limit
  and reported nothing. The CST remains lossless in both cases. For scale, the
  deepest file in the upstream Compact corpus has a CST depth of 20.
- Expression chains longer than `max_depth` links now emit an
  `expression nesting depth limit exceeded` diagnostic and an `ERROR` node
  rather than nesting without limit. Input with more than roughly 250 chained
  operators or postfix operations therefore reports diagnostics where it
  previously reported none; the CST remains lossless in both cases. For scale,
  the deepest file in the 486-file upstream Compact corpus has a CST depth of
  20. Raise `ParseOptions::max_depth` to accept deeper input, sizing the thread
  stack to match — the knob is a stack budget for the parser as well as for
  consumers of the tree.

## [0.1.0-beta.1]

First public beta of `compactp` and its five companion library crates
(`compactp_syntax`, `compactp_lexer`, `compactp_parser`, `compactp_ast`,
`compactp_diagnostics`).

### Added

- Lossless concrete syntax tree over `rowan` — every byte of source is recoverable.
- Typed, zero-allocation AST wrappers over the CST.
- Resilient recursive-descent parser with error recovery and explicit `ERROR`
  nodes; Pratt-style expression precedence.
- Bounded parse recursion depth (`ParseOptions::max_depth`, default 256) — no
  stack overflow on adversarial input.
- Rustc-style human diagnostics with optional ANSI color, and a structured,
  versioned JSON envelope (`tool_version`, `schema_version`, `language_version`)
  for every command.
- CLI commands: `lex`, `parse`, `cst`, `ast`, `diag`, `stats`, `watch`.
- Tracks the current Compact language surface (`pragma language_version >= 0.23`),
  validated against compiler `0.31.0`.
- Documented public API for all five library crates (`#![deny(missing_docs)]`),
  with committed `cargo public-api` baselines.
- `cargo-fuzz` harnesses for the lexer and parser, with a nightly CI fuzz job and
  a `scripts/fuzz.sh` long-run wrapper.
- Compatibility matrix and JSON `schema_version` policy.

### Known limitations

- No semantic checking, name resolution, constant evaluation, code generation, or
  runtime execution — `compactp` is a syntactic frontend only.
- Intentional strictness and upstream-bug-reproduction deviations from `compactc`
  acceptance are enumerated in `tests/corpus_known_failures.txt`.

[Unreleased]: https://github.com/devrelaicom/compactp/compare/compactp-v0.1.0-beta.1...HEAD
[0.1.0-beta.1]: https://github.com/devrelaicom/compactp/releases/tag/compactp-v0.1.0-beta.1
