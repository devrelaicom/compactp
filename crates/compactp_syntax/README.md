# compactp_syntax

Shared syntax-tree types for the [`compactp`](https://github.com/devrelaicom/compactp)
parser frontend for the Compact language (Midnight Network).

This crate defines `SyntaxKind` — the enum of every node and token kind in the
Compact concrete syntax tree — plus the [`rowan`](https://crates.io/crates/rowan)
`SyntaxNode` / `SyntaxToken` type aliases that the lexer, parser, and AST crates
build on. It has no dependencies beyond `rowan` and is the root of the
`compactp` crate graph.

## Status

Beta (`0.x`). APIs may change between minor versions. See the
[compatibility matrix](https://github.com/devrelaicom/compactp#compact-compatibility).

## License

MIT
