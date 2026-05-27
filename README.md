# compactp

![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)

`compactp` is a production-grade parser frontend for the Compact language
(Midnight Network) written in Rust. It produces a lossless concrete syntax
tree, a typed abstract syntax tree, structured diagnostics, and machine-
readable JSON for every command, with watch-mode support for iterative
workflows.

## Features

- Lossless concrete syntax trees on `rowan` — every byte of the original
  source is preserved and recoverable from the tree
- Typed AST wrappers over the CST, zero-allocation views
- Resilient parsing with recovery and explicit `ERROR` nodes
- Rustc-style human diagnostics with optional ANSI color
- Structured JSON output with a versioned envelope (`tool_version`,
  `schema_version`, `language_version`) for every command
- Library APIs for embedding Compact parsing in Rust tooling
- Watch mode re-runs any command on `.compact` file changes

## Compact compatibility

| compactp version | Compact language | Tested compiler | JSON schema | MSRV |
| ---------------- | ---------------- | --------------- | ----------- | ---- |
| `0.1.0-pre.3`    | `0.23.101`       | `0.31.0`        | `1`         | not pinned (Rust edition `2024`) |

The exact upstream commit hashes the parser is validated against are
recorded in `SOURCE_VERSIONS.md`. Known deviations from upstream
acceptance are enumerated in `tests/corpus_known_failures.txt` (each
entry annotated with category + reason) and explained in `LS.md`.

> **Note:** This table reflects the *current development target*.
> Until `0.1.0-beta.1` is tagged, the compatibility contract is
> provisional and may shift between commits.

## Installation

From this workspace:

```bash
cargo install --path crates/compactp
```

From source:

```bash
git clone https://github.com/devrelaicom/compactp.git
cd compactp
cargo build --workspace --release
./target/release/compactp --help
```

## Quick start

```bash
# parse a file and report diagnostics
compactp parse path/to/program.compact

# emit structured diagnostics as JSON
compactp --format json --pretty diag path/to/program.compact

# dump the typed AST of every declaration
compactp ast path/to/program.compact

# watch a directory and re-parse on change
compactp watch parse src/
```

Read from stdin:

```bash
cat program.compact | compactp --stdin-filename program.compact parse
```

## CLI reference

| Command | Description                                                                           |
| ------- | ------------------------------------------------------------------------------------- |
| `lex`   | Tokenize Compact source and print tokens with byte offsets                            |
| `parse` | Parse input and report diagnostics (structured in JSON, rustc-style in human mode)    |
| `cst`   | Dump the full lossless concrete syntax tree                                           |
| `ast`   | Dump the typed abstract syntax tree — heterogeneous items in source order             |
| `diag`  | Emit diagnostics only (silent on success; exits 1 if any diagnostic fires)            |
| `stats` | Report file size, token/node/error/recovery counts, and parse time                    |
| `watch` | Re-run any of the above when `.compact` files change under the watched paths          |

## Global flags

- `--format <human|json>` — output format (default: `human`)
- `--pretty` — pretty-print JSON output
- `--color <auto|always|never>` — ANSI color policy for human diagnostics (default: `auto`)
- `--timing` — include timing data in supported outputs
- `--stdin-filename <NAME>` — label for stdin input in diagnostics and JSON envelopes
- `--max-diagnostics <N>` — cap the number of emitted diagnostics per input
- `--max-errors <N>` — limit parser error collection before the parser stops reporting more (default: `256`)
- `--no-recover` — disable recovery-oriented parsing

## Exit codes

| Code | Meaning                                    |
| ---- | ------------------------------------------ |
| 0    | Success (no parse errors)                  |
| 1    | Parse errors reported or runtime failure   |
| 2    | I/O error (unreadable file/dir, stdin)     |
| 3    | Usage error (invalid flags, bad arguments) |
| 4    | Internal failure (e.g., watch debouncer)   |

## JSON output schema

Every JSON payload is wrapped in an envelope:

```json
{
  "tool_version":     "0.1.0",
  "schema_version":   1,
  "language_version": "0.22.0",
  "input":            "path/to/file.compact",
  "timing_ms":        0.42,
  "data":             { ... }
}
```

`timing_ms` is present only when `--timing` is passed. `data` is subcommand-
specific:

| Subcommand | `data` shape                                                                 |
| ---------- | ---------------------------------------------------------------------------- |
| `lex`      | `[{ kind, text, offset, len }]` — array of token records                     |
| `parse`    | `{ success, error_count, truncated?, diagnostics: [Diagnostic] }`            |
| `cst`      | `{ kind, text?, children: [CstNode] }` — recursive lossless tree             |
| `ast`      | `{ kind: "SourceFile", items: [Item] }` — items tagged by `kind` (14 variants) |
| `diag`     | `{ error_count, truncated?, diagnostics: [Diagnostic] }`                     |
| `stats`    | `{ file_size_bytes, token_count, node_count, error_count, recovery_count, parse_time_ms }` |

`error_count` is the count *before* `--max-diagnostics` applies — the CLI
will not erase the signal that something went wrong. `truncated` is present
(and `true`) only when the cap fired; omitted otherwise.

Diagnostic shape (identical for `parse` and `diag`):

```json
{
  "severity":        "error" | "warning" | "note",
  "code":            { "prefix": "E", "number": 1 },
  "message":         "expected SEMICOLON",
  "primary_span":    {
    "start": { "offset": 19, "line": 1, "column": 20 },
    "end":   { "offset": 20, "line": 1, "column": 21 }
  },
  "secondary_spans": [{
    "start": { "offset": 0, "line": 1, "column": 1 },
    "end":   { "offset": 4, "line": 1, "column": 5 },
    "label": null
  }],
  "notes":           ["did you mean `;`?"]
}
```

Line and column numbers are 1-based. Byte offsets index the original
source.

`ast` items are tagged unions — every element has a `kind` field and variant-
specific fields. The supported variants are:

| `kind`            | Extra fields                                           |
| ----------------- | ------------------------------------------------------ |
| `Pragma`          | —                                                      |
| `Include`         | —                                                      |
| `Import`          | —                                                      |
| `ExportList`      | —                                                      |
| `LedgerDecl`      | `name, exported, sealed`                               |
| `ConstructorDef`  | —                                                      |
| `CircuitDef`      | `name, exported, pure, has_body`                       |
| `CircuitDecl`     | `name, exported`                                       |
| `WitnessDecl`     | `name, exported`                                       |
| `ContractDecl`    | `name, exported, circuits: [name]`                     |
| `StructDef`       | `name, exported, fields: [name]`                       |
| `EnumDef`         | `name, exported, variants: [name]`                     |
| `ModuleDef`       | `name, exported`                                       |
| `TypeDecl`        | `name, exported, new, has_generic_params`              |

## Architecture

Six crates in bottom-up dependency order:

- `compactp_syntax` — `SyntaxKind` enum and `rowan` node/token wrappers
- `compactp_lexer` — hand-rolled lexer, lossless over the full Compact surface
- `compactp_parser` — event-based parser with recovery, CST construction
- `compactp_ast` — zero-allocation typed AST wrappers over CST nodes
- `compactp_diagnostics` — diagnostic data model, human and JSON renderers
- `compactp` — the CLI binary, integration tests, snapshots, fixtures

Data flow:

```
source text -> lexer -> parser events -> CST -> typed AST -> diagnostics/renderers
```

## Library usage

```rust
use compactp_parser::parse;
use compactp_syntax::SyntaxNode;

let result = parse("ledger count: Field;");
let root = SyntaxNode::new_root(result.green);

assert_eq!(root.kind(), compactp_syntax::SyntaxKind::SOURCE_FILE);
assert!(result.errors.is_empty());

// the lossless invariant — every byte is in the tree
assert_eq!(root.text().to_string(), "ledger count: Field;");
```

For custom recovery limits use `compactp_parser::parse_with` with
`ParseOptions`:

```rust
use compactp_parser::{parse_with, ParseOptions};

let opts = ParseOptions { recover: true, max_errors: 32 };
let result = parse_with(source, opts);
```

Walking the typed AST:

```rust
use compactp_ast::{AstNode, Item, SourceFile};
use compactp_syntax::SyntaxNode;

let result = compactp_parser::parse(source);
let root = SyntaxNode::new_root(result.green);
let file = SourceFile::cast(root).expect("root is SOURCE_FILE");

for item in file.items() {
    match item {
        Item::CircuitDef(c) => { /* … */ }
        Item::StructDef(s)  => { /* … */ }
        _ => {}
    }
}
```

Rendering diagnostics:

```rust
use compactp_diagnostics::{render_human, render_json};

for diag in &result.errors {
    print!("{}", render_human(diag, source, "input.compact", /* colored */ false));
}

for diag in &result.errors {
    let value = render_json(diag, source);
    println!("{}", serde_json::to_string_pretty(&value)?);
}
```

## Development

```bash
# build the workspace
cargo build --workspace

# full test suite (corpus + unit + CLI integration)
cargo test --workspace

# just the CLI integration tests and snapshots
cargo test -p compactp --test cli

# regenerate snapshots after an intentional change
cargo insta test --accept -p compactp --test cli

# parser corpus (486 upstream source files under tests/corpus/, lossless
# invariant enforced, known-failure manifest at
# tests/corpus_known_failures.txt)
cargo test -p compactp_parser --test corpus_test

# formatting + lints (CI enforces both)
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

## License

MIT — see [LICENSE](LICENSE) (or the workspace `Cargo.toml` `license` field).

## Acknowledgments

- Compact language and ecosystem work by Midnight Foundation.
- Parser architecture inspiration from rust-analyzer's `rowan` + event-based
  construction.
