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
  roughly 10 KB of *valid*
  Compact produced a 5000-deep tree with zero diagnostics. Dropping that tree
  overflowed the stack and aborted the process with `SIGABRT` — uncatchable,
  `catch_unwind` does not help — on any thread with a 2 MiB stack, and the
  `compactp cst` and `compactp stats` commands aborted on their own main
  thread. ([#21](https://github.com/devrelaicom/compactp/issues/21))

### Changed

- Expression chains longer than `max_depth` links now emit an
  `expression nesting depth limit exceeded` diagnostic and an `ERROR` node
  rather than nesting without limit. Input with more than roughly 250 chained
  operators or postfix operations therefore reports diagnostics where it
  previously reported none; the CST remains lossless in both cases. For scale,
  the deepest file in the 486-file upstream Compact corpus has a CST depth of
  20. Raise `ParseOptions::max_depth` to accept deeper input.

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
