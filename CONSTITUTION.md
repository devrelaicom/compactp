# compactp Constitution

## Core Principles

### I. Unix Philosophy

compactp is a command-line tool and library. It follows the Unix tradition: do one thing well, communicate via text, compose with other tools.

- Single purpose: parse Compact source. No compilation, no code generation, no semantic analysis in v1.
- Text I/O protocol: source in via files/stdin, structured output to stdout, diagnostics to stderr.
- Support both human-readable and machine-readable (JSON/JSONL) output for every command.
- Predictable exit codes: 0 success, 1 syntax errors, 2 IO errors, 3 invalid usage, 4 internal failure. These are stable contracts.
- Stdin support with `--stdin-filename` for editor/pipeline integration.

**Rationale:** Ecosystem tooling (editors, CI, other compilers) will consume compactp's output. Predictable, composable behavior is more important than feature richness.

### II. Modularity & Clear Boundaries

Each crate has one responsibility. Dependencies flow downward. No circular dependencies. Internal APIs are private; public APIs are stable.

- `compactp_syntax` depends on nothing internal. Every other crate depends on it.
- `compactp_ast` depends on `compactp_syntax`, NOT `compactp_parser`. The AST layer works over rowan nodes regardless of how they were produced.
- The parser is decoupled from tree construction via the event pipeline. You can test parsing without building a tree.
- No crate should reach into another's internals. Use the public API.

**Rationale:** This is a library stack that external projects will consume. Crate boundaries are API boundaries. Getting these wrong means breaking downstream users.

### III. Correctness Over Cleverness

The parser must produce correct, deterministic output for all valid Compact source. It must never panic on malformed input. Clever optimizations are welcome only after correctness is proven.

- No panics on user input. Ever. The parser must handle any byte sequence without crashing.
- Deterministic output: same input always produces the same CST, AST, and diagnostics.
- Lossless CST: every byte of the original source must be recoverable from the tree. Whitespace, comments, punctuation — nothing is discarded.
- Follow the upstream compiler as the source of truth, not the simplified tree-sitter grammar or public docs. When sources disagree, document the divergence.

**Rationale:** This tool will be used in CI pipelines, editors, and automated toolchains. A panic or non-deterministic output breaks trust. The lossless property enables future formatting, refactoring, and editor tooling.

### IV. Stable Public Contracts

JSON output schemas, exit codes, and Rust API signatures are public contracts from v1. Breaking changes require version bumps and migration paths.

- Every JSON payload includes envelope metadata: `tool_version`, `schema_version`, `language_version`.
- Schema version must be bumped on any breaking change to JSON output structure.
- Rust public API uses option structs (not boolean arguments), explicit error types, and `Option` for nullable positions.
- CLI flags and exit codes are stable once shipped. Deprecate before removing.
- Semantic versioning: breaking API/output changes require a major version bump.

**Rationale:** Downstream consumers (editors, CI tools, build systems) depend on stable output. Breaking changes without warning erode ecosystem trust.

### V. Test What Matters

Test corpus correctness, output contracts, and recovery behavior. Don't chase coverage percentages. Every test should catch a real class of bug.

- **Corpus tests:** Parse all 489 upstream .compact files without panics. No expected-pass file should produce errors.
- **Snapshot tests:** CST shape, AST accessors, diagnostic rendering, JSON output. These are regression guards.
- **Recovery tests:** Intentionally broken source must produce ERROR nodes, meaningful diagnostics, and an intact surrounding tree.
- **Contract tests:** JSON output snapshots are the public contract. Changing a snapshot is a conscious decision.
- **Benchmarks:** Track parse latency and throughput. Regressions are bugs.
- Don't test trivial getters or rowan internals. Test YOUR logic.

**Rationale:** The parser's value is in correctly handling the full Compact language surface, including edge cases. Tests prove this. Coverage metrics don't.

### VI. YAGNI & KISS

Build what the spec requires. Don't add features, abstractions, or configurability that isn't needed today. Three similar lines of code are better than a premature abstraction.

- No semantic analysis, type checking, name resolution, or code generation in v1.
- No LSP server, formatter, or linter in v1. The architecture should permit them later, but don't build hooks for them now.
- Don't add helper utilities for hypothetical future use. Add them when a real consumer needs them.
- Don't over-abstract the grammar rules. Each grammar function should be readable on its own without chasing indirection.
- If a design choice is "for future flexibility" but adds complexity now, don't make it.

**Rationale:** Scope creep kills parser projects. The spec is already ambitious. Ship a correct, fast, well-tested parser. The rest comes later.

### VII. Error Recovery Is a Feature

The parser must continue after errors. A file with one typo should produce a useful tree for the other 99% of the source. Recovery quality is a product differentiator.

- Missing semicolons, commas, and closing delimiters: emit a diagnostic, keep parsing.
- Malformed expressions, parameter lists, import clauses: wrap in ERROR nodes, synchronize on the next declaration or closing delimiter.
- ERROR nodes are never invisible. They are first-class CST nodes that tooling can inspect.
- Error budgeting: stop recovery after a configurable limit (default 256) to prevent cascading noise.
- Diagnostics must be actionable: say what was expected, point to where the problem is.

**Rationale:** Editors and CI tools parse incomplete, in-progress source files constantly. A parser that gives up on the first error is useless for tooling.

### VIII. Clear Naming & Code as Documentation

Names are the primary documentation. Code should be readable without comments. Comments explain WHY, never WHAT.

- Public types and functions must have doc comments explaining purpose and usage.
- Grammar functions should be named after the grammar production they implement: `circuit_def`, `expr_bp`, `type_ref`.
- SyntaxKind variants should be self-explanatory: `CIRCUIT_DEF`, `BINARY_EXPR`, `IMPORT_SPECIFIER`.
- Don't comment `// increment counter` above `counter += 1`. Do comment WHY a non-obvious recovery heuristic was chosen.
- Use the language of the spec: "circuit" not "function", "ledger" not "state variable", "witness" not "oracle".

**Rationale:** Open-source project with potential external contributors. Code must be approachable. Domain-specific naming reduces the gap between the Compact spec and the implementation.

## Development Workflow

### Commit Conventions

Use conventional commits for clear, searchable history:

```
feat(lexer): add hex/octal/binary literal support
fix(parser): handle missing semicolon in ledger declaration
test(corpus): add composable contract examples
refactor(ast): simplify Type sum-type matching
docs(cli): document JSON output envelope format
```

Scopes: `syntax`, `lexer`, `parser`, `ast`, `diagnostics`, `cli`, `corpus`, `bench`.

### Build Order Discipline

Crates are built bottom-up. Each crate must be complete and tested before its consumers are started:

1. compactp_syntax
2. compactp_lexer
3. compactp_parser
4. compactp_ast + compactp_diagnostics (parallel)
5. compactp (CLI binary)

### Dependency Hygiene

- Verify dependency versions via `cargo search` before adding. Never trust versions from memory or training data.
- Prefer stable releases over release candidates. Document the exception if an RC is used.
- Keep the dependency tree minimal. Every dependency is a maintenance burden for an open-source project.

## Governance

- This constitution supersedes ad-hoc decisions about architecture, testing, and API design.
- The design spec (`docs/superpowers/specs/2026-03-20-compactp-design.md`) is the technical reference for what to build. This constitution governs HOW to build it.
- Amendments require updating this document with a version bump and rationale.
- When upstream Compact syntax changes, update the parser to match, document the divergence, and bump the language version in output metadata.

**Version:** 1.0.0 | **Ratified:** 2026-03-20 | **Last Amended:** 2026-03-20
