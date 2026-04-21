# Contributing to compactp

Thanks for contributing. `compactp` is parser infrastructure for the Compact language (Midnight Network), so small regressions can cascade into editor tooling, CI, and downstream language integrations. Favor precise changes, explicit tests, and conservative parser behavior.

## Development setup

1. Install stable Rust with `rustup`.
2. Clone the repository and enter the workspace.
3. Build everything once so cargo downloads all dependencies.

```bash
git clone git@github.com:devrelaicom/compactp.git
cd compactp
cargo build --workspace
```

## Running tests

Run the full workspace suite before opening a PR:

```bash
cargo test --workspace
```

Useful focused commands:

```bash
# CLI integration tests and snapshots
cargo test -p compactp --test cli

# Regenerate CLI snapshots after an intentional change
cargo insta test --accept -p compactp --test cli

# Parser corpus (489 upstream files, lossless invariant enforced)
cargo test -p compactp_parser --test corpus_test

# Parse benchmark
cargo bench -p compactp_parser --bench parse_bench
```

## Code style and local hooks

- Formatting is enforced with `cargo fmt --all -- --check`.
- Linting is enforced with `cargo clippy --workspace --all-targets -- -D warnings` in CI.
- The `compactp` crate bans panic paths (`unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`) via `[lints.clippy]` in its `Cargo.toml`. Do not weaken these; use `CliError` instead.
- `lefthook` (configured in `lefthook.yml`) runs formatting and clippy checks on commit and build/test checks on push.
- Prefer small, reviewable commits over large mixed-purpose changes.

## Commit conventions

Commits must follow Conventional Commits. The commit-msg hook rejects anything outside this shape:

```text
type(scope): short description
```

Accepted types are `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, and `revert`.

The hook also enforces:

- commit body under 500 characters
- commit body under 10 lines

## Pull request workflow

1. Fork the repository or create a feature branch from `main`.
2. Implement the change with tests.
3. Run fmt, clippy, and tests locally.
4. Open a PR against `main` and fill out every checkbox in the PR template.
5. Call out any parser behavior change — especially recovery changes or grammar expansions — in the PR summary.

## Adding a new grammar construct

When adding new syntax, update the stack from the bottom up:

1. Add or update `SyntaxKind` entries in `crates/compactp_syntax/src/syntax_kind.rs`.
2. Extend the grammar in `crates/compactp_parser/src/grammar/`.
3. Add parser coverage in `crates/compactp_parser` unit tests.
4. Add typed wrappers or accessors in `crates/compactp_ast/` if the syntax should be exposed at the AST layer.
5. Update CLI rendering or snapshots if the user-facing output changes.
6. Add focused fixtures for valid syntax, invalid syntax, and recovery behavior.

Never weaken the lossless CST invariant (`root.text() == source` byte-for-byte). The corpus test enforces this across every upstream fixture.

## Fixtures, corpus data, and snapshots

- Small hand-written fixtures live under `crates/compactp/tests/fixtures/` for targeted CLI behaviors (see `tests/fixtures/README.md`).
- The parser corpus test walks `tests/corpus/`.
- Upstream corpus files copied from Compact references must retain their provenance and should not be relicensed or silently rewritten.
- CLI snapshots live under `crates/compactp/tests/snapshots/`.
- If JSON output changes, update snapshots intentionally and explain the schema impact in the PR. If the change is a breaking edit to the JSON envelope or `data` shape, bump `SCHEMA_VERSION` in `crates/compactp/src/output.rs`.

## Exit codes

`compactp` uses a fixed exit-code table (0 success, 1 runtime, 2 I/O, 3 usage, 4 internal). New failure modes should map to an existing code via `CliError::runtime` / `io` / `usage` / `internal`. Do not invent new codes without a README update.

## Release process

Releases are managed with `release-plz` and GitHub Actions:

- version bumps and changelog updates are driven by `release-plz.toml`
- release artifacts are produced by the release workflow and `cargo-dist`
- distribution targets are configured through `dist-workspace.toml`

If your change affects public APIs, CLI output, or packaging, call that out explicitly in the PR summary so release notes can flag it.
